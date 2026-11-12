//! SNMP Session Management
//!
//! Functions for creating and managing SNMP sessions.

use anyhow::{Result, anyhow};
use snmp2::AsyncSession;
use snmp2::v3::{AuthErrorKind, KeyExtension};
use std::net::IpAddr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tokio::time::timeout;

use crate::server::credentials::r#impl::mapping::{SnmpQueryCredential, SnmpV3Params};
use crate::server::credentials::r#impl::types::{
    SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpV3Security, SnmpVersion,
};

impl From<SnmpV3AuthProtocol> for snmp2::v3::AuthProtocol {
    fn from(p: SnmpV3AuthProtocol) -> Self {
        match p {
            SnmpV3AuthProtocol::Md5 => snmp2::v3::AuthProtocol::Md5,
            SnmpV3AuthProtocol::Sha1 => snmp2::v3::AuthProtocol::Sha1,
            SnmpV3AuthProtocol::Sha256 => snmp2::v3::AuthProtocol::Sha256,
        }
    }
}

impl From<SnmpV3PrivProtocol> for snmp2::v3::Cipher {
    fn from(p: SnmpV3PrivProtocol) -> Self {
        match p {
            SnmpV3PrivProtocol::Aes128 => snmp2::v3::Cipher::Aes128,
            SnmpV3PrivProtocol::Aes256 => snmp2::v3::Cipher::Aes256,
            SnmpV3PrivProtocol::Des => snmp2::v3::Cipher::Des,
        }
    }
}

/// Default timeout for SNMP operations
pub const SNMP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for SNMP session creation (UDP socket setup)
pub const SNMP_SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Overall timeout for a single SNMP liveness probe (one credential, one port). Caps
/// the whole create-session + sysDescr GET so a non-responder — especially v3, whose
/// engine-discovery handshake otherwise waits the full 5s SNMP_SESSION_TIMEOUT — costs
/// ~2s instead of up to 7s. A responsive device answers in well under this on a LAN.
pub const SNMP_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long an engine that answered discovery has to answer the first encrypted request before
/// an AES-256 credential tries its other key extension (GH #846).
///
/// A net-snmp agent holding a different privacy key says nothing at all: the request decrypts
/// to noise and is dropped (lab capture, net-snmp 5.9.4: discovery Report, then silence). So
/// the signal is a wait, and it has to leave room inside [`SNMP_PROBE_TIMEOUT`] for the second
/// handshake and the probe's GET. A host that passes the probe at all answers three round trips
/// inside that cap, so it answers one inside this.
pub const SNMP_KEY_EXTENSION_WAIT: Duration = Duration::from_millis(750);

/// Default timeout for table walks (longer since they involve multiple requests).
///
/// Sized against real switches rather than a round number: a busy device's bridge FDB and
/// per-port VLAN membership are the two largest tables SNMP reads, and at 30s they were being
/// cut short often enough that operators saw "incomplete" warnings on every scan. Raising this
/// only costs time on devices that are genuinely slow — a healthy walk returns as soon as it is
/// done. Keep [`super::SnmpIntegration::timeout`] above `13 * SNMP_WALK_TIMEOUT`, since the
/// walks run sequentially and the outer cap would otherwise kill the last ones first.
pub const SNMP_WALK_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of varbinds to process in a single walk
pub const MAX_WALK_ENTRIES: usize = 10000;

/// A starting request id unique to this session.
///
/// Every session used to start at 0, so hosts scanned concurrently issued identical request ids in
/// lockstep — leaving the community string as the only thing telling one collection's responses
/// from another's. RFC 3416 asks for request ids that are hard to predict; a process-wide counter
/// gives each session its own range at no cost.
fn starting_request_id() -> i32 {
    static NEXT: AtomicI32 = AtomicI32::new(0);
    // Stride so two long-running sessions can't converge on the same id.
    NEXT.fetch_add(0x0001_0000, Ordering::Relaxed)
}

