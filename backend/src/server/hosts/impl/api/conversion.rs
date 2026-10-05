//! `HostResponse` ↔ `Host`, in both directions.
//!
//! Split out of `mod.rs` when the eleven discovered attributes each gained a source: the two
//! conversions are the only places the flat wire pair (`model` / `model_source`) is taken apart
//! and put back together, and they are long because they are exhaustive on purpose.

use super::*;

impl HostResponse {
    /// Convert HostResponse back to a Host entity (without children).
    /// Uses exhaustive destructuring to ensure compile error if HostResponse changes.
    pub fn to_host(&self) -> Host {
        // Exhaustive destructuring of HostResponse
        let HostResponse {
            id,
            created_at,
            updated_at,
            last_seen_at,
            first_discovery_id,
            last_discovery_id,
            name,
            // Derived from the fields below on the way out; nothing to carry back in.
            display_name: _,
            display_name_rung: _,
            name_ladder: _,
            name_source,
            site_id,
            hostname,
            hostname_source,
            description,
            source,
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id,
            hidden,
            tags,
            sys_descr,
            sys_descr_source,
            sys_object_id,
            sys_object_id_source,
            sys_location,
            sys_location_source,
            sys_contact,
            sys_contact_source,
            management_url,
            management_url_source,
            chassis_id,
            chassis_id_source,
            sys_name,
            sys_name_source,
            manufacturer,
            manufacturer_source,
            model,
            model_source,
            serial_number,
            serial_number_source,
            firmware_revision,
            firmware_revision_source,
            software_revision,
            software_revision_source,
            os,
            os_source,
            credential_assignments,
            ip_addresses: _,
            ports: _,
            services: _,
            interfaces: _,
        } = self;

        // The remaining SCD2 fields aren't in HostResponse; defaults are filled
        // in here. The to_host() method is only used in legacy compat paths;
        // round-tripping a HostResponse → Host loses temporal info that can be
        // reconstructed from the live row's values via from_row.
        Host {
            id: *id,
            created_at: *created_at,
            updated_at: *updated_at,
            valid_from: *created_at,
            valid_to: None,
            lineage_id: None,
            last_seen_at: *last_seen_at,
            last_discovery_id: *last_discovery_id,
            first_discovery_id: *first_discovery_id,
            base: HostBase {
                name: host_name_from_parts(name.clone(), *name_source),
                site_id: *site_id,
                hostname: hostname
                    .clone()
                    .map(|v| Attributed::new(HostHostnameValue(v), *hostname_source)),
                description: description.clone(),
                source: source.clone(),
                virtualization_metadata: virtualization_metadata.clone(),
                virtualization_service_id: *virtualization_service_id,
                virtualization_interface_id: *virtualization_interface_id,
                hidden: *hidden,
                tags: tags.clone(),
                sys_descr: sys_descr
                    .clone()
                    .map(|v| Attributed::new(HostSysDescrValue(v), *sys_descr_source)),
                sys_object_id: sys_object_id
                    .clone()
                    .map(|v| Attributed::new(HostSysObjectIdValue(v), *sys_object_id_source)),
                sys_location: sys_location
                    .clone()
                    .map(|v| Attributed::new(HostSysLocationValue(v), *sys_location_source)),
                sys_contact: sys_contact
                    .clone()
                    .map(|v| Attributed::new(HostSysContactValue(v), *sys_contact_source)),
                management_url: management_url
                    .clone()
                    .map(|v| Attributed::new(HostManagementUrlValue(v), *management_url_source)),
                chassis_id: chassis_id
                    .clone()
                    .map(|v| Attributed::new(HostChassisIdValue(v), *chassis_id_source)),
                sys_name: sys_name
                    .clone()
                    .map(|v| Attributed::new(HostSysNameValue(v), *sys_name_source)),
                manufacturer: manufacturer
                    .clone()
                    .map(|v| Attributed::new(HostManufacturerValue(v), *manufacturer_source)),
                model: model
                    .clone()
                    .map(|v| Attributed::new(HostModelValue(v), *model_source)),
                serial_number: serial_number
                    .clone()
                    .map(|v| Attributed::new(HostSerialNumberValue(v), *serial_number_source)),
                firmware_revision: firmware_revision.clone().map(|v| {
                    Attributed::new(HostFirmwareRevisionValue(v), *firmware_revision_source)
                }),
                software_revision: software_revision.clone().map(|v| {
                    Attributed::new(HostSoftwareRevisionValue(v), *software_revision_source)
                }),
                os: os
                    .clone()
                    .map(|v| Attributed::new(HostOsValue(v), *os_source)),
                credential_assignments: credential_assignments.clone(),
            },
        }
    }

