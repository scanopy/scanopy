//! The provenanced values a host carries.
//!
//! One declaration per column, each naming its two wire keys, whether a re-read may move it, and
//! what an empty one looks like. Every field on [`HostBase`](super::base::HostBase) that discovery
//! writes is declared here — the `is_none()` gate that used to merge them let whichever probe ran
//! first own the value permanently, and there was no record of which one that was.

use crate::attributed_value;
use crate::server::shared::attribution::string_schema;

/// Blank means absent, for every string-valued attribute: a rung with nothing to attribute is not
/// a fact, and whitespace is not a model number.
fn blank(value: &str) -> bool {
    value.trim().is_empty()
}

attributed_value! {
    /// SNMP sysDescr.0 — full system description.
    pub struct HostSysDescrValue(String) as HostSysDescrAttributed {
        key: "sys_descr",
        source_key: "sys_descr_source",
        schema_name: "HostSysDescr",
        refreshable: true,
        blank: blank,
        schema: string_schema("SNMP sysDescr.0 - full system description"),
    }
}

attributed_value! {
    /// SNMP sysObjectID.0 — vendor OID for device identification.
    pub struct HostSysObjectIdValue(String) as HostSysObjectIdAttributed {
        key: "sys_object_id",
        source_key: "sys_object_id_source",
        schema_name: "HostSysObjectId",
        refreshable: true,
        blank: blank,
        schema: string_schema("SNMP sysObjectID.0 - vendor OID for device identification"),
    }
}

attributed_value! {
    /// SNMP sysLocation.0 — physical location.
    ///
    /// Human-authored at its source: an operator types it into the device, and we read it back.
    /// That is what `AttributeSource::Authored` exists to say.
    pub struct HostSysLocationValue(String) as HostSysLocationAttributed {
        key: "sys_location",
        source_key: "sys_location_source",
        schema_name: "HostSysLocation",
        refreshable: true,
        blank: blank,
        schema: string_schema("SNMP sysLocation.0 - physical location"),
    }
}

attributed_value! {
    /// SNMP sysContact.0 — admin contact info. Human-authored, as `sys_location` is.
    pub struct HostSysContactValue(String) as HostSysContactAttributed {
        key: "sys_contact",
        source_key: "sys_contact_source",
        schema_name: "HostSysContact",
        refreshable: true,
        blank: blank,
        schema: string_schema("SNMP sysContact.0 - admin contact info"),
    }
}

attributed_value! {
    /// URL for the device's management interface.
    pub struct HostManagementUrlValue(String) as HostManagementUrlAttributed {
        key: "management_url",
        source_key: "management_url_source",
        schema_name: "HostManagementUrl",
        refreshable: true,
        blank: blank,
        schema: string_schema("URL for device management interface (manual or discovered)"),
    }
}

attributed_value! {
    /// LLDP lldpLocChassisId — globally unique device identifier, used for deduplication.
    pub struct HostChassisIdValue(String) as HostChassisIdAttributed {
        key: "chassis_id",
        source_key: "chassis_id_source",
        schema_name: "HostChassisId",
        refreshable: true,
        blank: blank,
        schema: string_schema(
            "LLDP lldpLocChassisId - globally unique device identifier for deduplication",
        ),
    }
}

attributed_value! {
    /// SNMP sysName.0 — administratively-assigned hostname.
    pub struct HostSysNameValue(String) as HostSysNameAttributed {
        key: "sys_name",
        source_key: "sys_name_source",
        schema_name: "HostSysName",
        refreshable: true,
        blank: blank,
        schema: string_schema("SNMP sysName.0 - administratively-assigned hostname"),
    }
}

attributed_value! {
    /// The host's hostname: from a PTR lookup, the host's own OS, a controller's DHCP leases, or an
    /// mDNS SRV record.
    ///
    /// An identifier rather than a name, so it keeps its own column and is never copied into
    /// `name` (see the placement rule in `super::name`). Refreshable, because a lease or a DNS
    /// record can legitimately change.
    pub struct HostHostnameValue(String) as HostHostnameAttributed {
        key: "hostname",
        source_key: "hostname_source",
        schema_name: "HostHostname",
        refreshable: true,
        blank: blank,
        schema: string_schema("Hostname as resolved or reported for the host"),
    }
}

