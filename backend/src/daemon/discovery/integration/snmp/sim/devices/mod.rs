//! The devices themselves. One module per device.
//!
//! Each carries the defect it exists to catch in its `Purpose`, and its regression test lives
//! beside it — so a device and the guard on that device cannot drift apart.

use secrecy::SecretString;

use crate::server::credentials::r#impl::types::SecretValue;

use super::SimDevice;

pub mod ap_wireless_01;
pub mod firewall_01;
pub mod legacy_switch_01;
pub mod pc_windows_nic_filters;
pub mod printer_lobby;
pub mod router_gw_01;
pub mod secure_switch_01;
pub mod switch_access_01;
pub mod switch_aruba_01;
pub mod switch_cisco_01;
pub mod switch_core_01;
pub mod switch_dell_01;
pub mod switch_dlink_01;
pub mod switch_dlink_02;
pub mod switch_exos_01;
pub mod switch_fdb_only_01;
pub mod switch_flaky_01;
pub mod switch_hikvision_01;
pub mod switch_macport_01;
pub mod switch_mcast_rcv_01;
pub mod switch_mcast_src_01;
pub mod switch_mute_01;
pub mod switch_netgear_01;
pub mod switch_ocnos_01;
pub mod switch_offsite_01;
pub mod switch_omada_01;
pub mod switch_quietcol_01;
pub mod switch_segment_gw_01;
pub mod switch_shortports_01;
pub mod switch_slowbulk_01;
pub mod switch_stuck_01;
pub mod switch_tplink_01;
pub mod switch_unsorted_01;
pub mod switch_voss_01;

/// A seeded secret. The lab's credentials are published in `SNMP-TEST-ENV.md` and exist only to
/// be answered by a simulator, so they are inline rather than read from a file.
pub(super) fn inline(value: &str) -> SecretValue {
    SecretValue::Inline {
        value: SecretString::from(value.to_string()),
    }
}

/// Every device, in allocation order.
///
/// APPEND ONLY. `super::allocation::assign_addresses` gives each device its address purely from
/// its position here — position 0 gets the lab's first reserved address, position 1 the next, and
/// so on. Reordering or removing an entry silently renumbers every device after it; adding one is
/// always safe, and is the only change this list should ever see.
pub fn all() -> Vec<SimDevice> {
    vec![
        switch_core_01::device(),
        switch_access_01::device(),
        router_gw_01::device(),
        firewall_01::device(),
        printer_lobby::device(),
        ap_wireless_01::device(),
        legacy_switch_01::device(),
        secure_switch_01::device(),
        switch_exos_01::device(),
        switch_voss_01::device(),
        switch_netgear_01::device(),
        switch_aruba_01::device(),
        switch_omada_01::device(),
        switch_flaky_01::device(),
        switch_dlink_01::device(),
        switch_dlink_02::device(),
        pc_windows_nic_filters::device(),
        switch_tplink_01::device(),
        switch_unsorted_01::device(),
        switch_macport_01::device(),
        switch_mute_01::device(),
        switch_stuck_01::device(),
        switch_dell_01::device(),
        switch_cisco_01::device(),
        switch_slowbulk_01::device(),
        switch_shortports_01::device(),
        switch_offsite_01::device(),
        // GH #709: bridge FDB served, no LLDP remote table at all.
        switch_fdb_only_01::device(),
        // GH #701: shared L2 segment, each port hearing two neighbours.
        switch_mcast_rcv_01::device(),
        switch_mcast_src_01::device(),
        switch_segment_gw_01::device(),
        // GH #685: one neighbour column never answers, so the walk stops part way.
        switch_quietcol_01::device(),
        // GH #710: getbulk above ten repetitions answered with genErr.
        switch_hikvision_01::device(),
        // GH #688: LLDP-V2-MIB only, no classic LLDP tables.
        switch_ocnos_01::device(),
    ]
}
