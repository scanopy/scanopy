use std::{fmt, time::Instant};

#[cfg(feature = "crypto-openssl")]
use openssl::{
    hash::{Hasher, MessageDigest},
    pkey::PKey,
    sign::Signer,
};

#[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
use {
    aes::{Aes128, Aes192, Aes256},
    cbc::{Decryptor as CbcDecryptor, Encryptor as CbcEncryptor},
    cfb_mode::{Decryptor as CfbDecryptor, Encryptor as CfbEncryptor},
    cipher::{AsyncStreamCipher, BlockDecryptMut, BlockEncryptMut, KeyIvInit},
    des::Des,
    digest::{Digest, DynDigest},
    hmac::{Hmac, Mac},
};

use crate::{
    AsnReader, BUFFER_SIZE, Error, MessageType, Oid, Pdu, Result, Value, Varbinds, Version, asn1,
    pdu::{self, Buf},
    snmp::{self, V3_MSG_FLAGS_AUTH, V3_MSG_FLAGS_PRIVACY, V3_MSG_FLAGS_REPORTABLE},
};

const ENGINE_TIME_WINDOW: i64 = 150;

#[cfg(feature = "v3")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AuthErrorKind {
    UnsupportedUSM,
    EngineBootsMismatch,
    EngineTimeMismatch,
    NotAuthenticated,
    UsernameMismatch,
    EngineIdMismatch,
    SignatureMismatch,
    MessageIdMismatch,
    PrivLengthMismatch,
    KeyLengthMismatch,
    PayloadLengthMismatch,
    ReplyNotEncrypted,
    SecurityNotProvided,
    SecurityNotReady,
    KeyExtensionRequired,
    /// SCANOPY LOCAL PATCH (GH #846): the agent answered an encrypted request with a
    /// `usmStatsDecryptionErrors` Report, so the privacy key is not the one it holds.
    DecryptionError,
}

/// `usmStatsDecryptionErrors.0` (RFC 3414 §5).
const USM_STATS_DECRYPTION_ERRORS: &[u64] = &[1, 3, 6, 1, 6, 3, 15, 1, 1, 6, 0];

/// SCANOPY LOCAL PATCH (GH #846): whether the plaintext scoped PDU at `rdr` is a Report whose
/// first varbind is `usmStatsDecryptionErrors`. Anything that does not parse as one is not.
fn is_decryption_error_report(mut rdr: AsnReader<'_>) -> bool {
    let mut first_counter = || -> Result<Option<Oid<'_>>> {
        let mut scoped_pdu = AsnReader::from_bytes(rdr.read_raw(asn1::TYPE_SEQUENCE)?);
        scoped_pdu.read_asn_octetstring()?; // contextEngineID
        scoped_pdu.read_asn_octetstring()?; // contextName
        let ident = scoped_pdu.peek_byte()?;
        if MessageType::from_ident(ident)? != MessageType::Report {
            return Ok(None);
        }
        let mut pdu = AsnReader::from_bytes(scoped_pdu.read_raw(ident)?);
        pdu.read_asn_integer()?; // request-id
        pdu.read_asn_integer()?; // error-status
        pdu.read_asn_integer()?; // error-index
        let mut varbinds = Varbinds::from_bytes(pdu.read_raw(asn1::TYPE_SEQUENCE)?);
        Ok(varbinds.next().map(|(oid, _)| oid))
    };
    let Ok(expected) = Oid::from(USM_STATS_DECRYPTION_ERRORS) else {
        return false;
    };
    matches!(first_counter(), Ok(Some(oid)) if oid == expected)
}

