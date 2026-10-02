//! The contract between an SSH credential's script and the host record it fills.
//!
//! A script prints one JSON object. Its keys are [`SshScriptField`]s, and each one names an
//! existing host or interface field; a script cannot invent fields. Three things keep the contract
//! in step with the host model:
//!
//! - [`host_coverage`] and [`interface_coverage`] destructure `HostBase` and `InterfaceBase` with
//!   no `..`, generated from the same tokens as the coverage list. A field added, renamed or
//!   removed on either struct fails the build here until someone states whether a script may
//!   write it.
//! - The daemon's apply step destructures [`SshScriptOutput`] exhaustively, so a key parsed but
//!   never applied fails the build there.
//! - `every_field_parses_applies_and_lands` (daemon side) builds a document from every
//!   variant's example and asserts each one reaches the host with `SshScript` attribution.
//!
//! [`ssh_script_fields`] is the documentation view, emitted as `ssh-script-fields.json` and synced
//! to the website, so the docs table cannot drift from what the parser accepts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::hosts::r#impl::base::HostBase;
use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, InterfaceBase};

/// Largest stdout the daemon reads from a script. Past this the run is reported as a failure and
/// nothing is applied, rather than parsing a document that was cut off.
pub const MAX_SCRIPT_OUTPUT_BYTES: usize = 64 * 1024;

/// One key a script may print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, IntoStaticStr)]
pub enum SshScriptField {
    Hostname,
    SysName,
    SysDescr,
    SysObjectId,
    SysLocation,
    SysContact,
    ChassisId,
    Manufacturer,
    Model,
    SerialNumber,
    FirmwareRevision,
    SoftwareRevision,
    Os,
    ManagementUrl,
    InterfaceName,
    InterfaceDescr,
    InterfaceAlias,
    InterfaceMac,
    InterfaceSpeedBps,
    InterfaceAdminStatus,
    InterfaceOperStatus,
}

/// Where a key sits in the script's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SshScriptScope {
    /// A top-level key.
    Host,
    /// A key of an object in the top-level `interfaces` array.
    Interface,
}

impl SshScriptField {
    pub fn scope(self) -> SshScriptScope {
        match self {
            Self::Hostname
            | Self::SysName
            | Self::SysDescr
            | Self::SysObjectId
            | Self::SysLocation
            | Self::SysContact
            | Self::ChassisId
            | Self::Manufacturer
            | Self::Model
            | Self::SerialNumber
            | Self::FirmwareRevision
            | Self::SoftwareRevision
            | Self::Os
            | Self::ManagementUrl => SshScriptScope::Host,
            Self::InterfaceName
            | Self::InterfaceDescr
            | Self::InterfaceAlias
            | Self::InterfaceMac
            | Self::InterfaceSpeedBps
            | Self::InterfaceAdminStatus
            | Self::InterfaceOperStatus => SshScriptScope::Interface,
        }
    }

