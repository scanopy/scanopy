use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Wireguard;

impl ServiceDefinition for Wireguard {
    fn name(&self) -> &'static str {
        "WireGuard"
    }
    fn description(&self) -> &'static str {
        "WireGuard VPN"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::VPN
    }
    /// Not discoverable by probing, deliberately on WireGuard's part.
    ///
    /// A handshake initiation is authenticated against the responder's static public key, and an
    /// initiation that does not verify is dropped without a reply. Silence to unauthenticated
    /// traffic is a stated design goal, so there is no packet we can send that distinguishes a
    /// WireGuard endpoint from a closed port.
    ///
    /// This used to be `Port(Wireguard)` on 51820/udp, which could never match either: nothing
    /// reports a UDP port open without a probe behind it, so the definition sat in the registry
    /// detecting nothing while looking like coverage. `None` says the same thing honestly and
    /// keeps the definition assignable by hand, which is the only way a WireGuard endpoint gets
    /// recorded today.
    ///
    /// What would work is a `wg`-prefixed interface in the SNMP `ifTable`, which is evidence of a
    /// different type from anything this pattern language expresses.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
    fn is_generic(&self) -> bool {
        true
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/wireguard.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Wireguard>));
