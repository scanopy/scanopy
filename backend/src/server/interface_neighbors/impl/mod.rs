pub mod base;
mod child_storage; // ChildStorableEntity impls - parent relationship only
mod storage; // Storable/Snapshotable impls - full CRUD infrastructure, no Entity (see base.rs)