/// One host's SNMP session, and what this collection has learned about how to talk to it.
///
/// A newtype rather than the bare `AsyncSession` because the walk needs somewhere to record that
/// getbulk does not work here. That verdict belongs to the device and holds for the whole
/// collection: GH #668's switch3 spent fifteen seconds discovering it on `lldpRemChassisId` and
/// then fifteen more on the very next column, because the flag driving it lived in `walk_subtree`
/// and died with each walk. A session is the right lifetime — it is opened per host and dropped
/// when that host is done, so nothing carries between hosts or between scans.
///
/// `Deref`/`DerefMut` keep every direct use of the session (the probe's `get`, the v3 `init`)
/// working unchanged; the walk reaches it through [`SnmpWalkTransport`] instead.
pub struct SnmpSession {
    inner: Box<AsyncSession>,
    getbulk_unusable: bool,
    /// See [`SnmpWalkTransport::bulk_page_limit`].
    ///
    /// [`SnmpWalkTransport::bulk_page_limit`]: super::queries::SnmpWalkTransport::bulk_page_limit
    bulk_page_limit: Option<u32>,
    /// The discovery's cancellation, checked between the requests of a walk. A table walk can run
    /// for a minute or more, and nothing else in it looks at the token.
    cancel: Option<tokio_util::sync::CancellationToken>,
    /// The key extension this host's agent answered, for a credential whose privacy key needs
    /// one. `None` for every other credential.
    key_extension: Option<KeyExtension>,
}

impl SnmpSession {
    fn new(inner: AsyncSession, key_extension: Option<KeyExtension>) -> Self {
        Self {
            inner: Box::new(inner),
            getbulk_unusable: false,
            bulk_page_limit: None,
            cancel: None,
            key_extension,
        }
    }

    /// The key extension to open this host's later sessions with. See [`create_session`].
    pub fn key_extension(&self) -> Option<KeyExtension> {
        self.key_extension
    }

    /// Stop walks on this session when `token` is cancelled.
    pub fn watch_cancel(&mut self, token: tokio_util::sync::CancellationToken) {
        self.cancel = Some(token);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    }

    /// Whether a walk on this host has already found getbulk unanswerable.
    pub fn getbulk_unusable(&self) -> bool {
        self.getbulk_unusable
    }

    pub fn note_getbulk_unusable(&mut self) {
        self.getbulk_unusable = true;
    }

    /// The page size a walk on this host fell back to after a page went unanswered.
    pub fn bulk_page_limit(&self) -> Option<u32> {
        self.bulk_page_limit
    }

    pub fn note_bulk_page_limit(&mut self, max_repetitions: u32) {
        self.bulk_page_limit = Some(max_repetitions);
    }
}

impl std::ops::Deref for SnmpSession {
    type Target = AsyncSession;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for SnmpSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Create an SNMP session with the given credentials.
///
/// Returns a boxed session because `AsyncSession` contains ~131KB of stack-allocated
/// buffers (recv_buf + send_pdu). Without boxing, the async state machines that hold
/// a session overflow the tokio worker thread stacks in debug builds — the daemon
/// uses 4MB stacks, but `deep_scan_host`'s state machine (which includes SNMP query
/// sub-futures) is large enough in debug mode to overflow.
/// Which SNMPv3 context a session addresses.
///
/// Named at every call site rather than taken from the credential, because the two answers are
/// not interchangeable and the wrong one is silent. A credential's context names where that
/// device keeps its *bridge* data — Cisco IOS-XE partitions the forwarding database per VLAN and
/// serves ifTable, LLDP and ARP from the default context regardless — and only one SNMP
/// credential per host ever executes (`dispatch.rs` keys the winner by integration), so a context
/// applied to the whole session would take every other walk with it and leave the switch with no
/// interfaces at all. Non-v3 sessions ignore this: v1/v2c has no context field, and the
/// equivalent on Cisco is the `community@vlan` form, which is part of the community string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnmpContext {
    /// The default (empty) context, which is where every device serves its standard MIBs.
    Default,
    /// The context named on the credential, if it names one. Bridge and VLAN tables only.
    FromCredential,
}

