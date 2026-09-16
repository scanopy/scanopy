//! GH #701: 1:N neighbour evidence and resolution, replacing `interfaces`' old single-neighbour
//! columns. See `impl::base` for the three tables' shapes and why they're `Storable` rather than
//! `Entity`, and `service` for the bespoke read/write paths.
pub mod r#impl;
pub mod service;
