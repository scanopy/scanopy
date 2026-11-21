//! Why a person is creating a subnet: the first question of the create flow.
//!
//! The answer shapes what the flow asks next, and travels on the `subnet_created` event so the
//! two reasons a person creates a subnet can be told apart from each other and from the subnets
//! daemons create.
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::shared::{
    entities::EntityDiscriminants,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    EnumIter,
    IntoStaticStr,
    ToSchema,
)]
pub enum SubnetCreatePurpose {
    /// Find the hosts on the network: a routed scan from an existing daemon.
    Scan,
    /// Keep the range without scanning it: an address block, or a network no daemon reaches.
    Record,
}

impl HasId for SubnetCreatePurpose {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SubnetCreatePurpose {
    fn color(&self) -> Color {
        match self {
            SubnetCreatePurpose::Scan => EntityDiscriminants::Discovery.color(),
            SubnetCreatePurpose::Record => EntityDiscriminants::Subnet.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            SubnetCreatePurpose::Scan => Icon::Radar,
            SubnetCreatePurpose::Record => Icon::NotebookPen,
        }
    }
}

impl TypeMetadataProvider for SubnetCreatePurpose {
    fn name(&self) -> &'static str {
        match self {
            SubnetCreatePurpose::Scan => "Scan it",
            SubnetCreatePurpose::Record => "Record it without scanning",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            SubnetCreatePurpose::Scan => "Find the hosts on this subnet.",
            SubnetCreatePurpose::Record => {
                "Keep it as a range, for example an address block that holds other subnets, or a \
                 network no daemon reaches."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Operation};
    use crate::server::shared::events::types::EntityOperation;
    use crate::server::subnets::r#impl::base::Subnet;
    use uuid::Uuid;

    fn created_metadata(purpose: Option<SubnetCreatePurpose>) -> serde_json::Value {
        let scope = EntityScope::Site {
            site_id: Uuid::new_v4(),
            entity_id: Uuid::new_v4(),
            entity_type: Subnet::default().into(),
        };
        let flags = EntityEventFlags {
            subnet_create_purpose: purpose,
            ..Default::default()
        };
        serde_json::to_value(EntityOperation::Created.metadata(&scope, &flags)).unwrap()
    }

    /// A subnet the create flow made says why on its event; every other create says nothing, so
    /// the field also separates a person's subnets from a daemon's.
    #[test]
    fn the_create_event_carries_the_purpose_only_when_one_was_given() {
        assert_eq!(
            created_metadata(Some(SubnetCreatePurpose::Record))["purpose"],
            "Record"
        );
        assert!(created_metadata(None).get("purpose").is_none());
    }
}