impl SnmpV3Params {
    /// The contextName sent in each scoped PDU for `context`.
    ///
    /// The context name addresses a MIB view other than the default one. Cisco IOS-XE keeps its
    /// per-VLAN bridge FDB in one, so a switch read without it answers from a default context
    /// holding almost nothing — a full forwarding database that reads as a single row (GH #686).
    /// Applied only where the caller asked for it; see [`SnmpContext`] for why that is not the
    /// whole session.
    pub fn wire_context(&self, context: SnmpContext) -> &str {
        match context {
            SnmpContext::Default => "",
            SnmpContext::FromCredential => self.context_name.as_deref().unwrap_or_default(),
        }
    }

    /// Whether the privacy key is shorter than its cipher needs: AES-256 under MD5 or SHA-1.
    ///
    /// Two ways of stretching it are deployed and the credential does not say which the device
    /// uses, so these are the credentials [`create_session`] may have to try twice.
    pub fn extends_privacy_key(&self) -> bool {
        match &self.security {
            SnmpV3Security::AuthPriv {
                auth_protocol,
                priv_protocol,
                ..
            } => snmp2::v3::Cipher::from(*priv_protocol)
                .priv_key_needs_extension(&(*auth_protocol).into()),
            SnmpV3Security::NoAuthNoPriv | SnmpV3Security::AuthNoPriv { .. } => false,
        }
    }

    /// The snmp2 USM security for these parameters, at their security level.
    ///
    /// AES-256 with an MD5 or SHA-1 key needs the localized key stretched, and snmp2 refuses the
    /// pair without a method. `key_extension` is that method: Blumenthal is what net-snmp's
    /// `AES-256` uses, Reeder what Cisco IOS's `aes 256` and net-snmp's `AES-256-C` use. snmp2
    /// ignores it for every pair that needs no stretching.
    pub fn usm_security(
        &self,
        context: SnmpContext,
        key_extension: KeyExtension,
    ) -> Result<snmp2::v3::Security> {
        use snmp2::v3::{Auth, Security};

        let name = self.security_name.as_bytes();
        let security = match &self.security {
            SnmpV3Security::NoAuthNoPriv => Security::new(name, b"").with_auth(Auth::NoAuthNoPriv),
            SnmpV3Security::AuthNoPriv {
                auth_protocol,
                auth_password,
            } => {
                let auth_pw = auth_password.resolve("auth_password", "SNMPv3")?;
                Security::new(name, auth_pw.expose_secret().as_bytes())
                    .with_auth_protocol((*auth_protocol).into())
                    .with_auth(Auth::AuthNoPriv)
            }
            SnmpV3Security::AuthPriv {
                auth_protocol,
                auth_password,
                priv_protocol,
                priv_password,
            } => {
                let auth_pw = auth_password.resolve("auth_password", "SNMPv3")?;
                let priv_pw = priv_password.resolve("priv_password", "SNMPv3")?;
                Security::new(name, auth_pw.expose_secret().as_bytes())
                    .with_auth_protocol((*auth_protocol).into())
                    .with_auth(Auth::AuthPriv {
                        cipher: (*priv_protocol).into(),
                        privacy_password: priv_pw.expose_secret().as_bytes().to_vec(),
                    })
                    .with_key_extension_method(key_extension)
            }
        };
        Ok(security.with_context_name(self.wire_context(context).as_bytes()))
    }
}

/// Whether this credential names a bridge context worth opening a second session for.
///
/// `None` for v1/v2c and for a v3 credential that leaves the field blank, which is the common
/// case and must cost nothing — a second session means a second engine-discovery handshake.
pub fn bridge_context(credential: &SnmpQueryCredential) -> Option<&str> {
    credential
        .v3
        .as_ref()?
        .context_name
        .as_deref()
        .filter(|name| !name.is_empty())
}

