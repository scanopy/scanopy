//! SNMP Value Conversion Utilities
//!
//! Functions for extracting typed values from SNMP varbinds.

use crate::server::ip_addresses::r#impl::base::is_unset_mac;
use crate::server::shared::storage::pg_value::strip_nuls;
use mac_address::MacAddress;
use snmp2::Value;
use std::net::IpAddr;

/// Extract a string value from an SNMP varbind value.
///
/// NUL bytes are removed. Firmware NUL-terminates or NUL-pads OCTET STRINGs — D-Link's DGS series
/// sends `lldpRemPortId` as `31 00`, i.e. `"1\0"` (GH #668) — and 0x00 is valid UTF-8, so nothing
/// upstream rejects it. Two things then break: PostgreSQL cannot store the character at all, and
/// the daemon's own in-memory matching (`remap_lldp_local_ports` comparing `lldpLocPortId` against
/// `ifName`) never equates `"1\0"` with `"1"`. The storage layer sanitises independently, but it
/// runs too late for that second one, so the trim belongs here as well as there.
pub fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::OctetString(bytes) => String::from_utf8(bytes.to_vec())
            .ok()
            .map(|s| strip_nuls(&s).into_owned()),
        Value::ObjectIdentifier(oid) => oid.iter().map(|iter| {
            let parts: Vec<String> = iter.map(|n| n.to_string()).collect();
            parts.join(".")
        }),
        _ => None,
    }
}

/// Name the ASN.1 shape a varbind arrived as.
///
/// Used where a value is rejected for being the wrong type. A count of rejections says something
/// went wrong; the type says *what the agent actually sent*, which is the difference between
/// "this device is quiet" and "this device answers chassis IDs with a `Null`".
pub fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Boolean(_) => "Boolean",
        Value::Null => "Null",
        Value::Integer(_) => "Integer",
        Value::OctetString(_) => "OctetString",
        Value::ObjectIdentifier(_) => "ObjectIdentifier",
        Value::Sequence(_) => "Sequence",
        Value::Set(_) => "Set",
        Value::Constructed(_, _) => "Constructed",
        Value::IpAddress(_) => "IpAddress",
        Value::Counter32(_) => "Counter32",
        Value::Unsigned32(_) => "Unsigned32",
        Value::Timeticks(_) => "Timeticks",
        Value::Opaque(_) => "Opaque",
        Value::Counter64(_) => "Counter64",
        Value::EndOfMibView => "EndOfMibView",
        Value::NoSuchObject => "NoSuchObject",
        Value::NoSuchInstance => "NoSuchInstance",
        // The remaining variants are PDU shapes, not varbind values, so they cannot reach a
        // column handler. Grouped rather than enumerated so adding a PDU type upstream does not
        // churn a diagnostic helper that would never name it.
        _ => "Pdu",
    }
}

/// Extract an integer value from an SNMP varbind value
pub fn value_to_i32(value: &Value) -> Option<i32> {
    match value {
        Value::Integer(n) => i32::try_from(*n).ok(),
        Value::Unsigned32(n) => i32::try_from(*n).ok(),
        Value::Counter32(n) => i32::try_from(*n).ok(),
        Value::Counter64(n) => i32::try_from(*n).ok(),
        Value::Timeticks(n) => i32::try_from(*n).ok(),
        _ => None,
    }
}

/// Extract a u64 value from an SNMP varbind value
pub fn value_to_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Integer(n) => u64::try_from(*n).ok(),
        Value::Unsigned32(n) => Some(*n as u64),
        Value::Counter32(n) => Some(*n as u64),
        Value::Counter64(n) => Some(*n),
        Value::Timeticks(n) => Some(*n as u64),
        _ => None,
    }
}

