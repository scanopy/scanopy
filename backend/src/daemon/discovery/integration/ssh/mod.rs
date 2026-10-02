//! SSH: run the credential's script on the host and apply the JSON it prints.
//!
//! Probe completes a handshake and reads the host key, sending no credentials. Execute decides
//! whether that key is trusted (the credential's required fingerprint, else the key pinned on
//! first use), authenticates to that exact key, runs the script, and applies its output. The
//! contract the output follows lives in `server::credentials::impl::types::ssh_script`.

mod apply;
mod client;

use std::time::Duration;

use async_trait::async_trait;

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants, SshQueryCredential,
};
use crate::server::credentials::r#impl::types::ssh_script::{
    MAX_SCRIPT_OUTPUT_BYTES, SshScriptOutcome, SshScriptOutput, SshScriptRun,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use client::SessionError;

/// The longest a credential may let a script run, whatever it asks for.
const MAX_SCRIPT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub struct SshIntegration;

/// The host key the probe saw.
struct SshProbeHandle {
    fingerprint: String,
}

fn credential(c: &CredentialQueryPayload) -> Option<&SshQueryCredential> {
    match c {
        CredentialQueryPayload::Ssh(s) => Some(s),
        _ => None,
    }
}

fn script_timeout(cred: &SshQueryCredential) -> Duration {
    Duration::from_secs(u64::from(cred.timeout_seconds.max(1))).min(MAX_SCRIPT_TIMEOUT)
}

