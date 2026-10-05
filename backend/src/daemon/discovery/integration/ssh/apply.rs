//! Writing a parsed script document into the host being scanned.

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::InterfaceSource;
use crate::daemon::discovery::service::ops::HostData;
use crate::server::credentials::r#impl::types::ssh_script::{
    SshScriptField, SshScriptInterface, SshScriptOutput,
};
use crate::server::hosts::r#impl::os::HostOs;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::shared::attribution::AttributeSource;

const SOURCE: AttributeSource = AttributeSource::SshScript;

/// Apply every known key in `output` to `host_data`, returning the paths that were applied and
/// the ones that were present but unusable (an interface MAC that does not parse).
///
/// Both structs are destructured with no `..`, so a key added to the contract and parsed but not
/// handled here fails the build.
pub fn apply(
    output: SshScriptOutput,
    host_data: &mut HostData,
    interface_source: InterfaceSource,
    host_id: Uuid,
) -> (Vec<String>, Vec<String>) {
    let SshScriptOutput {
        hostname,
        sys_name,
        sys_descr,
        sys_object_id,
        sys_location,
        sys_contact,
        chassis_id,
        manufacturer,
        model,
        serial_number,
        firmware_revision,
        software_revision,
        os,
        management_url,
        interfaces,
        unknown: _,
    } = output;

    let mut applied: Vec<String> = Vec::new();
    let mut invalid: Vec<String> = Vec::new();
    let mut mark = |field: SshScriptField| {
        let path = field.path();
        if !applied.contains(&path) {
            applied.push(path);
        }
    };

    type Setter = fn(&mut HostData, String, AttributeSource) -> &mut HostData;
    let scalars: [(Option<String>, SshScriptField, Setter); 13] = [
        (hostname, SshScriptField::Hostname, HostData::with_hostname),
        (sys_name, SshScriptField::SysName, HostData::with_sys_name),
        (
            sys_descr,
            SshScriptField::SysDescr,
            HostData::with_sys_descr,
        ),
        (
            sys_object_id,
            SshScriptField::SysObjectId,
            HostData::with_sys_object_id,
        ),
        (
            sys_location,
            SshScriptField::SysLocation,
            HostData::with_sys_location,
        ),
        (
            sys_contact,
            SshScriptField::SysContact,
            HostData::with_sys_contact,
        ),
        (
            chassis_id,
            SshScriptField::ChassisId,
            HostData::with_chassis_id,
        ),
        (
            manufacturer,
            SshScriptField::Manufacturer,
            HostData::with_manufacturer,
        ),
        (model, SshScriptField::Model, HostData::with_model),
        (
            serial_number,
            SshScriptField::SerialNumber,
            HostData::with_serial_number,
        ),
        (
            firmware_revision,
            SshScriptField::FirmwareRevision,
            HostData::with_firmware_revision,
        ),
        (
            software_revision,
            SshScriptField::SoftwareRevision,
            HostData::with_software_revision,
        ),
        (
            management_url,
            SshScriptField::ManagementUrl,
            HostData::with_management_url,
        ),
    ];
    for (value, field, set) in scalars {
        // A blank string is "nothing to report", not a value to store.
        if let Some(value) = value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            set(host_data, value, SOURCE);
            mark(field);
        }
    }

    // An object rather than a string, so it is decoded here: an `os` that does not decode (an
    // unknown family, a missing one) is reported invalid and the rest of the document still lands.
    if let Some(os) = os.filter(|v| !v.is_null()) {
        match serde_json::from_value::<HostOs>(os) {
            Ok(os) => {
                host_data.with_os(os.without_blank_fields(), SOURCE);
                mark(SshScriptField::Os);
            }
            Err(_) => invalid.push(SshScriptField::Os.path()),
        }
    }

    if let Some(interfaces) = interfaces {
        let site_id = host_data.host.base.site_id;
        let rows: Vec<Interface> = interfaces
            .into_iter()
            .enumerate()
            .map(|(i, iface)| interface_row(iface, i, host_id, site_id, &mut mark, &mut invalid))
            .collect();
        if !rows.is_empty() {
            host_data.contribute_interfaces(
                interface_source,
                rows,
                // A script's list is whatever its author chose to print, never proof that an
                // interface it left out is gone, so the server must not prune against it.
                false,
                InterfaceDataComplete::none(),
            );
        }
    }

    (applied, invalid)
}