    /// Build HostResponse from a Host and its children.
    /// Uses exhaustive destructuring to ensure compile error if Host/HostBase changes.
    pub fn from_host_with_children(
        host: Host,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        interfaces: Vec<Interface>,
    ) -> Self {
        // Before the destructure below consumes `host`. The same ladder topology titles a host
        // container with, so the two surfaces cannot disagree about what a nameless device is
        // called.
        // The title, the rung it came from and the rungs it beat are read off one ladder, so the
        // editor's explanation of a title cannot disagree with the title.
        let name_ladder = host.name_ladder(ip_addresses.iter());
        let (display_name, display_name_rung) =
            match crate::server::hosts::r#impl::name_ladder::resolve_name_ladder(&name_ladder) {
                Some((value, rung)) => (Some(value), Some(rung)),
                None => (None, None),
            };

        // Same reasoning, one level down: an interface's `display_name` is computed here rather
        // than walked again in the frontend, so a port cannot be labelled one thing in a list and
        // another in its own detail panel (a PROFINET DCP identify's bare-MAC interface used to
        // render as the literal string "Interface null" — `if_descr || \`Interface ${if_index}\``
        // interpolating two absent fields — because the frontend had reimplemented this ladder
        // three different, disagreeing ways instead of reading it from here).
        let interfaces: Vec<Interface> = interfaces
            .into_iter()
            .map(|mut interface| {
                interface.display_name = Some(interface.display_name());
                interface
            })
            .collect();

        // Exhaustive destructuring of Host
        let Host {
            id,
            created_at,
            updated_at,
            // `last_seen_at` and the discovery runs that first and last found the host are part
            // of the response shape: they drive the "Last seen", "First found by" and "Last found
            // by" columns. The version-history fields stay internal.
            last_seen_at,
            valid_from: _,
            valid_to: _,
            lineage_id: _,
            last_discovery_id,
            first_discovery_id,
            base,
        } = host;

        // Exhaustive destructuring of HostBase
        // If a field is added to HostBase, this will fail to compile
        let crate::server::hosts::r#impl::base::HostBase {
            name,
            site_id,
            hostname,
            description,
            source,
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name,
            manufacturer,
            model,
            serial_number,
            firmware_revision,
            software_revision,
            os,
            credential_assignments,
        } = base;

        Self {
            id,
            created_at,
            updated_at,
            last_seen_at,
            first_discovery_id,
            last_discovery_id,
            display_name,
            display_name_rung,
            name_ladder: name_ladder.to_vec(),
            name_source: name.source(),
            name: name.value().to_string(),
            site_id,
            hostname_source: hostname.as_ref().map(|v| v.source()).unwrap_or_default(),
            hostname: attribution::text_of(&hostname),
            description,
            source,
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id,
            hidden,
            tags,
            sys_descr_source: sys_descr.as_ref().map(|v| v.source()).unwrap_or_default(),
            sys_descr: attribution::text_of(&sys_descr),
            sys_object_id_source: sys_object_id
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            sys_object_id: attribution::text_of(&sys_object_id),
            sys_location_source: sys_location
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            sys_location: attribution::text_of(&sys_location),
            sys_contact_source: sys_contact.as_ref().map(|v| v.source()).unwrap_or_default(),
            sys_contact: attribution::text_of(&sys_contact),
            management_url_source: management_url
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            management_url: attribution::text_of(&management_url),
            chassis_id_source: chassis_id.as_ref().map(|v| v.source()).unwrap_or_default(),
            chassis_id: attribution::text_of(&chassis_id),
            sys_name_source: sys_name.as_ref().map(|v| v.source()).unwrap_or_default(),
            sys_name: attribution::text_of(&sys_name),
            manufacturer_source: manufacturer
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            manufacturer: attribution::text_of(&manufacturer),
            model_source: model.as_ref().map(|v| v.source()).unwrap_or_default(),
            model: attribution::text_of(&model),
            serial_number_source: serial_number
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            serial_number: attribution::text_of(&serial_number),
            firmware_revision_source: firmware_revision
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            firmware_revision: attribution::text_of(&firmware_revision),
            software_revision_source: software_revision
                .as_ref()
                .map(|v| v.source())
                .unwrap_or_default(),
            software_revision: attribution::text_of(&software_revision),
            os_source: os.as_ref().map(|v| v.source()).unwrap_or_default(),
            os: os.map(|v| v.into_value().0),
            credential_assignments,
            ip_addresses,
            ports,
            services,
            interfaces,
        }
    }
}