impl fmt::Display for AuthErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthErrorKind::UnsupportedUSM => write!(f, "Unsupported USM"),
            AuthErrorKind::EngineBootsMismatch => write!(f, "Engine boots counter mismatch"),
            AuthErrorKind::EngineTimeMismatch => write!(f, "Engine time counter mismatch"),
            AuthErrorKind::NotAuthenticated => write!(f, "Not authenticated"),
            AuthErrorKind::EngineIdMismatch => write!(f, "Engine ID mismatch"),
            AuthErrorKind::UsernameMismatch => write!(f, "Username mismatch"),
            AuthErrorKind::SignatureMismatch => write!(f, "HMAC signature mismatch"),
            AuthErrorKind::MessageIdMismatch => write!(f, "Message ID mismatch"),
            AuthErrorKind::PrivLengthMismatch => write!(f, "Privacy parameters length mismatch"),
            AuthErrorKind::KeyLengthMismatch => write!(f, "Key length mismatch"),
            AuthErrorKind::PayloadLengthMismatch => write!(f, "Payload length mismatch"),
            AuthErrorKind::ReplyNotEncrypted => write!(f, "Not an encrypted reply"),
            AuthErrorKind::SecurityNotProvided => write!(f, "Security parameters not provided"),
            AuthErrorKind::SecurityNotReady => write!(f, "Security parameters not ready"),
            AuthErrorKind::KeyExtensionRequired => {
                write!(f, "Auth/Priv pair needs a key extension method")
            }
            AuthErrorKind::DecryptionError => {
                write!(f, "The agent could not decrypt the request")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AuthoritativeState {
    auth_key: Vec<u8>,
    priv_key: Vec<u8>,
    pub(crate) engine_id: Vec<u8>,
    engine_boots: i64,
    engine_time: i64,
    engine_time_current: i64,
    start_time: Instant,
    // SCANOPY LOCAL PATCH (GH #813): whether `engine_boots` and `engine_time` came from an
    // authenticated message (or the caller), as RFC 3414 §3.2 step 7b requires before they can
    // judge timeliness. Upstream read `engine_boots == 0` as "not discovered", but §2.2.1 allows
    // 0 and MikroTik RouterOS always reports it. See vendor/snmp2/README.md.
    time_synchronized: bool,
}

impl Default for AuthoritativeState {
    fn default() -> Self {
        Self {
            auth_key: Vec::new(),
            priv_key: Vec::new(),
            engine_id: Vec::new(),
            engine_boots: 0,
            engine_time: 0,
            engine_time_current: 0,
            start_time: Instant::now(),
            time_synchronized: false,
        }
    }
}

impl AuthoritativeState {
    fn update_authoritative(&mut self, engine_boots: i64, engine_time: i64) {
        self.engine_boots = engine_boots;
        self.engine_time = engine_time;
        self.start_time = Instant::now();
    }

    fn update_authoritative_engine_time(&mut self, engine_time: i64) {
        self.engine_time = engine_time;
        self.start_time = Instant::now();
    }

    /// SCANOPY LOCAL PATCH: RFC 3414's "local notion" of the authoritative engine's time: the
    /// latest time received, advanced by the local clock since.
    fn local_engine_time(&self) -> i64 {
        self.engine_time + i64::try_from(self.start_time.elapsed().as_secs()).unwrap_or(i64::MAX)
    }

    fn correct_engine_time(&mut self) {
        // SCANOPY LOCAL PATCH: was `self.engine_boots == 0`, which is a valid boots value.
        if !self.time_synchronized {
            self.engine_time_current = 0;
            return;
        }
        let max = i32::MAX.into();
        self.engine_time_current =
            i64::try_from(self.start_time.elapsed().as_secs()).unwrap() + self.engine_time;
        if self.engine_time_current >= max {
            self.engine_time_current -= max;
            self.engine_boots += 1;
        }
    }

    fn generate_key(&self, password: &[u8], auth_protocol: AuthProtocol) -> Result<Vec<u8>> {
        let mut hasher = auth_protocol.create_hasher()?;
        let mut password_index = 0;
        let mut password_buf = vec![0; 64];
        for _ in 0..16384 {
            for x in &mut password_buf {
                *x = password[password_index];
                password_index += 1;
                if password_index == password.len() {
                    password_index = 0;
                }
            }
            hasher.update(&password_buf)?;
        }
        let key = hasher.finish()?;
        password_buf.clear();
        password_buf.extend_from_slice(&key);
        password_buf.extend_from_slice(&self.engine_id);
        password_buf.extend_from_slice(&key);
        hasher.update(&password_buf)?;
        #[allow(clippy::implicit_clone)]
        Ok(hasher.finish()?.to_vec())
    }

    fn update_auth_key(
        &mut self,
        authentication_password: &[u8],
        auth_protocol: AuthProtocol,
    ) -> Result<()> {
        if self.engine_id.is_empty() {
            self.auth_key.clear();
            return Err(Error::AuthFailure(AuthErrorKind::NotAuthenticated));
        }
        self.auth_key = self.generate_key(authentication_password, auth_protocol)?;
        Ok(())
    }

    fn update_priv_key(
        &mut self,
        privacy_password: &[u8],
        auth_protocol: AuthProtocol,
        cipher: Cipher,
        extension_method: Option<&KeyExtension>,
    ) -> Result<()> {
        if self.engine_id.is_empty() {
            self.priv_key.clear();
            return Err(Error::AuthFailure(AuthErrorKind::NotAuthenticated));
        }
        self.priv_key = self.generate_key(privacy_password, auth_protocol)?;
        if !cipher.priv_key_needs_extension(&auth_protocol) {
            return Ok(());
        }
        match extension_method.as_ref() {
            Some(KeyExtension::Blumenthal) => {
                self.extend_priv_key_with_blumenthal_method(cipher.priv_key_len(), auth_protocol)?;
            }
            Some(KeyExtension::Reeder) => {
                self.extend_priv_key_with_reeder_method(cipher.priv_key_len(), auth_protocol)?;
            }
            None => return Err(Error::AuthFailure(AuthErrorKind::KeyExtensionRequired)),
        }
        Ok(())
    }

    /// Extend `priv_key` to the required length using the Blumenthal algorithm.
    /// This is used for AES-192/256 privacy keys when Kul is shorter than needed.
    fn extend_priv_key_with_blumenthal_method(
        &mut self,
        need_key_len: usize,
        auth_protocol: AuthProtocol,
    ) -> Result<()> {
        if need_key_len <= self.priv_key.len() {
            return Ok(());
        }

        let mut remaining = need_key_len - self.priv_key.len();

        while remaining > 0 {
            // Hash the current priv_key using the auth protocol’s hash function
            let mut hasher = auth_protocol.create_hasher()?;
            hasher.update(&self.priv_key)?;
            let new_hash = hasher.finish()?; // full digest

            // Append as much as needed
            let copy_len = remaining.min(new_hash.len());
            self.priv_key.extend_from_slice(&new_hash[..copy_len]);
            remaining -= copy_len;
        }

        Ok(())
    }
    /// Extend Kul to the required length using the Reeder method.
    /// `need_key_len` is the desired privacy key length (e.g. 24 for AES-192, 32 for AES-256).
    fn extend_priv_key_with_reeder_method(
        &mut self,
        need_key_len: usize,
        auth_protocol: AuthProtocol,
    ) -> Result<()> {
        if need_key_len < self.priv_key.len() {
            return Ok(());
        }
        let mut remaining = need_key_len - self.priv_key.len();
        while remaining > 0 {
            // Step 1: Ku' = Ku(origKul)
            // Here we treat the current Kul as the "password"
            let new_kul = self.generate_key(&self.priv_key, auth_protocol)?;

            // Step 2: append Kul' to the existing Kul
            let copy_len = remaining.min(new_kul.len());
            self.priv_key.extend_from_slice(&new_kul[..copy_len]);
            remaining -= copy_len;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyExtension {
    Blumenthal,
    Reeder,
}

impl KeyExtension {
    pub fn other(&self) -> Self {
        match self {
            KeyExtension::Blumenthal => KeyExtension::Reeder,
            KeyExtension::Reeder => KeyExtension::Blumenthal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Security {
    pub(crate) username: Vec<u8>,
    pub(crate) authentication_password: Vec<u8>,
    pub(crate) auth: Auth,
    pub(crate) auth_protocol: AuthProtocol,
    pub(crate) key_extension_method: Option<KeyExtension>,
    pub(crate) authoritative_state: AuthoritativeState,
    pub(crate) plain_buf: Vec<u8>,
    // SCANOPY LOCAL PATCH: the contextName sent in every outgoing scoped PDU. Upstream snmp2
    // 0.5.0 hard-codes it empty, so a caller could not address a non-default context at all —
    // Cisco IOS-XE keeps its per-VLAN bridge FDB in one, and it read as an empty table. Held on
    // `Security` rather than on `Pdu` because `Pdu::to_bytes_with_security` rebuilds the scoped
    // PDU from `Security` alone. Not part of key derivation, so it needs no `update_key`.
    // See vendor/snmp2/README.md.
    pub(crate) context_name: Vec<u8>,
}

impl Security {
    pub fn new(username: &[u8], authentication_password: &[u8]) -> Self {
        Self {
            username: username.to_vec(),
            authentication_password: authentication_password.to_vec(),
            auth: Auth::AuthNoPriv,
            auth_protocol: AuthProtocol::Md5,
            key_extension_method: None,
            authoritative_state: AuthoritativeState::default(),
            plain_buf: Vec::new(),
            context_name: Vec::new(),
        }
    }

    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_auth_protocol(mut self, auth_protocol: AuthProtocol) -> Self {
        self.auth_protocol = auth_protocol;
        self
    }

    pub fn with_key_extension_method(mut self, key_extension_method: KeyExtension) -> Self {
        self.key_extension_method = Some(key_extension_method);
        self
    }

    /// SCANOPY LOCAL PATCH: set the contextName sent in outgoing scoped PDUs. Empty — the
    /// default — addresses the default context, which is what every caller got before.
    /// Engine discovery is unaffected: `build_init` sends the null context regardless, as
    /// RFC 3414 §4 requires.
    pub fn with_context_name(mut self, context_name: &[u8]) -> Self {
        self.context_name = context_name.to_vec();
        self
    }

    pub(crate) fn another_key_extension_method(&mut self) -> Option<KeyExtension> {
        if let Auth::AuthPriv { ref cipher, .. } = self.auth
            && cipher.priv_key_needs_extension(&self.auth_protocol)
            && let Some(used_method) = self.key_extension_method
        {
            self.key_extension_method = Some(used_method.other());
            return self.key_extension_method;
        }
        None
    }

    /// Note: the engine_id MUST be provided as a hex array, not as a byte-string.
    /// E.g. if a target has got an engine id `80003a8c04` set, it should be provided as `&[0x80,
    /// 0x00, 0x3a, 0x8c, 0x04]`
    pub fn with_engine_id(mut self, engine_id: &[u8]) -> Result<Self> {
        self.authoritative_state.engine_id = engine_id.to_vec();
        self.update_key()?;
        Ok(self)
    }

    pub fn with_engine_boots_and_time(mut self, engine_boots: i64, engine_time: i64) -> Self {
        self.authoritative_state.engine_boots = engine_boots;
        self.authoritative_state
            .update_authoritative_engine_time(engine_time);
        // SCANOPY LOCAL PATCH: values the caller supplies are known, not a discovery hint.
        self.authoritative_state.time_synchronized = true;
        self
    }

    pub fn reset_engine_id(&mut self) {
        self.authoritative_state.engine_id.clear();
        self.authoritative_state.auth_key.clear();
        self.authoritative_state.priv_key.clear();
    }

    pub fn reset_engine_counters(&mut self) {
        self.authoritative_state.engine_boots = 0;
        self.authoritative_state.update_authoritative_engine_time(0);
        // SCANOPY LOCAL PATCH: "not discovered" is this flag, not boots = 0.
        self.authoritative_state.time_synchronized = false;
    }

    /// SCANOPY LOCAL PATCH: whether boots and time came from an authenticated message, so
    /// `init` knows whether RFC 3414 §4's second discovery step is still owed.
    pub(crate) fn is_time_synchronized(&self) -> bool {
        self.authoritative_state.time_synchronized
    }

    fn calculate_hmac(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.engine_id().is_empty() {
            return Err(Error::AuthFailure(AuthErrorKind::SecurityNotReady));
        }
        #[cfg(feature = "crypto-openssl")]
        {
            let pkey = PKey::hmac(&self.authoritative_state.auth_key)?;
            let mut signer = Signer::new(self.auth_protocol.digest(), &pkey)?;
            signer.update(data)?;
            signer.sign_to_vec().map_err(Error::from)
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            // SCANOPY LOCAL PATCH: upstream snmp2 0.5.0 writes `mac.update(data)`,
            // which is ambiguous (`Mac::update` vs `DynDigest::update`, both in
            // scope) and fails to compile with crypto-rust. Disambiguated to
            // `Mac::update(&mut mac, data)`. Remove this vendored crate once a
            // fixed upstream release is published. See vendor/snmp2/README.md.
            let key = &self.authoritative_state.auth_key;
            match self.auth_protocol {
                AuthProtocol::Md5 => {
                    let mut mac = Hmac::<md5::Md5>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
                AuthProtocol::Sha1 => {
                    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
                AuthProtocol::Sha224 => {
                    let mut mac = Hmac::<sha2::Sha224>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
                AuthProtocol::Sha256 => {
                    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
                AuthProtocol::Sha384 => {
                    let mut mac = Hmac::<sha2::Sha384>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
                AuthProtocol::Sha512 => {
                    let mut mac = Hmac::<sha2::Sha512>::new_from_slice(key)
                        .map_err(|_| Error::Crypto("Invalid key length".into()))?;
                    Mac::update(&mut mac, data);
                    Ok(mac.finalize().into_bytes().to_vec())
                }
            }
        }
    }

    pub(crate) fn update_key(&mut self) -> Result<()> {
        if !self.need_auth() {
            return Ok(());
        }

        self.authoritative_state
            .update_auth_key(&self.authentication_password, self.auth_protocol)?;
        if let Auth::AuthPriv {
            cipher,
            privacy_password,
        } = &self.auth
        {
            self.authoritative_state.update_priv_key(
                privacy_password,
                self.auth_protocol,
                *cipher,
                self.key_extension_method.as_ref(),
            )?;
        }
        Ok(())
    }

    pub fn engine_id(&self) -> &[u8] {
        &self.authoritative_state.engine_id
    }

    /// SCANOPY LOCAL PATCH: the contextName for outgoing scoped PDUs. See
    /// [`Security::with_context_name`].
    pub fn context_name(&self) -> &[u8] {
        &self.context_name
    }

    pub fn engine_boots(&self) -> i64 {
        self.authoritative_state.engine_boots
    }

    pub fn engine_time(&self) -> i64 {
        self.authoritative_state.engine_time
    }

    pub fn username(&self) -> &[u8] {
        &self.username
    }

    /// corrects authoritative state engine time using local monotonic time
    pub(crate) fn correct_authoritative_engine_time(&mut self) {
        self.authoritative_state.correct_engine_time();
    }

    pub(crate) fn need_auth(&self) -> bool {
        self.auth != Auth::NoAuthNoPriv
    }

    pub(crate) fn need_encrypt(&self) -> bool {
        !self.authoritative_state.priv_key.is_empty()
    }

    pub(crate) fn need_init(&self) -> bool {
        self.engine_id().is_empty()
    }

    fn encrypt_des(&self, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut salt = [0; 8];
        salt[..4].copy_from_slice(&u32::try_from(self.engine_boots())?.to_be_bytes());
        #[cfg(feature = "crypto-openssl")]
        openssl::rand::rand_bytes(&mut salt[4..])?;
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            use rand::Rng;
            rand::rng().fill(&mut salt[4..]);
        }

        if data.is_empty() {
            return Ok((vec![], salt.to_vec()));
        }

        if self.authoritative_state.priv_key.len() < 16 {
            return Err(Error::AuthFailure(AuthErrorKind::KeyLengthMismatch));
        }

        let des_key = &self.authoritative_state.priv_key[..8];
        let pre_iv = &self.authoritative_state.priv_key[8..16];

        let mut iv = [0; 8];
        for (i, (a, b)) in pre_iv.iter().zip(salt.iter()).enumerate() {
            iv[i] = a ^ b;
        }

        #[cfg(feature = "crypto-openssl")]
        {
            let cipher = openssl::symm::Cipher::des_cbc();
            let mut encrypted = vec![0; data.len() + cipher.block_size()];
            let mut crypter = openssl::symm::Crypter::new(
                cipher,
                openssl::symm::Mode::Encrypt,
                des_key,
                Some(&iv),
            )?;
            let mut count = crypter.update(data, &mut encrypted)?;

            if count < encrypted.len() {
                count += crypter.finalize(&mut encrypted[count..])?;
            }

            encrypted.truncate(count);
            Ok((encrypted, salt.to_vec()))
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            let mut encryptor = CbcEncryptor::<Des>::new_from_slices(des_key, &iv)
                .map_err(|e| Error::Crypto(e.to_string()))?;

            // PKCS#7 padding
            let len = data.len();
            let pad_len = 8 - (len % 8);
            let mut padded = Vec::with_capacity(len + pad_len);
            padded.extend_from_slice(data);
            padded.extend(std::iter::repeat_n(u8::try_from(pad_len)?, pad_len));

            for chunk in padded.chunks_mut(8) {
                let block = cipher::generic_array::GenericArray::from_mut_slice(chunk);
                encryptor.encrypt_block_mut(block);
            }

            Ok((padded, salt.to_vec()))
        }
    }

    fn encrypt_aes(&self, data: &[u8], key_len: usize) -> Result<(Vec<u8>, Vec<u8>)> {
        let iv_len = 16;
        let mut iv = Vec::with_capacity(iv_len);
        iv.extend_from_slice(&u32::try_from(self.engine_boots())?.to_be_bytes());
        iv.extend_from_slice(&u32::try_from(self.engine_time())?.to_be_bytes());
        let salt_pos = iv.len();
        iv.resize(iv_len, 0);

        #[cfg(feature = "crypto-openssl")]
        openssl::rand::rand_bytes(&mut iv[salt_pos..])?;
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            use rand::Rng;
            rand::rng().fill(&mut iv[salt_pos..]);
        }

        if self.authoritative_state.priv_key.len() < key_len {
            return Err(Error::AuthFailure(AuthErrorKind::KeyLengthMismatch));
        }

        #[cfg(feature = "crypto-openssl")]
        {
            let cipher = match key_len {
                16 => openssl::symm::Cipher::aes_128_cfb128(),
                24 => openssl::symm::Cipher::aes_192_cfb128(),
                32 => openssl::symm::Cipher::aes_256_cfb128(),
                _ => return Err(Error::Crypto("Invalid key len".into())),
            };

            let mut crypter = openssl::symm::Crypter::new(
                cipher,
                openssl::symm::Mode::Encrypt,
                &self.authoritative_state.priv_key[..key_len],
                Some(&iv),
            )?;

            let mut encrypted = vec![0; data.len() + 16];
            let mut count = crypter.update(data, &mut encrypted)?;

            if count < encrypted.len() {
                count += crypter.finalize(&mut encrypted[count..])?;
            }

            encrypted.truncate(count);
            Ok((encrypted, iv[salt_pos..].to_vec()))
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            let key = &self.authoritative_state.priv_key[..key_len];
            let mut encrypted = data.to_vec();

            match key_len {
                16 => {
                    let encryptor = CfbEncryptor::<Aes128>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    encryptor.encrypt(&mut encrypted);
                }
                24 => {
                    let encryptor = CfbEncryptor::<Aes192>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    encryptor.encrypt(&mut encrypted);
                }
                32 => {
                    let encryptor = CfbEncryptor::<Aes256>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    encryptor.encrypt(&mut encrypted);
                }
                _ => return Err(Error::Crypto("Invalid key len".into())),
            }

            Ok((encrypted, iv[salt_pos..].to_vec()))
        }
    }

    /// encrypts the data
    pub(crate) fn encrypt(&self, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let Auth::AuthPriv {
            cipher: cipher_kind,
            ..
        } = &self.auth
        else {
            return Err(Error::AuthFailure(AuthErrorKind::SecurityNotProvided));
        };

        if self.engine_id().is_empty() {
            return Err(Error::AuthFailure(AuthErrorKind::SecurityNotReady));
        }

        match cipher_kind {
            Cipher::Des => self.encrypt_des(data),
            Cipher::Aes128 => self.encrypt_aes(data, 16),
            Cipher::Aes192 => self.encrypt_aes(data, 24),
            Cipher::Aes256 => self.encrypt_aes(data, 32),
        }
    }

    #[cfg(feature = "crypto-openssl")]
    fn decrypt_data_to_plain_buf(
        &mut self,
        mut crypter: openssl::symm::Crypter,
        block_size: usize,
        encrypted: &[u8],
    ) -> Result<()> {
        self.plain_buf.resize(encrypted.len() + block_size, 0);
        let mut count = crypter.update(encrypted, &mut self.plain_buf)?;

        if count < self.plain_buf.len() {
            count += crypter.finalize(&mut self.plain_buf[count..])?;
        }

        self.plain_buf.truncate(count);
        Ok(())
    }

    fn decrypt_des(&mut self, encrypted: &[u8], priv_params: &[u8]) -> Result<()> {
        if priv_params.len() != 8 {
            return Err(Error::AuthFailure(AuthErrorKind::PrivLengthMismatch));
        }

        if self.authoritative_state.priv_key.len() < 16 {
            return Err(Error::AuthFailure(AuthErrorKind::KeyLengthMismatch));
        }

        let des_key = &self.authoritative_state.priv_key[..8];
        let pre_iv = &self.authoritative_state.priv_key[8..16];
        let mut iv = [0; 8];

        for (i, (a, b)) in pre_iv.iter().zip(priv_params.iter()).enumerate() {
            iv[i] = a ^ b;
        }

        #[cfg(feature = "crypto-openssl")]
        {
            let cipher = openssl::symm::Cipher::des_cbc();
            let block_size = 8;

            if !encrypted.len().is_multiple_of(block_size) {
                return Err(Error::AuthFailure(AuthErrorKind::PayloadLengthMismatch));
            }

            let crypter = openssl::symm::Crypter::new(
                cipher,
                openssl::symm::Mode::Decrypt,
                des_key,
                Some(&iv),
            )?;
            self.decrypt_data_to_plain_buf(crypter, block_size, encrypted)
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            let block_size = 8;
            if !encrypted.len().is_multiple_of(block_size) {
                return Err(Error::AuthFailure(AuthErrorKind::PayloadLengthMismatch));
            }

            let mut decryptor = CbcDecryptor::<Des>::new_from_slices(des_key, &iv)
                .map_err(|e| Error::Crypto(e.to_string()))?;

            self.plain_buf.clear();
            self.plain_buf.extend_from_slice(encrypted);

            for chunk in self.plain_buf.chunks_mut(8) {
                let block = cipher::generic_array::GenericArray::from_mut_slice(chunk);
                decryptor.decrypt_block_mut(block);
            }

            // SCANOPY LOCAL PATCH (GH #846): the padding is left in place. RFC 3414 §8.1.1.2
            // makes it arbitrary and §8.1.1.3 has the receiver ignore it, since the scoped PDU's
            // BER length says where it ends. Upstream stripped it as PKCS#7, which net-snmp does
            // not use: a reply that needed no padding lost its last byte to the check, or was
            // refused as "Invalid padding". See vendor/snmp2/README.md.
            Ok(())
        }
    }

    // SCANOPY LOCAL PATCH: the IV takes the boots and time the message carries (RFC 3826
    // §3.1.4), not the cached ones. Upstream overwrote the cache from the message before
    // decrypting, so the two agreed; the cache is now only written once the message passes the
    // timeliness checks, which need the decrypted PDU.
    fn decrypt_aes(
        &mut self,
        encrypted: &[u8],
        priv_params: &[u8],
        key_len: usize,
        engine_boots: i64,
        engine_time: i64,
    ) -> Result<()> {
        let iv_len = 16;
        let mut iv = Vec::with_capacity(iv_len);
        iv.extend_from_slice(&u32::try_from(engine_boots)?.to_be_bytes());
        iv.extend_from_slice(&u32::try_from(engine_time)?.to_be_bytes());
        iv.extend_from_slice(priv_params);

        if iv.len() != iv_len {
            return Err(Error::AuthFailure(AuthErrorKind::PrivLengthMismatch));
        }

        if self.authoritative_state.priv_key.len() < key_len {
            return Err(Error::AuthFailure(AuthErrorKind::KeyLengthMismatch));
        }

        #[cfg(feature = "crypto-openssl")]
        {
            let cipher = match key_len {
                16 => openssl::symm::Cipher::aes_128_cfb128(),
                24 => openssl::symm::Cipher::aes_192_cfb128(),
                32 => openssl::symm::Cipher::aes_256_cfb128(),
                _ => return Err(Error::Crypto("Invalid key len".into())),
            };

            let crypter = openssl::symm::Crypter::new(
                cipher,
                openssl::symm::Mode::Decrypt,
                &self.authoritative_state.priv_key[..key_len],
                Some(&iv),
            )?;

            self.decrypt_data_to_plain_buf(crypter, 16, encrypted)
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            let key = &self.authoritative_state.priv_key[..key_len];
            self.plain_buf.clear();
            self.plain_buf.extend_from_slice(encrypted);

            match key_len {
                16 => {
                    let decryptor = CfbDecryptor::<Aes128>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    decryptor.decrypt(&mut self.plain_buf);
                }
                24 => {
                    let decryptor = CfbDecryptor::<Aes192>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    decryptor.decrypt(&mut self.plain_buf);
                }
                32 => {
                    let decryptor = CfbDecryptor::<Aes256>::new_from_slices(key, &iv)
                        .map_err(|e| Error::Crypto(e.to_string()))?;
                    decryptor.decrypt(&mut self.plain_buf);
                }
                _ => return Err(Error::Crypto("Invalid key len".into())),
            }
            Ok(())
        }
    }

    /// decrypts the data, the result is stored in `self.plain_buf`
    fn decrypt(
        &mut self,
        encrypted: &[u8],
        priv_params: &[u8],
        engine_boots: i64,
        engine_time: i64,
    ) -> Result<()> {
        let Auth::AuthPriv {
            cipher: cipher_kind,
            ..
        } = &self.auth
        else {
            return Err(Error::AuthFailure(AuthErrorKind::SecurityNotProvided));
        };

        let (boots, time) = (engine_boots, engine_time);
        match cipher_kind {
            Cipher::Des => self.decrypt_des(encrypted, priv_params),
            Cipher::Aes128 => self.decrypt_aes(encrypted, priv_params, 16, boots, time),
            Cipher::Aes192 => self.decrypt_aes(encrypted, priv_params, 24, boots, time),
            Cipher::Aes256 => self.decrypt_aes(encrypted, priv_params, 32, boots, time),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Auth {
    NoAuthNoPriv,
    /// Authentication
    AuthNoPriv,
    /// Authentication and encryption
    AuthPriv {
        cipher: Cipher,
        privacy_password: Vec<u8>,
    },
}

#[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
pub struct Hasher(Box<dyn DynDigest + Send + Sync>);

#[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
impl Hasher {
    pub fn new(d: Box<dyn DynDigest + Send + Sync>) -> Result<Self> {
        Ok(Self(d))
    }

    pub fn update(&mut self, data: &[u8]) -> Result<()> {
        self.0.update(data);
        Ok(())
    }

    pub fn finish(&mut self) -> Result<Vec<u8>> {
        Ok(self.0.finalize_reset().to_vec())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AuthProtocol {
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

impl AuthProtocol {
    fn create_hasher(self) -> Result<Hasher> {
        #[cfg(feature = "crypto-openssl")]
        {
            Hasher::new(self.digest()).map_err(Into::into)
        }
        #[cfg(all(feature = "crypto-rust", not(feature = "crypto-openssl")))]
        {
            let d: Box<dyn DynDigest + Send + Sync> = match self {
                AuthProtocol::Md5 => Box::new(md5::Md5::new()),
                AuthProtocol::Sha1 => Box::new(sha1::Sha1::new()),
                AuthProtocol::Sha224 => Box::new(sha2::Sha224::new()),
                AuthProtocol::Sha256 => Box::new(sha2::Sha256::new()),
                AuthProtocol::Sha384 => Box::new(sha2::Sha384::new()),
                AuthProtocol::Sha512 => Box::new(sha2::Sha512::new()),
            };
            Hasher::new(d)
        }
    }
    #[cfg(feature = "crypto-openssl")]
    fn digest(self) -> MessageDigest {
        match self {
            AuthProtocol::Md5 => MessageDigest::md5(),
            AuthProtocol::Sha1 => MessageDigest::sha1(),
            AuthProtocol::Sha224 => MessageDigest::sha224(),
            AuthProtocol::Sha256 => MessageDigest::sha256(),
            AuthProtocol::Sha384 => MessageDigest::sha384(),
            AuthProtocol::Sha512 => MessageDigest::sha512(),
        }
    }

    fn truncation_length(self) -> usize {
        match self {
            AuthProtocol::Md5 | AuthProtocol::Sha1 => 12,
            AuthProtocol::Sha224 => 16,
            AuthProtocol::Sha256 => 24,
            AuthProtocol::Sha384 => 32,
            AuthProtocol::Sha512 => 48,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Cipher {
    Des,
    Aes128,
    Aes192,
    Aes256,
}

impl Cipher {
    pub fn priv_key_len(&self) -> usize {
        match self {
            Cipher::Des | Cipher::Aes128 => 16,
            Cipher::Aes192 => 24,
            Cipher::Aes256 => 32,
        }
    }

    /// Tells if for given auth_protocol and cipher pair, the priv_key is too short and need to be extended.
    ///
    /// The are 5 Auth-Priv pairs, where Auth hasher generates too short input for Priv:
    /// Auth Kul length vs required Priv key length table:
    /// Auth Algorithm   Kul Len   DES (16)   AES-128 (16)   AES-192 (24)   AES-256 (32)
    /// -------------------------------------------------------------------------------
    /// MD5              16        Enough     Enough         EXTEND         EXTEND
    /// SHA-1            20        Enough     Enough         EXTEND         EXTEND
    /// SHA-224          28        Enough     Enough         Enough         EXTEND
    /// SHA-256          32        Enough     Enough         Enough         Enough
    /// SHA-384          48        Enough     Enough         Enough         Enough
    /// SHA-512          64        Enough     Enough         Enough         Enough
    pub fn priv_key_needs_extension(&self, auth_protocol: &AuthProtocol) -> bool {
        #[allow(clippy::match_like_matches_macro, clippy::unnested_or_patterns)]
        match (auth_protocol, self) {
            (AuthProtocol::Md5, Cipher::Aes192)
            | (AuthProtocol::Md5, Cipher::Aes256)
            | (AuthProtocol::Sha1, Cipher::Aes192)
            | (AuthProtocol::Sha1, Cipher::Aes256)
            | (AuthProtocol::Sha224, Cipher::Aes256) => true,
            _ => false,
        }
    }
}

impl<'a> Pdu<'a> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn parse_v3(
        bytes: &'a [u8],
        mut rdr: AsnReader<'a>,
        security: &'a mut Security,
    ) -> Result<Pdu<'a>> {
        let truncation_len = security.auth_protocol.truncation_length();
        let global_data_seq = rdr.read_raw(asn1::TYPE_SEQUENCE)?;
        let mut global_data_rdr = AsnReader::from_bytes(global_data_seq);
        let msg_id = global_data_rdr.read_asn_integer()?;
        let max_size = global_data_rdr.read_asn_integer()?;

        if max_size < 0 || max_size > i64::try_from(BUFFER_SIZE).unwrap() {
            return Err(Error::BufferOverflow);
        }

        let flags = global_data_rdr
            .read_asn_octetstring()?
            .first()
            .copied()
            .unwrap_or_default();

        let security_model = global_data_rdr.read_asn_integer()?;
        if security_model != 3 {
            return Err(Error::AuthFailure(AuthErrorKind::UnsupportedUSM));
        }

        let security_params = rdr.read_asn_octetstring()?;
        let security_seq = AsnReader::from_bytes(security_params).read_raw(asn1::TYPE_SEQUENCE)?;
        let mut security_rdr = AsnReader::from_bytes(security_seq);
        let engine_id = security_rdr.read_asn_octetstring()?;
        let engine_boots = security_rdr.read_asn_integer()?;
        let engine_time = security_rdr.read_asn_integer()?;

        let username = security_rdr.read_asn_octetstring()?;
        let auth_params = security_rdr.read_asn_octetstring().map(<[u8]>::to_vec)?;
        let auth_params_pos =
            bytes.len() - rdr.bytes_left() - auth_params.len() - security_rdr.bytes_left();
        let priv_params = security_rdr.read_asn_octetstring()?;

        // SCANOPY LOCAL PATCH (GH #846): a plaintext Report naming usmStatsDecryptionErrors,
        // once this side holds a privacy key, means the agent holds a different one. Read
        // before the branches below, which report the same message as `NotAuthenticated` (the
        // Report is unauthenticated), the error a wrong auth password gets. AES-256 under MD5
        // or SHA-1 has two key derivations, and the caller needs to tell this failure apart
        // to try the other. See vendor/snmp2/README.md.
        if flags & V3_MSG_FLAGS_PRIVACY == 0
            && security.need_encrypt()
            && is_decryption_error_report(rdr.clone())
        {
            return Err(Error::AuthFailure(AuthErrorKind::DecryptionError));
        }

        // Discovery process requires two steps, each involving a separate request and response pair:
        // - Authoritive engine ID acknowledgement
        //   We expect a non-authenticated and not encrypted response with engine ID
        // - Authoritive engine time synchronization
        //   We expect an authenticated and not encrypted response with engine time and boots
        //
        // Only first step is done when using NoAuthNoPriv security level.
        //
        // See RFC3414 section 4 and section 3.2.7.a
        let mut is_discovery = false;

        if flags & V3_MSG_FLAGS_AUTH == 0 {
            // Unauthenticated REPORT (discovery step)
            // Update engine_id if unknown
            if security.authoritative_state.engine_id.is_empty() {
                security.authoritative_state.engine_id = engine_id.to_vec();
                security.update_key()?;
                is_discovery = true;
            } else if engine_id != security.authoritative_state.engine_id && !engine_id.is_empty() {
                // If agent reports a different engineID, that’s a mismatch
                return Err(Error::AuthFailure(AuthErrorKind::EngineIdMismatch));
            }

            // Many agents include boots/time in the first REPORT.
            //
            // SCANOPY LOCAL PATCH (GH #813): taken as a hint, whatever its value, only until an
            // authenticated message synchronizes the clock. Upstream took it only when boots
            // exceeded the cached value, so an engine reporting boots = 0 never had its time
            // adopted and the first authenticated request went out with time 0. Once
            // synchronized, an unauthenticated message cannot move the clock (RFC 3414 §3.2 step
            // 7 applies only to authenticated messages).
            if !security.authoritative_state.time_synchronized {
                if security.authoritative_state.engine_boots < engine_boots {
                    is_discovery = true;
                }
                security
                    .authoritative_state
                    .update_authoritative(engine_boots, engine_time);
            }

            // When in discovery and we updated state, tell caller to retry
            if is_discovery {
                return Err(Error::AuthUpdated);
            }

            // If we still need auth but haven’t updated state, it’s an auth failure
            if security.need_auth() {
                return Err(Error::AuthFailure(AuthErrorKind::NotAuthenticated));
            }
        } else {
            // Authenticated path
            //
            // SCANOPY LOCAL PATCH (GH #813): upstream rejected boots = 0 here
            // (`EngineBootsNotProvided`) and wrote the message's boots and time into the cache
            // before checking its digest. Both moved below, after authentication and
            // decryption, as RFC 3414 §3.2 steps 6 and 7 order them.

            if username != security.username {
                return Err(Error::AuthFailure(AuthErrorKind::UsernameMismatch));
            }

            if engine_id.is_empty() {
                return Err(Error::AuthFailure(AuthErrorKind::NotAuthenticated));
            }

            if security.authoritative_state.engine_id.is_empty() {
                security.authoritative_state.engine_id = engine_id.to_vec();
                security.update_key()?;
            } else if engine_id != security.authoritative_state.engine_id {
                return Err(Error::AuthFailure(AuthErrorKind::EngineIdMismatch));
            }

            if auth_params.len() != truncation_len
                || auth_params_pos + auth_params.len() > bytes.len()
            {
                return Err(Error::ValueOutOfRange);
            }

            unsafe {
                let auth_params_ptr = bytes.as_ptr().add(auth_params_pos).cast_mut();
                // TODO: switch to safe code as the solution may be pretty fragile
                std::hint::black_box(|| {
                    std::ptr::write_bytes(auth_params_ptr, 0, auth_params.len());
                })();
            }

            if security.need_auth() {
                let hmac = security.calculate_hmac(bytes)?;

                if hmac.len() < truncation_len || hmac[..truncation_len] != auth_params {
                    return Err(Error::AuthFailure(AuthErrorKind::SignatureMismatch));
                }
            }
        }

        let plaintext = flags & V3_MSG_FLAGS_PRIVACY == 0;
        let scoped_pdu_seq = if plaintext {
            rdr.read_raw(asn1::TYPE_SEQUENCE)?
        } else {
            let encrypted_pdu = rdr.read_asn_octetstring()?;
            security.decrypt(encrypted_pdu, priv_params, engine_boots, engine_time)?;
            let mut rdr = AsnReader::from_bytes(&security.plain_buf);
            rdr.read_raw(asn1::TYPE_SEQUENCE)?
        };

        let mut scoped_pdu_rdr = AsnReader::from_bytes(scoped_pdu_seq);

        let _context_engine_id = scoped_pdu_rdr.read_asn_octetstring()?;

        let _context_name = scoped_pdu_rdr.read_asn_octetstring()?;

        let ident = scoped_pdu_rdr.peek_byte()?;

        let message_type = MessageType::from_ident(ident)?;

        if message_type == MessageType::Trap {
            is_discovery = false;
        } else if flags & V3_MSG_FLAGS_AUTH != 0 {
            // SCANOPY LOCAL PATCH (GH #813): RFC 3414 §3.2 step 7b, for a non-authoritative
            // receiver. Before the first authenticated message the cache holds only a discovery
            // hint, so there is nothing yet to judge timeliness against. See
            // vendor/snmp2/README.md.
            let state = &mut security.authoritative_state;
            let was_synchronized = state.time_synchronized;
            if engine_boots >= i64::from(i32::MAX) {
                return Err(Error::AuthFailure(AuthErrorKind::EngineBootsMismatch));
            }
            if was_synchronized {
                if engine_boots < state.engine_boots {
                    return Err(Error::AuthFailure(AuthErrorKind::EngineBootsMismatch));
                }
                if engine_boots == state.engine_boots
                    && engine_time < state.local_engine_time() - ENGINE_TIME_WINDOW
                {
                    return Err(Error::AuthFailure(AuthErrorKind::EngineTimeMismatch));
                }
            }
            // The caller repeats the request when the engine rebooted, or when an unsynchronised
            // request drew a notInTimeWindow Report.
            if engine_boots > state.engine_boots
                || (!was_synchronized && message_type == MessageType::Report)
            {
                is_discovery = true;
            }
            if !was_synchronized
                || engine_boots > state.engine_boots
                || (engine_boots == state.engine_boots && engine_time > state.engine_time)
            {
                state.update_authoritative(engine_boots, engine_time);
            }
            state.time_synchronized = true;
        }

        if plaintext && security.need_encrypt() && !is_discovery {
            return Err(Error::AuthFailure(AuthErrorKind::ReplyNotEncrypted));
        }

        let mut response_pdu = AsnReader::from_bytes(scoped_pdu_rdr.read_raw(ident)?);

        let req_id: i32 = i32::try_from(response_pdu.read_asn_integer()?)?;

        let error_status: u32 =
            u32::try_from(response_pdu.read_asn_integer()?).map_err(|_| Error::ValueOutOfRange)?;

        let error_index: u32 = u32::try_from(response_pdu.read_asn_integer()?)?;

        let varbind_bytes = response_pdu.read_raw(asn1::TYPE_SEQUENCE)?;
        let varbinds = Varbinds::from_bytes(varbind_bytes);

        if is_discovery {
            return Err(Error::AuthUpdated);
        }

        Ok(Pdu {
            version: Version::V3 as i64,
            community: username,
            message_type,
            req_id,
            error_status,
            error_index,
            varbinds,
            v1_trap_info: None,
            v3_msg_id: i32::try_from(msg_id).map_err(|_| Error::ValueOutOfRange)?,
        })
    }
}

pub(crate) fn build_init(req_id: i32, buf: &mut Buf) {
    buf.reset();
    let mut sec_buf = Buf::default();
    sec_buf.push_sequence(|sec| {
        sec.push_octet_string(&[]); // priv params
        sec.push_octet_string(&[]); // auth params
        sec.push_octet_string(&[]); // user name
        sec.push_integer(0); // time
        sec.push_integer(0); // boots
        sec.push_octet_string(&[]); // engine ID
    });
    buf.push_sequence(|message| {
        message.push_sequence(|pdu| {
            pdu.push_constructed(snmp::MSG_GET, |req| {
                req.push_sequence(|_varbinds| {});
                req.push_integer(0); // error index
                req.push_integer(0); // error status
                req.push_integer(req_id.into());
            });
            // contextName then contextEngineID, both null on purpose and not covered by the
            // contextName patch above: engine discovery happens before the engine id is known,
            // and RFC 3414 §4 requires this probe to carry the null context.
            pdu.push_octet_string(&[]);
            pdu.push_octet_string(&[]);
        });
        message.push_octet_string(&sec_buf);
        message.push_sequence(|global| {
            global.push_integer(3); // security_model
            global.push_octet_string(&[V3_MSG_FLAGS_REPORTABLE]); // flags
            global.push_integer(BUFFER_SIZE.try_into().unwrap()); // max_size
            global.push_integer(req_id.into()); // msg_id
        });
        message.push_integer(Version::V3 as i64);
    });
}

pub(crate) fn build(
    ident: u8,
    req_id: i32,
    values: &[(&Oid, Value)],
    non_repeaters: u32,
    max_repetitions: u32,
    buf: &mut Buf,
    security: Option<&Security>,
) -> Result<()> {
    let security = security.ok_or(Error::AuthFailure(AuthErrorKind::SecurityNotProvided))?;
    let truncation_len = security.auth_protocol.truncation_length();
    buf.reset();
    let mut sec_buf_seq = Buf::default();
    sec_buf_seq.reset();
    let mut auth_pos = 0;
    let mut sec_buf_len = 0;
    let mut priv_params: Vec<u8> = Vec::new();
    let mut inner_len = 0;
    let mut flags = V3_MSG_FLAGS_REPORTABLE;

    if security.need_auth() {
        flags |= V3_MSG_FLAGS_AUTH;
    }

    let encrypted = if security.need_encrypt() {
        flags |= V3_MSG_FLAGS_PRIVACY;
        let mut pdu_buf = Buf::default();
        pdu_buf.push_sequence(|buf| {
            pdu::build_inner(req_id, ident, values, max_repetitions, non_repeaters, buf);
            // SCANOPY LOCAL PATCH: contextName, was hard-coded empty upstream. The buffer is
            // written backwards, so this must stay directly above contextEngineID — swapping
            // the two transposes the fields on the wire with no error anywhere.
            buf.push_octet_string(security.context_name());
            buf.push_octet_string(security.engine_id());
        });
        let (encrypted, salt) = security.encrypt(&pdu_buf)?;
        priv_params.extend_from_slice(&salt);
        Some(encrypted)
    } else {
        None
    };

    buf.push_sequence(|buf| {
        if let Some(ref encrypted) = encrypted {
            buf.push_octet_string(encrypted);
        } else {
            buf.push_sequence(|buf| {
                pdu::build_inner(req_id, ident, values, max_repetitions, non_repeaters, buf);
                // SCANOPY LOCAL PATCH: contextName — the plaintext twin of the branch above.
                // Both must carry it or an AuthNoPriv session addresses a different context
                // from an AuthPriv one with the same credential.
                buf.push_octet_string(security.context_name());
                buf.push_octet_string(security.engine_id());
            });
        }
        let l0 = buf.len();
        sec_buf_seq.push_sequence(|buf| {
            buf.push_octet_string(&priv_params); // priv params
            let l0 = buf.len() - priv_params.len();
            buf.push_octet_string(&vec![0u8; truncation_len]); // auth params
            let l1 = buf.len() - l0;
            buf.push_octet_string(security.username()); // user name
            buf.push_integer(security.engine_time()); // time
            buf.push_integer(security.engine_boots()); // boots
            buf.push_octet_string(security.engine_id()); // engine ID
            auth_pos = buf.len() - l1;
            sec_buf_len = buf.len();
        });
        buf.push_octet_string(&sec_buf_seq);
        buf.push_sequence(|buf| {
            buf.push_integer(3); // security_model
            buf.push_octet_string(&[flags]); // flags
            buf.push_integer(BUFFER_SIZE.try_into().unwrap()); // max_size
            buf.push_integer(req_id.into()); // msg_id
        });
        buf.push_integer(3); // version
        auth_pos = buf.len() - l0 - (sec_buf_len - auth_pos);
        inner_len = buf.len();
    });

    auth_pos += buf.len() - inner_len;
    if (auth_pos + truncation_len) > buf.len() {
        return Err(Error::ValueOutOfRange);
    }

    if security.need_auth() {
        let hmac = security.calculate_hmac(buf)?;
        buf[auth_pos..auth_pos + truncation_len].copy_from_slice(&hmac[..truncation_len]);
    }

    Ok(())
}

/// SCANOPY LOCAL PATCH tests (GH #846): DES replies as net-snmp pads them.
///
/// RFC 3414 §8.1.1.2 leaves DES padding arbitrary and §8.1.1.3 has the receiver ignore it: the
/// scoped PDU's own BER length says where it ends. net-snmp adds no padding when the scoped PDU
/// is already a multiple of 8 bytes, and pads with bytes that are not a PKCS#7 length otherwise.
/// These encrypt the way such an agent does, rather than through `encrypt_des`, which pads the way
/// this crate's own decryption expects and so cannot catch a mismatch.
#[cfg(all(test, feature = "crypto-rust", not(feature = "crypto-openssl")))]
mod des_padding {
    use super::*;

    const SALT: [u8; 8] = [0, 0, 0, 1, 0xde, 0xad, 0xbe, 0xef];

    fn user() -> Security {
        Security::new(b"scanopydes", b"desauthpass12345")
            .with_auth_protocol(AuthProtocol::Sha1)
            .with_auth(Auth::AuthPriv {
                cipher: Cipher::Des,
                privacy_password: b"desprivpass12345".to_vec(),
            })
            .with_engine_id(&[0x80, 0x00, 0x1f, 0x88, 0x04, 0x01])
            .unwrap()
    }

    /// `plaintext` encrypted with `security`'s DES key under `SALT`, with no padding of its own.
    fn encrypt_unpadded(security: &Security, plaintext: &[u8]) -> Vec<u8> {
        assert!(plaintext.len().is_multiple_of(8));
        let key = &security.authoritative_state.priv_key;
        let mut iv = [0u8; 8];
        for (i, (a, b)) in key[8..16].iter().zip(SALT.iter()).enumerate() {
            iv[i] = a ^ b;
        }
        let mut encryptor = CbcEncryptor::<Des>::new_from_slices(&key[..8], &iv).unwrap();
        let mut out = plaintext.to_vec();
        for chunk in out.chunks_mut(8) {
            encryptor.encrypt_block_mut(cipher::generic_array::GenericArray::from_mut_slice(chunk));
        }
        out
    }

    /// The scoped PDU `decrypt_des` leaves in `plain_buf`, read as the parser reads it.
    fn decrypted_sequence(plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut security = user();
        let encrypted = encrypt_unpadded(&security, plaintext);
        security.decrypt_des(&encrypted, &SALT)?;
        let mut rdr = AsnReader::from_bytes(&security.plain_buf);
        Ok(rdr.read_raw(asn1::TYPE_SEQUENCE)?.to_vec())
    }

    /// A 16-byte SEQUENCE whose last content byte is 0x00, sent with no padding at all.
    #[test]
    fn a_reply_with_no_padding_decrypts_whole() {
        let sequence = [0x30, 0x0e, 0x04, 0x0c, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0];
        assert_eq!(decrypted_sequence(&sequence).unwrap(), sequence[2..]);
    }

    /// A 16-byte SEQUENCE whose last content byte happens to read as a PKCS#7 length.
    #[test]
    fn a_reply_ending_in_a_byte_that_looks_like_padding_is_not_cut_short() {
        let sequence = [0x30, 0x0e, 0x04, 0x0c, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 3];
        assert_eq!(decrypted_sequence(&sequence).unwrap(), sequence[2..]);
    }

    /// A 13-byte SEQUENCE padded to 16 with zero bytes.
    #[test]
    fn a_reply_padded_with_zeros_decrypts() {
        let mut padded = vec![0x30, 0x0b, 0x04, 0x09, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        padded.resize(16, 0);
        assert_eq!(decrypted_sequence(&padded).unwrap(), padded[2..13]);
    }
}