    /// The JSON key, within its scope.
    pub fn key(self) -> &'static str {
        match self {
            Self::Hostname => "hostname",
            Self::SysName => "sys_name",
            Self::SysDescr => "sys_descr",
            Self::SysObjectId => "sys_object_id",
            Self::SysLocation => "sys_location",
            Self::SysContact => "sys_contact",
            Self::ChassisId => "chassis_id",
            Self::Manufacturer => "manufacturer",
            Self::Model => "model",
            Self::SerialNumber => "serial_number",
            Self::FirmwareRevision => "firmware_revision",
            Self::SoftwareRevision => "software_revision",
            Self::Os => "os",
            Self::ManagementUrl => "management_url",
            Self::InterfaceName => "name",
            Self::InterfaceDescr => "descr",
            Self::InterfaceAlias => "alias",
            Self::InterfaceMac => "mac",
            Self::InterfaceSpeedBps => "speed_bps",
            Self::InterfaceAdminStatus => "admin_status",
            Self::InterfaceOperStatus => "oper_status",
        }
    }

    /// The key as the docs and the run summary show it: `interfaces[].mac` for an interface key.
    pub fn path(self) -> String {
        match self.scope() {
            SshScriptScope::Host => self.key().to_string(),
            SshScriptScope::Interface => format!("interfaces[].{}", self.key()),
        }
    }

    pub fn value_type(self) -> &'static str {
        match self {
            Self::InterfaceSpeedBps => "integer",
            Self::InterfaceAdminStatus | Self::InterfaceOperStatus => "string (enum)",
            Self::Os => "object",
            _ => "string",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Hostname => "The host's own hostname.",
            Self::SysName => "Administrative name, as SNMP sysName reports it.",
            Self::SysDescr => "Free-text system description, typically OS and version.",
            Self::SysObjectId => "Vendor object identifier, as SNMP sysObjectID reports it.",
            Self::SysLocation => "Physical location.",
            Self::SysContact => "Contact person or team.",
            Self::ChassisId => "Chassis identifier, usually the base MAC address.",
            Self::Manufacturer => "Hardware manufacturer.",
            Self::Model => "Hardware model.",
            Self::SerialNumber => "Hardware serial number.",
            Self::FirmwareRevision => "Firmware or BIOS version.",
            Self::SoftwareRevision => {
                "Software version, as ENTITY-MIB entPhysicalSoftwareRev reports it."
            }
            Self::Os => {
                "The operating system: an object with family (required, such as Linux, Windows or MacOs) and optional name, version, edition, codename and kernel_version."
            }
            Self::ManagementUrl => "URL of the host's management interface.",
            Self::InterfaceName => {
                "Interface name. Interfaces are matched to ones discovery already holds by name, then by MAC."
            }
            Self::InterfaceDescr => "Interface description, as SNMP ifDescr reports it.",
            Self::InterfaceAlias => {
                "Operator-assigned interface label, as SNMP ifAlias reports it."
            }
            Self::InterfaceMac => "Interface MAC address.",
            Self::InterfaceSpeedBps => "Link speed in bits per second.",
            Self::InterfaceAdminStatus => "Configured state: Up, Down or Testing.",
            Self::InterfaceOperStatus => {
                "Operational state: Up, Down, Testing, Unknown, Dormant, NotPresent or LowerLayerDown."
            }
        }
    }

    /// A valid value, as JSON. Also what the round-trip test feeds the parser, so every example
    /// in the docs is one the parser accepts.
    pub fn example(self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Hostname => json!("nas-01"),
            Self::SysName => json!("nas-01.example.lan"),
            Self::SysDescr => json!("Debian GNU/Linux 12 (bookworm) 6.1.0-18-amd64"),
            Self::SysObjectId => json!("1.3.6.1.4.1.8072.3.2.10"),
            Self::SysLocation => json!("Rack 2, shelf 3"),
            Self::SysContact => json!("ops@example.com"),
            Self::ChassisId => json!("3c:ec:ef:12:34:56"),
            Self::Manufacturer => json!("Supermicro"),
            Self::Model => json!("X11SCL-F"),
            Self::SerialNumber => json!("ZM19AS012345"),
            Self::FirmwareRevision => json!("2.1"),
            Self::SoftwareRevision => json!("4.2.1"),
            Self::Os => json!({
                "family": "Linux",
                "name": "Debian GNU/Linux",
                "version": "12",
                "codename": "bookworm",
                "kernel_version": "6.1.0-18-amd64"
            }),
            Self::ManagementUrl => json!("https://nas-01.example.lan:5001"),
            Self::InterfaceName => json!("eth0"),
            Self::InterfaceDescr => json!("Intel Corporation I210 Gigabit"),
            Self::InterfaceAlias => json!("uplink to core-sw-01"),
            Self::InterfaceMac => json!("3c:ec:ef:12:34:57"),
            Self::InterfaceSpeedBps => json!(1_000_000_000_i64),
            Self::InterfaceAdminStatus => serde_json::to_value(IfAdminStatus::Up).unwrap(),
            Self::InterfaceOperStatus => serde_json::to_value(IfOperStatus::Up).unwrap(),
        }
    }

    /// The host or interface field this key writes, read off the coverage tables so the two can
    /// never disagree.
    pub fn target_field(self) -> &'static str {
        let table = match self.scope() {
            SshScriptScope::Host => host_coverage(),
            SshScriptScope::Interface => interface_coverage(),
        };
        table
            .into_iter()
            .find(|(_, coverage)| *coverage == ScriptCoverage::Writable(self))
            .map(|(field, _)| field)
            .expect("every SshScriptField is Writable in exactly one coverage table")
    }
}

/// Whether a script may write one field of `HostBase` or `InterfaceBase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptCoverage {
    Writable(SshScriptField),
    /// Not writable from a script, and why.
    NotWritable(&'static str),
}

/// Build a coverage table and, from the same field list, an exhaustive destructure of `$ty`. A
/// field added to `$ty` without a line here fails the build in the destructure.
macro_rules! script_coverage {
    ($ty:ident { $($field:ident => $coverage:expr),* $(,)? }) => {{
        #[allow(dead_code)]
        fn exhaustive(value: $ty) {
            let $ty { $($field: _),* } = value;
        }
        vec![$((stringify!($field), $coverage)),*]
    }};
}

const IDENTITY: &str = "assigned by Scanopy";
const NAMING: &str = "derived by Scanopy from the host's identifiers; scripts set hostname";
const USER_SET: &str = "set by a person in Scanopy";
const VIRTUALIZATION: &str = "set by the hypervisor and container integrations";
const NEIGHBOURS: &str = "neighbour data comes from LLDP, CDP and forwarding tables";

