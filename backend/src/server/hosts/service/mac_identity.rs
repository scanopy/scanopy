//! MAC addresses as host identity: what a MAC is worth, and what that buys.
//!
//! Identity is address-based (`select_matching_host`), and this is the tier consulted when
//! addresses have nothing to work with — a device with no IP at all, or one whose address moved
//! between scans. Two orthogonal gates decide what a given MAC may do, and both have to pass:
//!
//! - **Value quality**, which is a property of the address itself. A locally administered MAC is
//!   one a device chose and may choose again, so anchoring identity to it duplicates the host on
//!   every rotation.
//! - **Provenance**, the §7 rung it arrived on. Our own ARP sweep, a device's `ifPhysAddress`, a
//!   *router's* ARP cache and a controller's inventory are four very different claims that were
//!   indistinguishable downstream until `mac_address_source` existed.
//!
//! **Minting requires a strong value and `Queried`-or-better provenance; matching an existing host
//! accepts weaker provenance.** The asymmetry is the whole design: attaching evidence to a host
//! that already exists is recoverable, conjuring one is not. A weak *value* minted gives unbounded
//! duplication; a weak *provenance* minted gives a ghost — a record for a device nothing has ever
//! contacted, on a third party's say-so, which is what GH #668 describes from the LLDP side.

use std::collections::HashSet;

use mac_address::MacAddress;
use uuid::Uuid;

use super::is_virtual_router_mac;
use crate::server::ip_addresses::r#impl::base::MacEvidence;
use crate::server::shared::oui;

/// What a MAC is worth as an identity anchor, judged on the value alone.
///
/// Orthogonal to provenance: a strong value can still arrive on a rung too weak to mint from, and
/// that combination is exactly the one the two gates below tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacQuality {
    /// A burned-in, vendor-assigned unicast address. Identifies one NIC and keeps identifying it.
    Strong,
    /// Locally administered, or no OUI that resolves. Real enough to break a tie inside one
    /// payload — which is all `ip_addresses_match` ever uses it for — and not stable enough to
    /// carry identity between scans.
    Weak,
    /// Not a device address at all: a shared virtual-router MAC, or a group address.
    Excluded,
}

/// Grade a MAC on its value.
///
/// Order matters, and the excluded tests must run first. `00:00:5e:00:01:0a` is a VRRP address
/// with the U/L bit clear whose OUI resolves to IANA, and `01:00:5e:…` is IPv4 multicast with the
/// same two properties — so a classifier that reached for the OUI first would grade both `Strong`
/// and let a redundancy group, or a multicast destination, mint and merge hosts.
pub(crate) fn classify(mac: &MacAddress) -> MacQuality {
    let bytes = mac.bytes();

    // I/G bit: a group address is a destination, never a device. The virtual-router prefixes are
    // the same statement made specifically — several physical routers deliberately wearing one
    // address, which is the merge `is_virtual_router_mac` was written to prevent.
    if bytes[0] & 0x01 != 0 || is_virtual_router_mac(mac) {
        return MacQuality::Excluded;
    }

    // U/L bit: locally administered. Nobody assigned it, so nothing stops the device picking a
    // different one after a reboot — and a host keyed on a rotating address becomes a new host
    // every rotation. An OUI that resolves is the other half of the same question: an address from
    // a registered block was allocated to hardware by somebody.
    if bytes[0] & 0x02 != 0 || oui::lookup_by_mac(&mac.to_string()).is_none() {
        return MacQuality::Weak;
    }

    MacQuality::Strong
}

/// Whether this MAC may anchor a match against a host that already exists.
///
/// Provenance is not consulted. A weak rung is a weak claim about *where the address came from*,
/// and the cost of believing it here is attaching evidence to the wrong existing host — visible,
/// and recoverable by the next scan that reads the device directly.
pub(crate) fn may_match(evidence: &MacEvidence) -> bool {
    classify(&evidence.value().0) == MacQuality::Strong
}

/// Whether this MAC may conjure a host that does not exist.
///
/// Both gates. A minted host is a row an operator has to disprove: it counts against the plan
/// limit, appears in inventory, and nothing later in the pipeline knows it was a guess.
pub(crate) fn may_mint(evidence: &MacEvidence) -> bool {
    may_match(evidence) && evidence.source().method().binds_claim_to_subject()
}

