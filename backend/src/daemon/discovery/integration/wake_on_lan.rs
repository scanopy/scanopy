//! Wake-on-LAN: record which of the hosts woken before the sweep the scan then found.
//!
//! The packets go out before the sweep (`discovery::wake_on_lan`), since the sweep only finds
//! hosts that are already up. Whether a host woke is then decided by the same liveness checks as
//! every other host: the deep scan, and so this probe, only runs for an address that answered
//! them. Reaching the probe is the proof, so it always succeeds, and `execute` records the host as
//! woken. A target the scan never found keeps the `woke: false` the wake step recorded and gets
//! the standard not-responding issue from `unanswered_credential_targets`.

use async_trait::async_trait;

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::credentials::r#impl::run_results::WakeOnLanResult;

pub struct WakeOnLanIntegration;

#[async_trait]
impl DiscoveryIntegration for WakeOnLanIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::WakeOnLan
    }

    fn estimated_seconds(&self) -> u32 {
        0
    }

    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::NoInterfaces
    }

    /// No client probe: a NIC that took a magic packet has no service to identify, and the
    /// associated service's `Pattern::None` lets execute run without a match.
    async fn probe(&self, _ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        Ok(ProbeSuccess {
            client_probe: None,
            ports: Vec::new(),
            handle: None,
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        _host_data: &mut HostData,
        _checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        if let Some(id) = ctx.credential_id {
            ctx.ops
                .record_wake_on_lan(
                    id,
                    WakeOnLanResult {
                        ip: ctx.ip,
                        woke: true,
                    },
                )
                .await;
        }
        Ok(Completeness::Complete)
    }
}
