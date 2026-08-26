//! Pre-generated tonic/prost bindings for gNMI (github.com/openconfig/gnmi, pinned v0.10.0).
//!
//! Generated once with `tonic-prost-build` 0.14 (`build_server(false)`) and committed, the same
//! way the project vendors snmp2: no protoc or build.rs required to build the workspace. To
//! regenerate: fetch proto/gnmi/gnmi.proto + proto/gnmi_ext/gnmi_ext.proto at the pinned tag,
//! run tonic_prost_build::configure().build_server(false).compile_protos(...), and replace the
//! two *_generated.rs files.
#![allow(clippy::all, rustdoc::all, deprecated)]

pub mod gnmi_ext {
    include!("gnmi_ext_generated.rs");
}
pub mod gnmi {
    include!("gnmi_generated.rs");
}