/// The one host this payload's MACs identify, or `None`.
///
/// Pure, and consulted only once the address and chassis-id tiers in `select_matching_host` have
/// failed — which is what keeps MAC off the fleet-wide identity path. A device with an address
/// matches on IP and subnet long before it reaches here; what arrives is a device with no address,
/// or one whose address moved.
///
/// `candidates` is `(host_id, mac)` for the live rows a targeted lookup returned, from both
/// `ip_addresses` and `interfaces` — a MAC-identified device may carry its address on either.
///
/// Resolves on a **single** host only, for the reason `match_by_chassis_id` gives: two hosts
/// wearing one identifier are a duplicate this cannot choose between, and picking either merges a
/// scan into an arbitrary one of them.
pub(crate) fn select_matching_host_by_mac(
    incoming: &[MacEvidence],
    candidates: &[(Uuid, MacAddress)],
) -> Option<Uuid> {
    let anchors: HashSet<MacAddress> = incoming
        .iter()
        .filter(|e| may_match(e))
        .map(|e| e.value().0)
        .collect();
    if anchors.is_empty() {
        return None;
    }

    let matched: HashSet<Uuid> = candidates
        .iter()
        .filter(|(_, mac)| anchors.contains(mac))
        .map(|(host_id, _)| *host_id)
        .collect();

    let mut found = matched.into_iter();
    let first = found.next()?;
    if found.next().is_some() {
        tracing::debug!(
            "A MAC in this payload names more than one host; leaving it to the address tiers"
        );
        return None;
    }
    tracing::debug!(
        existing_host_id = %first,
        "Found matching host via MAC identity"
    );
    Some(first)
}