pub fn host_coverage() -> Vec<(&'static str, ScriptCoverage)> {
    use ScriptCoverage::{NotWritable, Writable};
    use SshScriptField as F;
    script_coverage!(HostBase {
        name => NotWritable(NAMING),
        network_id => NotWritable(IDENTITY),
        hostname => Writable(F::Hostname),
        description => NotWritable(USER_SET),
        source => NotWritable(IDENTITY),
        virtualization_metadata => NotWritable(VIRTUALIZATION),
        virtualization_service_id => NotWritable(VIRTUALIZATION),
        hidden => NotWritable(USER_SET),
        tags => NotWritable(USER_SET),
        sys_descr => Writable(F::SysDescr),
        sys_object_id => Writable(F::SysObjectId),
        sys_location => Writable(F::SysLocation),
        sys_contact => Writable(F::SysContact),
        management_url => Writable(F::ManagementUrl),
        chassis_id => Writable(F::ChassisId),
        sys_name => Writable(F::SysName),
        manufacturer => Writable(F::Manufacturer),
        model => Writable(F::Model),
        serial_number => Writable(F::SerialNumber),
        firmware_revision => Writable(F::FirmwareRevision),
        software_revision => Writable(F::SoftwareRevision),
        os => Writable(F::Os),
        credential_assignments => NotWritable(USER_SET),
    })
}

pub fn interface_coverage() -> Vec<(&'static str, ScriptCoverage)> {
    use ScriptCoverage::{NotWritable, Writable};
    use SshScriptField as F;
    script_coverage!(InterfaceBase {
        host_id => NotWritable(IDENTITY),
        network_id => NotWritable(IDENTITY),
        if_index => NotWritable("an SNMP table index; interfaces are matched by name, then MAC"),
        if_descr => Writable(F::InterfaceDescr),
        if_name => Writable(F::InterfaceName),
        if_alias => Writable(F::InterfaceAlias),
        if_type => NotWritable("an IANA ifType number only SNMP reports reliably"),
        speed_bps => Writable(F::InterfaceSpeedBps),
        admin_status => Writable(F::InterfaceAdminStatus),
        oper_status => Writable(F::InterfaceOperStatus),
        mac_address => Writable(F::InterfaceMac),
        ip_address_id => NotWritable("linked by Scanopy from the addresses discovery finds"),
        ip_configured => NotWritable("derived by Scanopy from the linked address"),
        neighbor_candidates => NotWritable(NEIGHBOURS),
        fdb_macs => NotWritable(NEIGHBOURS),
        native_vlan_id => NotWritable("VLAN membership comes from SNMP and controller integrations"),
        vlan_ids => NotWritable("VLAN membership comes from SNMP and controller integrations"),
    })
}

/// A script's stdout, parsed. Unknown keys are kept so the run can report them.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct SshScriptOutput {
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub sys_name: Option<String>,
    #[serde(default)]
    pub sys_descr: Option<String>,
    #[serde(default)]
    pub sys_object_id: Option<String>,
    #[serde(default)]
    pub sys_location: Option<String>,
    #[serde(default)]
    pub sys_contact: Option<String>,
    #[serde(default)]
    pub chassis_id: Option<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub serial_number: Option<String>,
    #[serde(default)]
    pub firmware_revision: Option<String>,
    #[serde(default)]
    pub software_revision: Option<String>,
    /// Kept as raw JSON so a malformed object is reported as one invalid key rather than failing
    /// the whole document; the daemon decodes it when applying.
    #[serde(default)]
    pub os: Option<serde_json::Value>,
    #[serde(default)]
    pub management_url: Option<String>,
    #[serde(default)]
    pub interfaces: Option<Vec<SshScriptInterface>>,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct SshScriptInterface {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub descr: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub mac: Option<String>,
    #[serde(default)]
    pub speed_bps: Option<i64>,
    #[serde(default)]
    pub admin_status: Option<IfAdminStatus>,
    #[serde(default)]
    pub oper_status: Option<IfOperStatus>,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, serde_json::Value>,
}

impl SshScriptOutput {
    /// Parse a script's stdout. The error is serde's, which names the line and column.
    pub fn parse(stdout: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(stdout.trim())
    }

    /// Every key the contract does not know, as a path (`interfaces[2].colour`).
    pub fn unknown_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.unknown.keys().cloned().collect();
        for (i, iface) in self.interfaces.iter().flatten().enumerate() {
            keys.extend(iface.unknown.keys().map(|k| format!("interfaces[{i}].{k}")));
        }
        keys
    }
}

/// Largest stderr excerpt a run summary carries.
pub const MAX_STDERR_EXCERPT_BYTES: usize = 4 * 1024;