/// This is the last place a credential field can still reach the device, so both structs are
/// destructured exhaustively under `#[deny(unused_variables)]` rather than read field-by-field.
/// A field added to either one then fails to compile here as "missing field in pattern", and one
/// that is bound but never put on the wire fails as an unused binding — which is exactly how
/// `context_name` came to be stored, rendered as a form field and documented for three releases
/// without ever being transmitted (GH #686). Skipping a field stays possible, but only by writing
/// `field: _` with the reason beside it. Same guard as `ServiceFactory::all_services`.
///
/// `key_extension` is what an earlier session to this host learned
/// ([`SnmpSession::key_extension`]), or `None` for the first one. It matters only to an AES-256
/// credential under MD5 or SHA-1 (GH #846), where the device stretches the privacy key one of
/// two ways and nothing but trying tells which:
///
/// - `None`: Blumenthal first. If the engine answers discovery and then either stays silent to
///   the first encrypted request for [`SNMP_KEY_EXTENSION_WAIT`] or reports
///   `usmStatsDecryptionErrors`, the handshake is repeated once with Reeder. A host that does
///   not answer discovery is never retried, so an offline address costs what it always did, and
///   a wrong auth password or user is answered with a different Report and is not retried.
/// - `Some`: that method, with the full timeout and no second try. The host has already
///   answered it in this scan.
#[deny(unused_variables)]
pub async fn create_session(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
    context: SnmpContext,
    key_extension: Option<KeyExtension>,
) -> Result<SnmpSession> {
    let target = format!("{}:{}", ip, port);

    let SnmpQueryCredential {
        version,
        community,
        v3,
    } = credential;

    match version {
        SnmpVersion::V1 | SnmpVersion::V2c => {
            let is_v1 = matches!(version, SnmpVersion::V1);
            // Verbatim on the wire, which is what makes Cisco's `community@vlan` indexing work:
            // the VLAN is part of the string the device parses, not a field of our own.
            let community_secret = community.resolve("community", "SNMP")?;
            let community = community_secret.expose_secret();
            tracing::debug!(
                ip = %ip,
                version = %version,
                community_len = community.len(),
                "Creating SNMP community session"
            );

            // v1 and v2c differ only in the wire protocol version negotiated;
            // both authenticate with a community string.
            let create = async {
                if is_v1 {
                    AsyncSession::new_v1(&target, community.as_bytes(), starting_request_id()).await
                } else {
                    AsyncSession::new_v2c(&target, community.as_bytes(), starting_request_id())
                        .await
                }
            };

            match timeout(SNMP_SESSION_TIMEOUT, create).await {
                Ok(Ok(session)) => {
                    tracing::debug!(ip = %ip, "SNMP session created successfully");
                    Ok(SnmpSession::new(session, None))
                }
                Ok(Err(e)) => Err(anyhow!(
                    "Failed to create SNMP{} session to {}: {:?}",
                    if is_v1 { "v1" } else { "v2c" },
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!(
                    "Timeout creating SNMP session to {} ({}s)",
                    ip,
                    SNMP_SESSION_TIMEOUT.as_secs()
                )),
            }
        }
        SnmpVersion::V3 => {
            let params = v3
                .as_ref()
                .ok_or_else(|| anyhow!("SNMPv3 credential missing USM parameters"))?;
            let wire_context = params.wire_context(context);
            let extends_privacy_key = params.extends_privacy_key();
            // Whether the other key extension is still worth a second handshake.
            let untried = extends_privacy_key && key_extension.is_none();
            let first = key_extension.unwrap_or(KeyExtension::Blumenthal);
            let security = params.usm_security(context, first)?;

            tracing::debug!(
                ip = %ip,
                security_name = %params.security_name,
                security_level = ?params.security.level(),
                context = if wire_context.is_empty() { "(default)" } else { wire_context },
                "Creating SNMPv3 session"
            );

            let mut session = match timeout(
                SNMP_SESSION_TIMEOUT,
                AsyncSession::new_v3(&target, starting_request_id(), security),
            )
            .await
            {
                Ok(Ok(session)) => session,
                Ok(Err(e)) => {
                    return Err(anyhow!(
                        "Failed to create SNMPv3 session to {}: {:?}",
                        ip,
                        e
                    ));
                }
                Err(_) => {
                    return Err(anyhow!(
                        "Timeout creating SNMPv3 session to {} ({}s)",
                        ip,
                        SNMP_SESSION_TIMEOUT.as_secs()
                    ));
                }
            };

            // SNMPv3 requires an engine-discovery handshake before queries so the
            // session learns the authoritative engine ID / boots / time. Its two steps are
            // timed apart: the first is unauthenticated and says only that an SNMPv3 engine is
            // there, the second is the first request the credential's keys are used on.
            match timeout(SNMP_SESSION_TIMEOUT, session.discover_engine()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    return Err(anyhow!(
                        "Failed SNMPv3 engine discovery / authentication to {}: {:?}",
                        ip,
                        e
                    ));
                }
                Err(_) => return Err(anyhow!("Timeout during SNMPv3 engine discovery to {}", ip)),
            }

            let wait = if untried {
                SNMP_KEY_EXTENSION_WAIT
            } else {
                SNMP_SESSION_TIMEOUT
            };
            let failure = match timeout(wait, session.synchronize_time()).await {
                Ok(Ok(())) => {
                    tracing::debug!(ip = %ip, "SNMPv3 session initialized");
                    return Ok(SnmpSession::new(
                        session,
                        extends_privacy_key.then_some(first),
                    ));
                }
                Ok(Err(e)) => Some(e),
                Err(_) => None,
            };
            let undecryptable = matches!(
                failure,
                None | Some(snmp2::Error::AuthFailure(AuthErrorKind::DecryptionError))
            );
            if !(untried && undecryptable) {
                return Err(match failure {
                    Some(e) => anyhow!(
                        "Failed SNMPv3 engine discovery / authentication to {}: {:?}",
                        ip,
                        e
                    ),
                    None => anyhow!(
                        "The SNMPv3 engine at {} answered discovery and then no authenticated \
                         request",
                        ip
                    ),
                });
            }

            tracing::debug!(
                ip = %ip,
                first = ?first,
                "SNMPv3 engine could not read a request encrypted with the first AES-256 key \
                 extension; trying the other"
            );
            match timeout(
                SNMP_KEY_EXTENSION_WAIT,
                session.try_another_key_extension_method(),
            )
            .await
            {
                Ok(Ok(Some(answered))) => {
                    tracing::debug!(ip = %ip, key_extension = ?answered, "SNMPv3 session initialized");
                    Ok(SnmpSession::new(session, Some(answered)))
                }
                Ok(Ok(None)) => Err(anyhow!(
                    "SNMPv3 credential for {} has no second key extension to try",
                    ip
                )),
                Ok(Err(e)) => Err(anyhow!(
                    "The SNMPv3 engine at {} could not read a request encrypted with either \
                     AES-256 key extension (check the privacy password): {:?}",
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!(
                    "The SNMPv3 engine at {} answered discovery and then no request encrypted \
                     with either AES-256 key extension (check the privacy password)",
                    ip
                )),
            }
        }
    }
}

/// Regressions for a session that has lost sync with its own traffic.
///
/// The daemon abandons SNMP requests constantly — a 5s cap per query and 30s per walk — and then
/// keeps using the same session for the next query. That makes request-id hygiene load-bearing
/// rather than theoretical: a timed-out request leaves its answer queued on the socket, so if the
/// next request reuses the id, that stale answer validates and every subsequent response is one
/// behind for the life of the session. Truncated columns then look like completed ones.
#[cfg(test)]
mod session_sync_tests {
    use super::*;
    use snmp2::{AsyncSession, Oid};
    use tokio::net::UdpSocket;

    const SYS_DESCR: &[u64] = &[1, 3, 6, 1, 2, 1, 1, 1, 0];

    /// A socket that receives requests and answers only when told to.
    async fn silent_agent() -> (UdpSocket, String) {
        let agent = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = agent.local_addr().unwrap().to_string();
        (agent, addr)
    }

    async fn abandon_one_request(session: &mut AsyncSession) {
        let oid = Oid::from(SYS_DESCR).unwrap();
        // Drop the future mid-`recv`, exactly as `query_or_default` does on its timeout.
        let _ = timeout(Duration::from_millis(150), session.get(&oid)).await;
    }

    /// A request that is abandoned must still consume its id. Reusing it is what lets the
    /// previous request's answer satisfy the next one.
    #[tokio::test]
    async fn an_abandoned_request_does_not_reuse_its_id() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 0).await.unwrap();

        let mut ids = Vec::new();
        for _ in 0..2 {
            abandon_one_request(&mut session).await;
            let mut buf = [0u8; 2048];
            let (len, _) = agent.recv_from(&mut buf).await.unwrap();
            ids.push(snmp2::pdu::Pdu::from_bytes(&buf[..len]).unwrap().req_id);
        }

        assert_ne!(
            ids[0], ids[1],
            "the second request reused the abandoned request's id"
        );
    }

    /// The answer to an abandoned request is still sitting in the socket buffer. The next request
    /// must not read it: it belongs to a question nobody is waiting for.
    #[tokio::test]
    async fn a_late_answer_is_not_served_to_the_next_request() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 0).await.unwrap();

        abandon_one_request(&mut session).await;

        // The agent answers after the caller gave up; the datagram queues on the session socket.
        let mut buf = [0u8; 2048];
        let (len, from) = agent.recv_from(&mut buf).await.unwrap();
        agent.send_to(&buf[..len], from).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // The next request goes unanswered, so it must stay pending. If the queued datagram were
        // consumed the call would resolve immediately instead — right or wrong, but not pending.
        let oid = Oid::from(SYS_DESCR).unwrap();
        let outcome = timeout(Duration::from_millis(300), session.get(&oid)).await;

        assert!(
            outcome.is_err(),
            "the stale datagram was served to a request that never got an answer"
        );
    }

    /// The Response an agent sends to `request`, with `text` as `sysDescr` and `request_id` in
    /// place of the request's own when given.
    fn answer_to(request: &[u8], text: &str, request_id: Option<i32>) -> Vec<u8> {
        use crate::daemon::discovery::integration::snmp::sim::udp::{encode_varbinds, tlv};

        let varbinds = encode_varbinds(&[(SYS_DESCR.to_vec(), tlv(0x04, text.as_bytes()))]);
        let mut reply = snmp2::Pdu::from_bytes(request).unwrap();
        reply.message_type = snmp2::MessageType::Response;
        reply.req_id = request_id.unwrap_or(reply.req_id);
        reply.varbinds = snmp2::Varbinds::from_bytes(&varbinds);
        reply.to_bytes().unwrap()
    }

    /// GH #848: the agent takes longer than the timeout over one request and answers it just
    /// after the next request has gone out, then answers that one. The drain before the send
    /// cannot help, because the late answer was not there yet. The caller must get the answer to
    /// its own request.
    #[tokio::test]
    async fn an_answer_arriving_after_the_next_request_is_sent_is_passed_over() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 0).await.unwrap();

        abandon_one_request(&mut session).await;
        let mut abandoned = [0u8; 2048];
        let (abandoned_len, _) = agent.recv_from(&mut abandoned).await.unwrap();

        let answers = async {
            let mut current = [0u8; 2048];
            let (current_len, from) = agent.recv_from(&mut current).await.unwrap();
            let late = answer_to(&abandoned[..abandoned_len], "late", None);
            agent.send_to(&late, from).await.unwrap();
            let own = answer_to(&current[..current_len], "current", None);
            agent.send_to(&own, from).await.unwrap();
        };
        let oid = Oid::from(SYS_DESCR).unwrap();
        let (response, ()) =
            tokio::join!(timeout(Duration::from_secs(1), session.get(&oid)), answers);

        let response = response
            .expect("the request's own answer was sent")
            .expect("the late answer is not this request's to fail on");
        let (_, value) = response.varbinds.clone().next().unwrap();
        assert!(
            matches!(value, snmp2::Value::OctetString(b"current")),
            "read {value:?}"
        );
    }

    /// Only an answer to a request this session sent earlier is passed over. One carrying an id
    /// the session has not issued is nobody's late answer, and the request fails on it.
    #[tokio::test]
    async fn an_answer_to_a_request_never_sent_fails_the_request() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 40).await.unwrap();

        let answers = async {
            let mut request = [0u8; 2048];
            let (len, from) = agent.recv_from(&mut request).await.unwrap();
            for foreign in [39, 41] {
                let reply = answer_to(&request[..len], "foreign", Some(foreign));
                agent.send_to(&reply, from).await.unwrap();
            }
        };
        let oid = Oid::from(SYS_DESCR).unwrap();
        let (response, ()) =
            tokio::join!(timeout(Duration::from_secs(1), session.get(&oid)), answers);

        assert_eq!(
            response.expect("an answer arrived").err(),
            Some(snmp2::Error::RequestIdMismatch)
        );
    }
}
