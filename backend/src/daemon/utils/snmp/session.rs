//! SNMP Session Management
//!
//! Functions for creating and managing SNMP sessions.

use anyhow::{Result, anyhow};
use snmp2::AsyncSession;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time::timeout;

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;
use crate::server::credentials::r#impl::types::SnmpVersion;

/// Default timeout for SNMP operations
pub const SNMP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for SNMP session creation (UDP socket setup)
pub const SNMP_SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for table walks (longer since they involve multiple requests)
pub const SNMP_WALK_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of varbinds to process in a single walk
pub const MAX_WALK_ENTRIES: usize = 10000;

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
        SnmpVersion::V2c => {
            let community_secret = credential.community.resolve("community", "SNMP")?;
            let community = community_secret.expose_secret();
            tracing::debug!(
                ip = %ip,
                community_len = community.len(),
                "Creating SNMPv2c session"
            );

            match timeout(
                SNMP_SESSION_TIMEOUT,
                AsyncSession::new_v2c(&target, community.as_bytes(), 0),
            )
            .await
            {
                Ok(Ok(session)) => {
                    tracing::debug!(ip = %ip, "SNMP session created successfully");
                    Ok(Box::new(session))
                }
                Ok(Err(e)) => Err(anyhow!(
                    "Failed to create SNMPv2c session to {}: {:?}",
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!(
                    "Timeout creating SNMPv2c session to {} ({}s)",
                    ip,
                    SNMP_SESSION_TIMEOUT.as_secs()
                )),
            }
        }
        SnmpVersion::V3 => {
            // V3 support would require additional auth/priv parameters
            Err(anyhow!("SNMPv3 not yet implemented"))
        }
    }
}