fn interface_row(
    iface: SshScriptInterface,
    index: usize,
    host_id: Uuid,
    site_id: Uuid,
    mark: &mut impl FnMut(SshScriptField),
    invalid: &mut Vec<String>,
) -> Interface {
    let SshScriptInterface {
        name,
        descr,
        alias,
        mac,
        speed_bps,
        admin_status,
        oper_status,
        unknown: _,
    } = iface;

    let mut present = |value: bool, field: SshScriptField| {
        if value {
            mark(field);
        }
    };
    let text = |v: Option<String>| v.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let name = text(name);
    let descr = text(descr);
    let alias = text(alias);
    present(name.is_some(), SshScriptField::InterfaceName);
    present(descr.is_some(), SshScriptField::InterfaceDescr);
    present(alias.is_some(), SshScriptField::InterfaceAlias);
    present(speed_bps.is_some(), SshScriptField::InterfaceSpeedBps);
    present(admin_status.is_some(), SshScriptField::InterfaceAdminStatus);
    present(oper_status.is_some(), SshScriptField::InterfaceOperStatus);

    let mac_address = match text(mac) {
        None => None,
        Some(raw) => match raw.parse::<MacAddress>() {
            Ok(m) => {
                present(true, SshScriptField::InterfaceMac);
                Some(MacEvidence::new(MacEvidenceValue(m), SOURCE))
            }
            Err(_) => {
                invalid.push(format!("interfaces[{index}].mac"));
                None
            }
        },
    };

    Interface::new(InterfaceBase {
        host_id,
        site_id,
        if_descr: descr,
        if_name: name,
        if_alias: alias,
        speed_bps,
        admin_status,
        oper_status,
        mac_address,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::InterfaceViewScope;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
    use crate::server::credentials::r#impl::types::ssh_script::SshScriptScope;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use strum::IntoEnumIterator;

    fn source() -> InterfaceSource {
        InterfaceSource {
            credential: CredentialQueryPayloadDiscriminants::Ssh,
            scope: InterfaceViewScope::PhysicalPortsOnly,
        }
    }

    fn blank_host() -> HostData {
        HostData::new(
            Host::new(HostBase::default()),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn document_of_every_example() -> String {
        let mut host = serde_json::Map::new();
        let mut iface = serde_json::Map::new();
        for f in SshScriptField::iter() {
            match f.scope() {
                SshScriptScope::Host => host.insert(f.key().into(), f.example()),
                SshScriptScope::Interface => iface.insert(f.key().into(), f.example()),
            };
        }
        host.insert("interfaces".into(), serde_json::json!([iface]));
        serde_json::Value::Object(host).to_string()
    }

    /// The guard on the whole contract: every key the docs list parses, is applied, and lands on
    /// the field its coverage table names, attributed to the script.
    #[test]
    fn every_field_parses_applies_and_lands() {
        let output = SshScriptOutput::parse(&document_of_every_example()).unwrap();
        let mut host_data = blank_host();
        let host_id = host_data.host.id;
        let (applied, invalid) = apply(output, &mut host_data, source(), host_id);
        assert!(invalid.is_empty(), "{invalid:?}");

        let host = serde_json::to_value(&host_data.host.base).unwrap();
        let iface = serde_json::to_value(&host_data.interfaces[0].base).unwrap();
        for f in SshScriptField::iter() {
            assert!(
                applied.contains(&f.path()),
                "{} was not reported applied",
                f.path()
            );
            let row = match f.scope() {
                SshScriptScope::Host => &host,
                SshScriptScope::Interface => &iface,
            };
            let target = f.target_field();
            assert!(
                !row[target].is_null(),
                "{} did not reach {target}: {row}",
                f.path()
            );
            if let Some(src) = row.get(format!("{target}_source")) {
                assert_eq!(src, "SshScript", "{} landed without attribution", f.path());
            }
        }
    }

    #[test]
    fn a_script_value_displaces_snmp_and_yields_to_a_manual_edit() {
        let mut host_data = blank_host();
        let host_id = host_data.host.id;
        host_data.with_model(
            "from snmp".into(),
            AttributeSource::Probe(crate::server::services::r#impl::patterns::ClientProbe::Snmp),
        );
        let output = SshScriptOutput::parse(r#"{"model": "from script"}"#).unwrap();
        apply(output, &mut host_data, source(), host_id);
        let model = |h: &HostData| h.host.base.model.as_ref().unwrap().value().0.clone();
        assert_eq!(model(&host_data), "from script");

        host_data.with_model("typed by a person".into(), AttributeSource::Manual);
        let output = SshScriptOutput::parse(r#"{"model": "from script"}"#).unwrap();
        apply(output, &mut host_data, source(), host_id);
        assert_eq!(model(&host_data), "typed by a person");
    }

    #[test]
    fn an_os_with_blank_fields_lands_without_them() {
        let output = SshScriptOutput::parse(
            r#"{"os": {"family": "Linux", "name": "Debian GNU/Linux", "version": "12", "codename": ""}}"#,
        )
        .unwrap();
        let mut host_data = blank_host();
        let host_id = host_data.host.id;
        apply(output, &mut host_data, source(), host_id);
        let os = &host_data.host.base.os.as_ref().unwrap().value().0;
        assert_eq!(os.name.as_deref(), Some("Debian GNU/Linux"));
        assert_eq!(os.codename, None);
    }

    #[test]
    fn an_os_with_an_unknown_family_is_reported_and_the_rest_kept() {
        let output =
            SshScriptOutput::parse(r#"{"model": "X11SCL-F", "os": {"family": "Plan9"}}"#).unwrap();
        let mut host_data = blank_host();
        let host_id = host_data.host.id;
        let (applied, invalid) = apply(output, &mut host_data, source(), host_id);
        assert_eq!(invalid, vec!["os"]);
        assert_eq!(applied, vec!["model"]);
        assert!(host_data.host.base.os.is_none());
    }

    #[test]
    fn an_unparseable_mac_is_reported_and_the_rest_of_the_row_kept() {
        let output =
            SshScriptOutput::parse(r#"{"interfaces": [{"name": "eth0", "mac": "nope"}]}"#).unwrap();
        let mut host_data = blank_host();
        let host_id = host_data.host.id;
        let (applied, invalid) = apply(output, &mut host_data, source(), host_id);
        assert_eq!(invalid, vec!["interfaces[0].mac"]);
        assert_eq!(applied, vec!["interfaces[].name"]);
        assert_eq!(
            host_data.interfaces[0].base.if_name.as_deref(),
            Some("eth0")
        );
    }
}
