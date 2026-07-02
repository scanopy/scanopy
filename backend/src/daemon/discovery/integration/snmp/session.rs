//! SNMP Session Management
//!
//! Functions for creating and managing SNMP sessions.

use anyhow::{Result, anyhow};
use snmp2::AsyncSession;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time::timeout;

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;
use crate::server::credentials::r#impl::types::{
    SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpVersion,
};

impl From<SnmpV3AuthProtocol> for snmp2::v3::AuthProtocol {
    fn from(p: SnmpV3AuthProtocol) -> Self {
        match p {
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
        }
    }
}

/// Default timeout for SNMP operations
pub const SNMP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for SNMP session creation (UDP socket setup)
pub const SNMP_SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for table walks (longer since they involve multiple requests)
pub const SNMP_WALK_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of varbinds to process in a single walk
pub const MAX_WALK_ENTRIES: usize = 10000;

/// Max varbinds requested per GETBULK PDU. Kept modest so responses fit in a
/// single UDP datagram even for columns with long string values (ifAlias/ifDescr).
pub const BULK_MAX_REPETITIONS: u32 = 20;

/// Create an SNMP session with the given credentials.
///
/// Returns a boxed session because `AsyncSession` contains ~131KB of stack-allocated
/// buffers (recv_buf + send_pdu). Without boxing, the async state machines that hold
/// a session overflow the tokio worker thread stacks in debug builds — the daemon
/// uses 4MB stacks, but `deep_scan_host`'s state machine (which includes SNMP query
/// sub-futures) is large enough in debug mode to overflow.
pub async fn create_session(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Box<AsyncSession>> {
    let target = format!("{}:{}", ip, port);

    match credential.version {
        SnmpVersion::V1 | SnmpVersion::V2c => {
            let is_v1 = matches!(credential.version, SnmpVersion::V1);
            let community_secret = credential.community.resolve("community", "SNMP")?;
            let community = community_secret.expose_secret();
            tracing::debug!(
                ip = %ip,
                version = %credential.version,
                community_len = community.len(),
                "Creating SNMP community session"
            );

            // v1 and v2c differ only in the wire protocol version negotiated;
            // both authenticate with a community string.
            let create = async {
                if is_v1 {
                    AsyncSession::new_v1(&target, community.as_bytes(), 0).await
                } else {
                    AsyncSession::new_v2c(&target, community.as_bytes(), 0).await
                }
            };

            match timeout(SNMP_SESSION_TIMEOUT, create).await {
                Ok(Ok(session)) => {
                    tracing::debug!(ip = %ip, "SNMP session created successfully");
                    Ok(Box::new(session))
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
            let v3 = credential
                .v3
                .as_ref()
                .ok_or_else(|| anyhow!("SNMPv3 credential missing USM parameters"))?;
            let auth_pw = v3.auth_password.resolve("auth_password", "SNMPv3")?;
            let priv_pw = v3.priv_password.resolve("priv_password", "SNMPv3")?;

            // AuthPriv: both authentication and encryption. snmp2 0.5.0 only
            // transmits the default (empty) context, so context_name is ignored
            // on the wire here.
            let security = snmp2::v3::Security::new(
                v3.security_name.as_bytes(),
                auth_pw.expose_secret().as_bytes(),
            )
            .with_auth_protocol(v3.auth_protocol.into())
            .with_auth(snmp2::v3::Auth::AuthPriv {
                cipher: v3.priv_protocol.into(),
                privacy_password: priv_pw.expose_secret().as_bytes().to_vec(),
            });

            tracing::debug!(
                ip = %ip,
                security_name = %v3.security_name,
                "Creating SNMPv3 session"
            );

            let mut session = match timeout(
                SNMP_SESSION_TIMEOUT,
                AsyncSession::new_v3(&target, 0, security),
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
            // session learns the authoritative engine ID / boots / time.
            match timeout(SNMP_SESSION_TIMEOUT, session.init()).await {
                Ok(Ok(())) => {
                    tracing::debug!(ip = %ip, "SNMPv3 session initialized");
                    Ok(Box::new(session))
                }
                Ok(Err(e)) => Err(anyhow!(
                    "Failed SNMPv3 engine discovery / authentication to {}: {:?}",
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!("Timeout during SNMPv3 engine discovery to {}", ip)),
            }
        }
    }
}