/// Extract a MAC address from an SNMP varbind value.
///
/// Six zero bytes read as no address, the same as an empty value: Net-SNMP answers `lo`'s
/// `ifPhysAddress` that way, where other agents send nothing.
pub fn value_to_mac(value: &Value) -> Option<MacAddress> {
    match value {
        Value::OctetString(bytes) if bytes.len() == 6 => {
            let arr: [u8; 6] = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
            Some(MacAddress::new(arr)).filter(|mac| !is_unset_mac(mac))
        }
        _ => None,
    }
}

/// Extract an IP address from an SNMP varbind value.
/// Handles IpAddress (4-byte) and OctetString (4-byte) formats.
pub fn value_to_ip(value: &Value) -> Option<IpAddr> {
    match value {
        Value::IpAddress(bytes) => Some(IpAddr::from(*bytes)),
        Value::OctetString(bytes) if bytes.len() == 4 => {
            Some(IpAddr::from([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        _ => None,
    }
}

/// Extract an unsigned 16-bit value from an SNMP varbind value
pub fn value_to_u16(value: &Value) -> Option<u16> {
    match value {
        Value::Integer(n) => u16::try_from(*n).ok(),
        Value::Unsigned32(n) => u16::try_from(*n).ok(),
        Value::Counter32(n) => u16::try_from(*n).ok(),
        _ => None,
    }
}

/// Parse a Q-BRIDGE PortList OCTET STRING into a Vec of bridge port numbers (1-based).
///
/// The PortList is an MSB-first bitmap: bit 0 (most significant) of octet 0 = port 1,
/// bit 1 of octet 0 = port 2, etc. This follows the BRIDGE-MIB PortList convention.
pub fn parse_portlist_bitmap(bytes: &[u8]) -> Vec<i32> {
    let mut ports = Vec::new();
    for (octet_idx, &byte) in bytes.iter().enumerate() {
        for bit in 0..8u32 {
            if byte & (0x80 >> bit) != 0 {
                let port_num = (octet_idx as i32) * 8 + (bit as i32) + 1;
                ports.push(port_num);
            }
        }
    }
    ports
}

/// Parse a Q-BRIDGE `dot1qTpFdbTable` OID index suffix into a MAC address.
///
/// The table is indexed by `{ dot1qFdbId, dot1qTpFdbAddress }`, so the OID suffix
/// after a column base is `[fdb_id, mac0, mac1, mac2, mac3, mac4, mac5]` — 7
/// sub-identifiers, with the 6-octet MAC carried in the index (not a column value).
/// Returns `None` for a wrong-length suffix or an octet outside 0..=255.
pub fn qbridge_fdb_index_to_mac(suffix: &[u64]) -> Option<MacAddress> {
    if suffix.len() != 7 {
        return None;
    }
    let mac = &suffix[1..7];
    if mac.iter().any(|&o| o > 255) {
        return None;
    }
    Some(MacAddress::new([
        mac[0] as u8,
        mac[1] as u8,
        mac[2] as u8,
        mac[3] as u8,
        mac[4] as u8,
        mac[5] as u8,
    ]))
}

/// Parse LLDP management address from raw SNMP bytes.
///
/// SNMP returns the address in one of these formats:
/// - Raw IPv4: 4 bytes
/// - Raw IPv6: 16 bytes
/// - With IANA address family prefix: 1 byte family + address bytes
///   (family 1 = IPv4, family 2 = IPv6 per IANA Address Family Numbers)
pub fn parse_lldp_mgmt_addr(bytes: &[u8]) -> Option<IpAddr> {
    match bytes.len() {
        // Raw IPv4
        4 => Some(IpAddr::from(<[u8; 4]>::try_from(bytes).ok()?)),
        // Raw IPv6
        16 => Some(IpAddr::from(<[u8; 16]>::try_from(bytes).ok()?)),
        // IANA family 1 (IPv4) + 4 address bytes
        5 if bytes[0] == 1 => Some(IpAddr::from(<[u8; 4]>::try_from(&bytes[1..5]).ok()?)),
        // IANA family 2 (IPv6) + 16 address bytes
        17 if bytes[0] == 2 => Some(IpAddr::from(<[u8; 16]>::try_from(&bytes[1..17]).ok()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_portlist_bitmap_empty() {
        assert_eq!(parse_portlist_bitmap(&[]), Vec::<i32>::new());
    }

    #[test]
    fn test_parse_portlist_bitmap_all_set() {
        assert_eq!(parse_portlist_bitmap(&[0xFF]), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_parse_portlist_bitmap_msb_only() {
        assert_eq!(parse_portlist_bitmap(&[0x80]), vec![1]);
    }

    #[test]
    fn test_parse_portlist_bitmap_lsb_only() {
        assert_eq!(parse_portlist_bitmap(&[0x01]), vec![8]);
    }

    #[test]
    fn test_parse_portlist_bitmap_multi_octet() {
        assert_eq!(parse_portlist_bitmap(&[0x80, 0x40]), vec![1, 10]);
    }

    #[test]
    fn test_parse_portlist_bitmap_all_zeros() {
        assert_eq!(parse_portlist_bitmap(&[0x00, 0x00]), Vec::<i32>::new());
    }

    /// GH #668: D-Link's DGS series NUL-terminates its OCTET STRINGs. The byte is valid UTF-8, so
    /// it survived the decode, and then PostgreSQL refused the whole row.
    #[test]
    fn value_to_string_strips_nul_padding() {
        assert_eq!(
            value_to_string(&Value::OctetString(&[0x31, 0x00])),
            Some("1".to_string())
        );
        assert_eq!(
            value_to_string(&Value::OctetString(b"Slot0/9\0\0")),
            Some("Slot0/9".to_string())
        );
    }

    #[test]
    fn value_to_string_leaves_a_clean_string_alone() {
        assert_eq!(
            value_to_string(&Value::OctetString(b"ten-gigabitEthernet 1/0/1")),
            Some("ten-gigabitEthernet 1/0/1".to_string())
        );
    }

    /// Naming the type is the point: a count of rejections says something went wrong, the type
    /// says what the agent actually sent.
    #[test]
    fn value_type_name_names_the_shapes_a_column_handler_can_reject() {
        assert_eq!(value_type_name(&Value::Null), "Null");
        assert_eq!(value_type_name(&Value::Integer(4)), "Integer");
        assert_eq!(value_type_name(&Value::OctetString(b"x")), "OctetString");
        assert_eq!(value_type_name(&Value::NoSuchObject), "NoSuchObject");
    }

    #[test]
    fn test_value_to_u16_integer() {
        assert_eq!(value_to_u16(&Value::Integer(100)), Some(100));
    }

    #[test]
    fn test_value_to_u16_unsigned32() {
        assert_eq!(value_to_u16(&Value::Unsigned32(4094)), Some(4094));
    }

    #[test]
    fn test_value_to_u16_overflow() {
        assert_eq!(value_to_u16(&Value::Integer(70000)), None);
    }

    #[test]
    fn test_qbridge_fdb_index_to_mac_valid() {
        // Suffix = [fdb_id=20, aa:bb:cc:dd:ee:ff]. The MAC is the last 6 sub-ids.
        let suffix = [20u64, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        assert_eq!(
            qbridge_fdb_index_to_mac(&suffix),
            Some(MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]))
        );
    }

    #[test]
    fn test_qbridge_fdb_index_to_mac_wrong_length() {
        // Legacy dot1dTpFdbTable suffix (6 sub-ids, no fdb_id) must not parse here.
        assert_eq!(
            qbridge_fdb_index_to_mac(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
            None
        );
    }

    #[test]
    fn test_qbridge_fdb_index_to_mac_octet_out_of_range() {
        let suffix = [1u64, 256, 0, 0, 0, 0, 0];
        assert_eq!(qbridge_fdb_index_to_mac(&suffix), None);
    }
}
