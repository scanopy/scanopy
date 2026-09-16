use jsonwebtoken::{DecodingKey, EncodingKey};

/// The public key used to verify license JWTs, embedded at compile time from
/// the PEM file in this directory. Unit tests trust [`test_keys`] instead so
/// they can sign keys and entitlements; every other build, integration tests
/// included, trusts only the production key.
#[cfg(not(test))]
const PUBLIC_KEY_PEM: &[u8] = include_bytes!("public_key.pem");
#[cfg(test)]
const PUBLIC_KEY_PEM: &[u8] = test_keys::PUBLIC_KEY.as_bytes();

/// Get the decoding (verification) key for license JWTs.
pub fn decoding_key() -> DecodingKey {
    DecodingKey::from_ed_pem(PUBLIC_KEY_PEM).expect("Bundled public key must be valid Ed25519 PEM")
}

/// Parse an Ed25519 private signing key from PEM.
pub fn encoding_key_from_pem(key_pem: &str) -> anyhow::Result<EncodingKey> {
    Ok(EncodingKey::from_ed_pem(key_pem.as_bytes())?)
}

/// Load the private signing key from the `SCANOPY_LICENSE_SIGNING_KEY` env var.
/// Used by the license CLI tool. The cloud server reads the same variable
/// through `ServerConfig::license_signing_key`; it is never present on
/// customer servers.
pub fn encoding_key_from_env() -> anyhow::Result<EncodingKey> {
    let key_pem = std::env::var("SCANOPY_LICENSE_SIGNING_KEY")
        .map_err(|_| anyhow::anyhow!("SCANOPY_LICENSE_SIGNING_KEY env var not set"))?;
    encoding_key_from_pem(&key_pem)
}

#[cfg(test)]
pub(crate) mod test_keys {
    /// Test-only Ed25519 keypair, unrelated to the production signing key and
    /// compiled only into `cfg(test)` builds. Tests that verify through
    /// [`super::decoding_key`] sign with `PRIVATE_KEY`; tests that inject their
    /// own verification key use both halves directly.
    pub const PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIKbHD9jZVew/xQZxpo+jfYTQnZMHUNUK3EZPEYVwE1TL
-----END PRIVATE KEY-----
";
    pub const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAook5qHgu6TfUZ3SRN1UpztcrryUarXRkoUBf26YRvDg=
-----END PUBLIC KEY-----
";

    /// The production verification key, which [`super::decoding_key`] does not
    /// return in test builds. A token signed with `PRIVATE_KEY` never verifies
    /// against it, which is how tests exercise a foreign signature.
    pub fn production_decoding_key() -> jsonwebtoken::DecodingKey {
        jsonwebtoken::DecodingKey::from_ed_pem(include_bytes!("public_key.pem"))
            .expect("Bundled public key must be valid Ed25519 PEM")
    }
}

#[cfg(test)]
pub(crate) mod test_signing {
    use super::test_keys;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use serde::Serialize;

    /// Sign `claims` with the test key that unit-test builds trust.
    pub fn sign(claims: &impl Serialize) -> String {
        let key = EncodingKey::from_ed_pem(test_keys::PRIVATE_KEY.as_bytes())
            .expect("test key is valid Ed25519 PEM");
        jsonwebtoken::encode(&Header::new(Algorithm::EdDSA), claims, &key)
            .expect("signing with the test key succeeds")
    }
}
