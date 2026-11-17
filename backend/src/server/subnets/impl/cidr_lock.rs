//! Which subnets a person may move to a new range, and why the rest are fixed.
//!
//! A range comes from one of two places. A person typed it, or Scanopy assumed it from an address
//! nothing else held: those are assertions and guesses, and editing them is how they get
//! corrected. Everything else was read from the network itself: a daemon's own interface, a
//! container runtime's bridge, a device's address table, or the seeded rows that hold every
//! public and remote address. Moving one of those by hand doesn't change the network, so the next
//! report brings the old range back as a second row beside the edited one.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;

/// Why a subnet's range can't be changed by hand. `None` on a subnet means it can.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CidrLock {
    /// The Internet or Remote Network row, which holds every address of its kind.
    #[schema(title = "Organizational")]
    Organizational,
    /// A container runtime's bridge network, reported by the runtime on its host.
    #[schema(title = "ContainerBridge")]
    ContainerBridge {
        /// The runtime service that owns the bridge.
        service_id: Uuid,
    },
    /// A network a daemon's host has an interface on. The daemon reports its range.
    #[schema(title = "DaemonInterface")]
    DaemonInterface {
        /// The daemon that reports it.
        daemon_id: Uuid,
    },
    /// A range discovery read from the network, such as a device's address table.
    #[schema(title = "Discovered")]
    Discovered,
}

impl Subnet {
    /// Why this subnet's range is fixed, given the daemon interfaced on it, if any.
    ///
    /// `interfaced_by` comes from the daemon side, which this module can't read. A daemon's
    /// interfaced subnet can be `Manual`: a person created the range before the daemon reported
    /// it, and the report matched the row instead of adding one. It is fixed all the same, because
    /// the daemon's next report after an edit wouldn't match it any more.
    pub fn cidr_lock(&self, interfaced_by: Option<Uuid>) -> Option<CidrLock> {
        if self.is_organizational_subnet() || self.base.source == EntitySource::System {
            return Some(CidrLock::Organizational);
        }
        if let Some(service_id) = self.base.virtualization_service_id {
            return Some(CidrLock::ContainerBridge { service_id });
        }
        if let Some(daemon_id) = interfaced_by {
            return Some(CidrLock::DaemonInterface { daemon_id });
        }
        match self.base.source {
            EntitySource::Manual | EntitySource::Inferred { .. } => None,
            // `Unknown` is a row from a newer server this one can't classify, so it is treated as
            // read from the network: refusing an edit is recoverable, a duplicate row isn't.
            EntitySource::Discovery
            | EntitySource::DiscoveryWithMatch { .. }
            | EntitySource::System
            | EntitySource::Unknown => Some(CidrLock::Discovered),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::InferredVia;
    use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};

    fn subnet(cidr: &str, source: EntitySource) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                AttributeSource::Manual,
            ),
            source,
            ..Default::default()
        })
    }

    /// Only the ranges a person typed or Scanopy guessed are editable, and a daemon reporting
    /// the range fixes it whoever created the row.
    #[test]
    fn only_typed_or_assumed_ranges_off_a_daemon_interface_are_editable() {
        let inferred = EntitySource::Inferred {
            via: InferredVia::ReportedAddress,
        };
        assert_eq!(
            subnet("10.0.50.0/24", EntitySource::Manual).cidr_lock(None),
            None
        );
        assert_eq!(
            subnet("10.0.50.0/24", inferred.clone()).cidr_lock(None),
            None
        );

        let daemon_id = Uuid::new_v4();
        assert_eq!(
            subnet("10.0.50.0/24", EntitySource::Manual).cidr_lock(Some(daemon_id)),
            Some(CidrLock::DaemonInterface { daemon_id })
        );
        assert!(
            subnet("10.0.50.0/24", EntitySource::Discovery)
                .cidr_lock(None)
                .is_some()
        );
    }

    /// The seeded rows and container bridges stay fixed even if a person created them, since
    /// the range is what makes them what they are.
    #[test]
    fn organizational_rows_and_bridges_are_fixed_whatever_their_source() {
        assert_eq!(
            subnet("0.0.0.0/0", EntitySource::Manual).cidr_lock(None),
            Some(CidrLock::Organizational)
        );
        let mut bridge = subnet("172.17.0.0/16", EntitySource::Manual);
        let service_id = Uuid::new_v4();
        bridge.base.virtualization_service_id = Some(service_id);
        assert_eq!(
            bridge.cidr_lock(None),
            Some(CidrLock::ContainerBridge { service_id })
        );
    }
}