/// How one script run ended.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, IntoStaticStr,
)]
pub enum SshScriptOutcome {
    /// Exited 0 with a valid document; its fields were applied.
    Applied,
    /// Exited non-zero. Nothing was applied.
    NonZeroExit,
    /// Did not finish within the credential's timeout. Nothing was applied.
    TimedOut,
    /// Exited 0 but stdout was not one JSON object of known types. Nothing was applied.
    InvalidOutput,
    /// Printed more than [`MAX_SCRIPT_OUTPUT_BYTES`]. Nothing was applied.
    OutputTooLarge,
    /// The host presented a different key from the pinned or required one. The script never ran.
    HostKeyMismatch,
    /// The host refused the username, password or key. The script never ran.
    AuthenticationFailed,
    /// The SSH session could not be set up after the probe reached the host. The script never ran.
    ConnectionFailed,
    /// An outcome from a newer daemon than this server.
    #[serde(other)]
    Unknown,
}

/// What one SSH script did on one host, carried on the run's results.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct SshScriptRun {
    /// The address the script ran at.
    #[schema(value_type = String)]
    pub ip: std::net::IpAddr,
    pub outcome: SshScriptOutcome,
    /// The script's exit status, when it exited.
    pub exit_code: Option<u32>,
    /// Keys whose values were applied, as paths (`model`, `interfaces[].mac`).
    #[serde(default)]
    pub applied_keys: Vec<String>,
    /// Keys that were present and not applied, as paths (`interfaces[0].vlan`): keys the
    /// contract does not know, and values that did not parse.
    #[serde(default)]
    pub rejected_keys: Vec<String>,
    /// The end of stderr, or the parse error, capped at [`MAX_STDERR_EXCERPT_BYTES`].
    pub detail: Option<String>,
    /// How long the script ran, in milliseconds.
    pub duration_ms: u64,
}

/// One row of the documentation table.
#[derive(Debug, Clone, Serialize)]
pub struct SshScriptFieldDoc {
    pub key: String,
    pub scope: SshScriptScope,
    pub value_type: &'static str,
    /// The host or interface field it writes, e.g. `serial_number` or `if_alias`.
    pub target_field: &'static str,
    pub description: &'static str,
    pub example: serde_json::Value,
}

/// The documentation view of the contract, for the `ssh-script-fields.json` fixture.
pub fn ssh_script_fields() -> Vec<SshScriptFieldDoc> {
    SshScriptField::iter()
        .map(|f| SshScriptFieldDoc {
            key: f.path(),
            scope: f.scope(),
            value_type: f.value_type(),
            target_field: f.target_field(),
            description: f.description(),
            example: f.example(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn every_field_is_writable_in_exactly_one_table_of_its_scope() {
        let mut seen: HashMap<SshScriptField, usize> = HashMap::new();
        for (scope, table) in [
            (SshScriptScope::Host, host_coverage()),
            (SshScriptScope::Interface, interface_coverage()),
        ] {
            for (field, coverage) in table {
                if let ScriptCoverage::Writable(f) = coverage {
                    assert_eq!(f.scope(), scope, "{field} is in the wrong table for {f:?}");
                    *seen.entry(f).or_default() += 1;
                }
            }
        }
        for f in SshScriptField::iter() {
            assert_eq!(
                seen.get(&f),
                Some(&1),
                "{f:?} must be Writable exactly once"
            );
        }
    }

    #[test]
    fn every_documented_example_parses_with_no_unknown_keys() {
        let mut host = serde_json::Map::new();
        let mut iface = serde_json::Map::new();
        for f in SshScriptField::iter() {
            let target = match f.scope() {
                SshScriptScope::Host => &mut host,
                SshScriptScope::Interface => &mut iface,
            };
            target.insert(f.key().to_string(), f.example());
        }
        host.insert("interfaces".into(), serde_json::json!([iface]));
        let parsed = SshScriptOutput::parse(&serde_json::Value::Object(host).to_string())
            .expect("the documented examples are a valid document");
        assert!(
            parsed.unknown_keys().is_empty(),
            "{:?}",
            parsed.unknown_keys()
        );
    }

    #[test]
    fn unknown_keys_are_reported_with_their_path() {
        let parsed = SshScriptOutput::parse(
            r#"{"model": "X", "colour": "red", "interfaces": [{"name": "eth0", "vlan": 5}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.model.as_deref(), Some("X"));
        assert_eq!(parsed.unknown_keys(), vec!["colour", "interfaces[0].vlan"]);
    }

    #[test]
    fn a_wrongly_typed_known_key_is_a_parse_error() {
        assert!(SshScriptOutput::parse(r#"{"interfaces": [{"speed_bps": "fast"}]}"#).is_err());
        assert!(SshScriptOutput::parse("not json").is_err());
    }
}
