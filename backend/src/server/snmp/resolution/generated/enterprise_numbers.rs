// AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
// Run `cargo test --test integration generate_fixtures` to regenerate
// Source: https://www.iana.org/assignments/enterprise-numbers/enterprise-numbers

use phf::phf_map;

/// IANA Private Enterprise Numbers mapped to organization names.
/// These are the numeric suffixes of OIDs under 1.3.6.1.4.1 (iso.org.dod.internet.private.enterprise)
static ENTERPRISE_NUMBERS: phf::Map<u32, &'static str> = phf_map! {
    0u32 => "Reserved",
    2u32 => "IBM",
    9u32 => "ciscoSystems",
    11u32 => "Hewlett-Packard",
    42u32 => "Sun Microsystems",
    43u32 => "3Com",
    45u32 => "SynOptics",
    63u32 => "Apple Computer",
    72u32 => "Retix",
    94u32 => "Nokia",
    171u32 => "D-Link",
    207u32 => "Allied Telesis",
    232u32 => "Compaq",
    311u32 => "Microsoft",
    318u32 => "APC",
    674u32 => "Dell",
    789u32 => "Juniper Networks",
    1916u32 => "Extreme Networks",
    2011u32 => "Huawei",
    2021u32 => "UCD-SNMP-MIB",
    2636u32 => "Juniper Networks (alternate)",
    3076u32 => "Alteon",
    3224u32 => "Netscreen",
    3375u32 => "F5 Networks",
    3417u32 => "Blue Coat",
    4874u32 => "Unisphere Networks",
    5624u32 => "Foundry Networks",
    6027u32 => "Force10 Networks",
    6486u32 => "Alcatel-Lucent",
    6527u32 => "Alcatel-Lucent (Timetra)",
    6876u32 => "VMware",
    8072u32 => "Net-SNMP",
    9148u32 => "Acme Packet",
    9303u32 => "PacketFront",
    12356u32 => "Fortinet",
    14179u32 => "Airespace (Cisco WLC)",
    14988u32 => "MikroTik",
    25506u32 => "H3C",
    30065u32 => "Arista Networks",
    35098u32 => "Ruckus",
    41112u32 => "Ubiquiti Networks",
};

/// Look up an enterprise name by its IANA Private Enterprise Number.
/// Returns the organization name if found, or None if the number is not in the registry.
///
/// # Example
/// ```
/// use scanopy::server::snmp::resolution::generated::get_enterprise_name;
///
/// assert_eq!(get_enterprise_name(9), Some("ciscoSystems"));
/// assert_eq!(get_enterprise_name(311), Some("Microsoft"));
/// assert_eq!(get_enterprise_name(999999), None);
/// ```
pub fn get_enterprise_name(enterprise_number: u32) -> Option<&'static str> {
    ENTERPRISE_NUMBERS.get(&enterprise_number).copied()
}

/// Extract enterprise number from a sysObjectID OID string.
/// The sysObjectID typically has the format "1.3.6.1.4.1.{enterprise}.{product...}"
/// This function extracts the enterprise number (the first component after 1.3.6.1.4.1).
///
/// # Example
/// ```
/// use scanopy::server::snmp::resolution::generated::extract_enterprise_number;
///
/// assert_eq!(extract_enterprise_number("1.3.6.1.4.1.9.1.1"), Some(9)); // Cisco
/// assert_eq!(extract_enterprise_number("1.3.6.1.4.1.2011.2.23"), Some(2011)); // Huawei
/// assert_eq!(extract_enterprise_number("1.3.6.1.2.1"), None); // Not under enterprise
/// ```
#[allow(dead_code)] // Used by SNMP collection module
pub fn extract_enterprise_number(oid: &str) -> Option<u32> {
    const ENTERPRISE_PREFIX: &str = "1.3.6.1.4.1.";

    if !oid.starts_with(ENTERPRISE_PREFIX) {
        return None;
    }

    let suffix = &oid[ENTERPRISE_PREFIX.len()..];
    let enterprise_str = suffix.split('.').next()?;
    enterprise_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_enterprises() {
        assert_eq!(get_enterprise_name(9), Some("ciscoSystems"));
        assert_eq!(get_enterprise_name(311), Some("Microsoft"));
        assert_eq!(get_enterprise_name(674), Some("Dell"));
        assert_eq!(get_enterprise_name(6876), Some("VMware"));
    }

    #[test]
    fn test_unknown_enterprise() {
        assert_eq!(get_enterprise_name(999999999), None);
    }

    #[test]
    fn test_extract_enterprise_number() {
        assert_eq!(extract_enterprise_number("1.3.6.1.4.1.9.1.1"), Some(9));
        assert_eq!(
            extract_enterprise_number("1.3.6.1.4.1.2011.2.23.999"),
            Some(2011)
        );
        assert_eq!(extract_enterprise_number("1.3.6.1.2.1.1.1.0"), None);
        assert_eq!(extract_enterprise_number("invalid"), None);
    }
}