#[async_trait]
impl DiscoveryIntegration for SshIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Ssh
    }

    fn estimated_seconds(&self) -> u32 {
        5
    }

    /// A script reports the interfaces its author chose to print. Narrower than an ifTable walk,
    /// so SNMP's row wins where both describe one interface.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::PhysicalPortsOnly
    }

    /// Backstop only; the script's own timeout is enforced inside `execute`.
    fn timeout(&self) -> Duration {
        MAX_SCRIPT_TIMEOUT + client::CONNECT_TIMEOUT * 2
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        self::credential(credential)
            .map(|c| vec![PortType::new_tcp(c.port)])
            .unwrap_or_default()
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let cred = credential(ctx.credential)
            .ok_or_else(|| ProbeFailure::malformed("Expected SSH credential"))?;
        let fingerprint = client::read_host_key(ctx.ip, cred.port)
            .await
            .map_err(|e| ProbeFailure::not_this_service(format!("SSH handshake failed: {e}")))?;
        Ok(ProbeSuccess {
            client_probe: Some(ClientProbe::Ssh),
            ports: vec![PortType::new_tcp(cred.port)],
            handle: Some(Box::new(SshProbeHandle { fingerprint })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        _checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        let cred = credential(ctx.credential)
            .ok_or_else(|| IntegrationFailure::collection_failed("Expected SSH credential"))?;
        let observed = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<SshProbeHandle>())
            .map(|h| h.fingerprint.clone())
            .ok_or_else(|| anyhow::anyhow!("SSH execute called without SshProbeHandle"))?;

        let mut run = SshScriptRun {
            ip: ctx.ip,
            outcome: SshScriptOutcome::Applied,
            exit_code: None,
            applied_keys: Vec::new(),
            rejected_keys: Vec::new(),
            detail: None,
            duration_ms: 0,
        };
        let result = self.run(ctx, cred, &observed, host_data, &mut run).await;
        if let Some(id) = ctx.credential_id {
            ctx.ops.record_ssh_script_run(id, run).await;
        }
        result
    }
}

impl SshIntegration {
    /// The body of `execute`, filling in `run` as it goes so the caller records it on every path.
    async fn run(
        &self,
        ctx: &IntegrationContext<'_>,
        cred: &SshQueryCredential,
        observed: &str,
        host_data: &mut HostData,
        run: &mut SshScriptRun,
    ) -> Result<Completeness, IntegrationFailure> {
        let endpoint = format!("{}:{}", ctx.ip, cred.port);
        let store = &ctx.ops.config_store;
        let pinned = store.ssh_known_host_key(&endpoint).await;
        let pin_after_login = match host_key_verdict(
            observed,
            cred.host_key_fingerprint.as_deref(),
            pinned.as_deref(),
        ) {
            KeyVerdict::Trust { pin } => pin,
            KeyVerdict::Mismatch { trusted } => {
                run.outcome = SshScriptOutcome::HostKeyMismatch;
                let message = format!(
                    "the host presented key {observed}, not the trusted {trusted}; the script was not run. If the host was reinstalled, remove {endpoint} from ssh_known_host_keys in the daemon config"
                );
                run.detail = Some(message.clone());
                return Err(IntegrationFailure::with_outcome(
                    AttemptOutcome::Rejected,
                    message,
                ));
            }
        };

        // A script file on the daemon's own disk that cannot be read is our configuration, not the
        // host: report it before connecting.
        let command = match cred.script.command(cred.target_os) {
            Ok(c) => c,
            Err(e) => {
                run.outcome = SshScriptOutcome::ConnectionFailed;
                run.detail = Some(e.to_string());
                return Err(IntegrationFailure::with_outcome(
                    AttemptOutcome::Malformed,
                    e.to_string(),
                ));
            }
        };
        let script = client::run_script(
            ctx.ip,
            cred.port,
            observed,
            &cred.username,
            &cred.auth,
            &command,
            script_timeout(cred),
        )
        .await;
        let script = match script {
            Ok(s) => s,
            Err(e) => {
                let (outcome, attempt) = match &e {
                    SessionError::HostKeyMismatch { .. } => {
                        (SshScriptOutcome::HostKeyMismatch, AttemptOutcome::Rejected)
                    }
                    SessionError::AuthenticationFailed(_) => (
                        SshScriptOutcome::AuthenticationFailed,
                        AttemptOutcome::Rejected,
                    ),
                    SessionError::Malformed(_) => (
                        SshScriptOutcome::ConnectionFailed,
                        AttemptOutcome::Malformed,
                    ),
                    SessionError::Unreachable(_) => (
                        SshScriptOutcome::ConnectionFailed,
                        AttemptOutcome::Unreachable,
                    ),
                };
                run.outcome = outcome;
                run.detail = Some(e.to_string());
                return Err(IntegrationFailure::with_outcome(attempt, e.to_string()));
            }
        };

        if pin_after_login && let Err(e) = store.pin_ssh_host_key(&endpoint, observed).await {
            tracing::warn!(%endpoint, error = %e, "Could not pin the SSH host key");
        }

        run.exit_code = script.exit_code;
        run.duration_ms = u64::try_from(script.elapsed.as_millis()).unwrap_or(u64::MAX);
        let stderr = String::from_utf8_lossy(&script.stderr_tail)
            .trim()
            .to_string();
        run.detail = (!stderr.is_empty()).then_some(stderr.clone());

        let failure = |run: &mut SshScriptRun, outcome, message: String| {
            run.outcome = outcome;
            IntegrationFailure::collection_failed(message)
        };
        if script.timed_out {
            return Err(failure(
                run,
                SshScriptOutcome::TimedOut,
                format!("the script did not finish within {}s", cred.timeout_seconds),
            ));
        }
        if script.stdout_overflowed {
            return Err(failure(
                run,
                SshScriptOutcome::OutputTooLarge,
                format!(
                    "the script printed more than {} KiB; nothing was applied",
                    MAX_SCRIPT_OUTPUT_BYTES / 1024
                ),
            ));
        }
        match script.exit_code {
            Some(0) => {}
            code => {
                let code = code.map_or("no status".to_string(), |c| c.to_string());
                let tail = if stderr.is_empty() {
                    String::new()
                } else {
                    format!(": {stderr}")
                };
                return Err(failure(
                    run,
                    SshScriptOutcome::NonZeroExit,
                    format!("the script exited with {code}{tail}"),
                ));
            }
        }

        let stdout = String::from_utf8_lossy(&script.stdout);
        let output = match SshScriptOutput::parse(&stdout) {
            Ok(o) => o,
            Err(e) => {
                let message = format!("the script's output is not a valid document: {e}");
                run.detail = Some(message.clone());
                return Err(failure(run, SshScriptOutcome::InvalidOutput, message));
            }
        };
        run.rejected_keys = output.unknown_keys();
        let (applied, invalid) = apply::apply(output, host_data, ctx.interface_source, ctx.host_id);
        run.applied_keys = applied;
        run.rejected_keys.extend(invalid);
        run.outcome = SshScriptOutcome::Applied;
        tracing::info!(
            ip = %ctx.ip,
            applied = run.applied_keys.len(),
            rejected = run.rejected_keys.len(),
            "SSH script applied"
        );
        Ok(Completeness::Complete)
    }
}

/// Whether to run the script on a host presenting `observed`.
#[derive(Debug, PartialEq, Eq)]
enum KeyVerdict {
    /// Go ahead. `pin` when this is the first key seen at the address, to be pinned once the
    /// login succeeds; pinning before it would trust whatever answered, login or not.
    Trust {
        pin: bool,
    },
    Mismatch {
        trusted: String,
    },
}

/// The credential's required fingerprint wins; without one, the key pinned on first use; without
/// either, trust this key and pin it.
fn host_key_verdict(observed: &str, required: Option<&str>, pinned: Option<&str>) -> KeyVerdict {
    match required.or(pinned) {
        None => KeyVerdict::Trust { pin: true },
        Some(trusted) if trusted == observed => KeyVerdict::Trust { pin: false },
        Some(trusted) => KeyVerdict::Mismatch {
            trusted: trusted.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_verdict_pins_first_use_and_refuses_a_change() {
        assert_eq!(
            host_key_verdict("A", None, None),
            KeyVerdict::Trust { pin: true }
        );
        assert_eq!(
            host_key_verdict("A", None, Some("A")),
            KeyVerdict::Trust { pin: false }
        );
        assert_eq!(
            host_key_verdict("B", None, Some("A")),
            KeyVerdict::Mismatch {
                trusted: "A".into()
            }
        );
        // A required fingerprint overrides a stale pin, in both directions.
        assert_eq!(
            host_key_verdict("B", Some("B"), Some("A")),
            KeyVerdict::Trust { pin: false }
        );
        assert_eq!(
            host_key_verdict("A", Some("B"), Some("A")),
            KeyVerdict::Mismatch {
                trusted: "B".into()
            }
        );
    }
}