/// Whether this payload may become a host that does not exist yet.
///
/// `true` for everything carrying an address or a chassis id, unconditionally. Those are the
/// identities every path that mints today already carries — the sweep, the controller, the LLDP
/// far end — and this gate exists to admit a *new* class of payload, not to re-adjudicate them.
///
/// A payload with none of those is identified by its MAC alone, and then both gates apply.
pub(crate) fn identity_permits_minting(
    host: &crate::server::hosts::r#impl::base::Host,
    ip_addresses: &[crate::server::ip_addresses::r#impl::base::IPAddress],
    interfaces: &[crate::server::interfaces::r#impl::base::Interface],
) -> bool {
    if !ip_addresses.is_empty() || host.base.chassis_id.is_some() {
        return true;
    }
    payload_macs(ip_addresses, interfaces).iter().any(may_mint)
}

/// Every MAC this payload offers as an identity, whichever child row carries it.
///
/// A device reached by an address puts its MAC on an `ip_addresses` row; one known only at the
/// link layer puts it on an interface. Both are the same claim about the same NIC.
pub(crate) fn payload_macs<'a>(
    ip_addresses: &'a [crate::server::ip_addresses::r#impl::base::IPAddress],
    interfaces: &'a [crate::server::interfaces::r#impl::base::Interface],
) -> Vec<MacEvidence> {
    ip_addresses
        .iter()
        .filter_map(|i| i.base.mac_address.clone())
        .chain(interfaces.iter().filter_map(|i| i.base.mac_address.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::MacEvidenceValue;
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::AttributeSource;

    /// Dell, and a real assignment in `assets/oui.csv`: U/L bit clear, OUI resolves.
    fn burned_in() -> MacAddress {
        MacAddress::new([0xB0, 0x83, 0xFE, 0x11, 0x22, 0x33])
    }

    fn evidence(mac: MacAddress, source: AttributeSource) -> MacEvidence {
        MacEvidence::new(MacEvidenceValue(mac), source)
    }

    /// The address we solicited ourselves is the case both gates are meant to admit.
    #[test]
    fn a_burned_in_mac_we_arped_for_ourselves_both_matches_and_mints() {
        let e = evidence(burned_in(), AttributeSource::ArpReply);
        assert!(may_match(&e));
        assert!(may_mint(&e), "an ARP reply we solicited is Queried");
    }

    /// The asymmetry, on one value: the same NIC, heard about from a router's ARP cache.
    /// Good enough to attach evidence to a host we have. Not good enough to invent one.
    #[test]
    fn the_same_mac_reported_by_a_third_party_matches_but_never_mints() {
        let e = evidence(burned_in(), AttributeSource::ForwardingTable);
        assert!(
            may_match(&e),
            "a weaker rung still identifies a host we hold"
        );
        assert!(
            !may_mint(&e),
            "minting on a third party's say-so is how ghost hosts get created"
        );
    }

    /// A neighbour's advertisement is Announced — the right speaker, no proof it spoke.
    #[test]
    fn an_announced_mac_matches_but_never_mints() {
        let e = evidence(burned_in(), AttributeSource::LldpChassisId);
        assert!(may_match(&e));
        assert!(!may_mint(&e));
    }

    /// A device reporting its own `ifPhysAddress` over a session we opened to it.
    #[test]
    fn a_mac_the_device_reported_over_snmp_mints() {
        let e = evidence(burned_in(), AttributeSource::Probe(ClientProbe::Snmp));
        assert!(may_mint(&e));
    }

    /// The acceptance bar for the PROFINET DCP item: a device with no IP address reaches a
    /// `Host` at a provenance rung that permits minting one — DCP's directed identify exchange is
    /// `Native`, which `binds_claim_to_subject()` treats the same as `Queried`/`Manual`.
    #[test]
    fn a_mac_identified_over_profinet_dcp_mints() {
        let e = evidence(burned_in(), AttributeSource::ProfinetDcp);
        assert!(may_mint(&e));
    }

    /// A row written before the provenance column existed claims nothing, so it may not mint —
    /// but it still names the host it is already attached to.
    #[test]
    fn an_unattributed_mac_matches_but_never_mints() {
        let e = evidence(burned_in(), AttributeSource::Unspecified);
        assert!(may_match(&e));
        assert!(!may_mint(&e));
    }

    /// A device that picked its own address can pick another one. Neither gate opens, however
    /// strong the provenance — the failure a weak value causes is a new host per rotation, and no
    /// amount of certainty about *where we heard it* prevents that.
    #[test]
    fn a_locally_administered_mac_neither_matches_nor_mints() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        assert_eq!(classify(&mac), MacQuality::Weak);
        let e = evidence(mac, AttributeSource::ArpReply);
        assert!(!may_match(&e));
        assert!(!may_mint(&e));
    }

    /// Unicast and globally-scoped by its bits, but from no block anyone registered.
    #[test]
    fn a_mac_with_no_registered_oui_neither_matches_nor_mints() {
        let mac = MacAddress::new([0x0C, 0x00, 0x01, 0x11, 0x22, 0x33]);
        assert_eq!(
            classify(&mac),
            MacQuality::Weak,
            "if this block is ever registered, pick another unassigned prefix"
        );
        let e = evidence(mac, AttributeSource::ArpReply);
        assert!(!may_match(&e));
    }

    /// The two addresses the OUI test would otherwise wave through: both have the U/L bit clear
    /// and both resolve to IANA, so ordering inside `classify` is the only thing stopping them.
    #[test]
    fn shared_and_group_addresses_are_excluded_despite_resolving_to_an_oui() {
        for (label, mac) in [
            (
                "VRRP VRID 10",
                MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x01, 0x0a]),
            ),
            (
                "IPv4 multicast",
                MacAddress::new([0x01, 0x00, 0x5e, 0x01, 0x02, 0x03]),
            ),
            ("broadcast", MacAddress::new([0xff; 6])),
        ] {
            assert_eq!(classify(&mac), MacQuality::Excluded, "{label}");
            let e = evidence(mac, AttributeSource::ArpReply);
            assert!(!may_match(&e), "{label} must never anchor a host");
            assert!(!may_mint(&e), "{label} must never mint a host");
        }
    }

    #[test]
    fn a_strong_mac_names_the_host_that_carries_it() {
        let host = Uuid::new_v4();
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(burned_in(), AttributeSource::ForwardingTable)],
                &[
                    (host, burned_in()),
                    (
                        Uuid::new_v4(),
                        MacAddress::new([0xB0, 0x83, 0xFE, 0x99, 0x99, 0x99])
                    )
                ],
            ),
            Some(host)
        );
    }

    /// Merge behaviour across sources, order-independent: a PROFINET DCP submission resolves onto
    /// a host an earlier ARP submission already established (an `IPAddress` row carrying the same
    /// MAC), and the reverse — an ARP submission resolves onto a host DCP already established (an
    /// `Interface` row). `select_matching_host_by_mac` draws candidates from both entity types and
    /// is blind to which discovery method produced either side, so which one ran first cannot
    /// change the outcome.
    #[test]
    fn a_dcp_submission_and_an_arp_submission_resolve_to_one_host_regardless_of_order() {
        let host = Uuid::new_v4();

        // DCP arrives after ARP already put the MAC on an `IPAddress` row.
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(burned_in(), AttributeSource::ProfinetDcp)],
                &[(host, burned_in())], // stands in for an existing ip_addresses row
            ),
            Some(host),
            "a DCP submission must resolve onto a host ARP already established"
        );

        // ARP arrives after DCP already put the MAC on an `Interface` row.
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(burned_in(), AttributeSource::ArpReply)],
                &[(host, burned_in())], // stands in for an existing interfaces row
            ),
            Some(host),
            "an ARP submission must resolve onto a host DCP already established"
        );
    }

    /// Several rows of one host carrying the address — a NIC and the IP bound to it — is one
    /// answer, not an ambiguity.
    #[test]
    fn one_host_holding_the_mac_on_two_rows_still_resolves() {
        let host = Uuid::new_v4();
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(burned_in(), AttributeSource::ArpReply)],
                &[(host, burned_in()), (host, burned_in())],
            ),
            Some(host)
        );
    }

    /// Two hosts wearing one address is a duplicate this cannot choose between; picking either
    /// would merge the scan into an arbitrary one of them.
    #[test]
    fn a_mac_held_by_two_hosts_matches_neither() {
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(burned_in(), AttributeSource::ArpReply)],
                &[(Uuid::new_v4(), burned_in()), (Uuid::new_v4(), burned_in())],
            ),
            None
        );
    }

    /// The gate is what selects the anchors, so a payload of weak addresses reaches no candidate
    /// even when a host is sitting there carrying the very same value.
    #[test]
    fn a_weak_mac_does_not_reach_a_candidate_that_carries_it() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);
        assert_eq!(
            select_matching_host_by_mac(
                &[evidence(mac, AttributeSource::ArpReply)],
                &[(Uuid::new_v4(), mac)],
            ),
            None
        );
    }
}
