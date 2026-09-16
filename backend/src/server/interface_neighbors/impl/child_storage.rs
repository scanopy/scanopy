use uuid::Uuid;

use crate::server::{
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborHost, InterfaceNeighborInterface,
    },
    shared::storage::child::ChildStorableEntity,
};

// All three tables are children of `interfaces` — parent relationship only, no full CRUD
// infrastructure (see `storage.rs` for that, and the module docs in `impl/base.rs` for why these
// three are `Storable` rather than `Entity`).

impl ChildStorableEntity for InterfaceNeighborCandidate {
    fn parent_column() -> &'static str {
        "interface_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.interface_id
    }
}

impl ChildStorableEntity for InterfaceNeighborInterface {
    fn parent_column() -> &'static str {
        "interface_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.interface_id
    }
}

impl ChildStorableEntity for InterfaceNeighborHost {
    fn parent_column() -> &'static str {
        "interface_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.interface_id
    }
}
