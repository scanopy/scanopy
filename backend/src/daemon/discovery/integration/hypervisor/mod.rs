//! What every hypervisor integration does the same way.
//!
//! Proxmox VE and libvirt read their guests differently, but once read a guest is the same thing:
//! the NICs its config declares, the addresses something reports for it, and what a guest agent
//! says about it. The rules that turn that into hosts live here, so a guest under any hypervisor
//! gets the same handling: a Docker host's bridges kept off its addresses ([`guest`]), its macvlan
//! containers and virtual IPs as network identities ([`identities`]), its record linked to the
//! hypervisor's service ([`host`], [`sync`]), and a warning when the scan cannot reach it
//! ([`reach`]). Each integration only reads its hypervisor's wire format into these types.

pub mod guest;
pub mod host;
pub mod identities;
pub mod qemu_agent;
pub mod reach;
pub mod sync;