attributed_value! {
    /// ENTITY-MIB entPhysicalMfgName — hardware manufacturer.
    ///
    /// Not refreshable: a device does not change manufacturer. A stronger source may still correct
    /// a weaker one's guess — which matters here, because two of the three writers synthesise the
    /// string rather than reading it.
    pub struct HostManufacturerValue(String) as HostManufacturerAttributed {
        key: "manufacturer",
        source_key: "manufacturer_source",
        schema_name: "HostManufacturer",
        refreshable: false,
        blank: blank,
        schema: string_schema("ENTITY-MIB entPhysicalMfgName - hardware manufacturer"),
    }
}

attributed_value! {
    /// ENTITY-MIB entPhysicalModelName — hardware model.
    ///
    /// Refreshable, unlike the serial: the device's model does not change, but the *reported* model
    /// improves — a weak source writes "Cisco Switch" and SNMP later reports `WS-C2960X-48FPD-L`.
    /// Safe only because `Manual` sits above everything discovery can write.
    pub struct HostModelValue(String) as HostModelAttributed {
        key: "model",
        source_key: "model_source",
        schema_name: "HostModel",
        refreshable: true,
        blank: blank,
        schema: string_schema("ENTITY-MIB entPhysicalModelName - hardware model"),
    }
}

attributed_value! {
    /// ENTITY-MIB entPhysicalSerialNum — hardware serial number.
    ///
    /// Not refreshable: a different serial is a different device, not a device whose serial moved.
    pub struct HostSerialNumberValue(String) as HostSerialNumberAttributed {
        key: "serial_number",
        source_key: "serial_number_source",
        schema_name: "HostSerialNumber",
        refreshable: false,
        blank: blank,
        schema: string_schema("ENTITY-MIB entPhysicalSerialNum - hardware serial number"),
    }
}

attributed_value! {
    /// Firmware revision of the device as a whole — ENTITY-MIB `entPhysicalFirmwareRev`.
    ///
    /// Refreshable, and the field that most needs to be: it changes on every upgrade, which is the
    /// point of tracking it.
    ///
    /// Firmware alone, not "firmware or software". It was the latter until ENTITY-MIB arrived as
    /// the fifth writer and the first able to report two versions at once — `.9` here and `.10` in
    /// [`HostSoftwareRevisionValue`]. A column holding either with nothing recording which is a
    /// union with no discriminator, so on a Cisco chassis a stored `15.0(2)SE11` could be the
    /// bootloader or the IOS version and nothing could tell them apart. The other four writers
    /// (UniFi, Instant On, Modbus, EtherNet/IP) each report one version and it is this one.
    pub struct HostFirmwareRevisionValue(String) as HostFirmwareRevisionAttributed {
        key: "firmware_revision",
        source_key: "firmware_revision_source",
        schema_name: "HostFirmwareRevision",
        refreshable: true,
        blank: blank,
        schema: string_schema("ENTITY-MIB entPhysicalFirmwareRev - firmware revision of the device as a whole"),
    }
}

attributed_value! {
    /// Software revision of the device as a whole — ENTITY-MIB `entPhysicalSoftwareRev`.
    ///
    /// Refreshable for the same reason as the firmware: an OS upgrade changing it is the point of
    /// tracking it.
    ///
    /// Separate from [`HostFirmwareRevisionValue`] rather than folded into it, because RFC 4133
    /// defines `.9` and `.10` as distinct objects and a chassis with a bootloader and an OS
    /// genuinely has two versions. Only ENTITY-MIB writes this today; every other source reports a
    /// single version, which is the firmware.
    pub struct HostSoftwareRevisionValue(String) as HostSoftwareRevisionAttributed {
        key: "software_revision",
        source_key: "software_revision_source",
        schema_name: "HostSoftwareRevision",
        refreshable: true,
        blank: blank,
        schema: string_schema("ENTITY-MIB entPhysicalSoftwareRev - software revision of the device as a whole"),
    }
}
