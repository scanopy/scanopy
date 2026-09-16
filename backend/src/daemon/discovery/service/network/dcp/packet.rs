//! PROFINET DCP Identify wire format.
//!
//! Confirmed against Wireshark's `packet-pn-dcp.c` dissector (the durable open-source reference —
//! not a device, and not the spec itself, which this session had no access to). Validated against
//! no PROFINET device or simulator; every claim below is a claim to falsify against a real
//! responder or IEC 61158-6-10, not a given. See the module doc table for what that means for
//! coverage.
//!
//! Frame layout, after the 14-byte Ethernet header:
//! ```text
//! FrameID (2)  ServiceID (1)  ServiceType (1)  Xid (4)  ResponseDelay/Reserved (2)  DataLength (2)
//! ```
//! followed by `DataLength` bytes of blocks, each `Option (1) Suboption (1) BlockLength (2) value
//! (BlockLength, padded to even)`.

use mac_address::MacAddress;
use pnet::packet::Packet;
use pnet::packet::ethernet::{EtherType, EthernetPacket, MutableEthernetPacket};
use pnet::util::MacAddr;

/// PROFINET's EtherType. Not in `pnet::packet::ethernet::EtherTypes` — no PROFINET/DCP support
/// ships in `pnet_packet` at all, confirmed by reading its source.
pub const ETHERTYPE_PROFINET: u16 = 0x8892;

/// The DCP Identify multicast group every station listens on for this exchange.
pub fn dcp_identify_multicast() -> MacAddr {
    MacAddr::new(0x01, 0x0E, 0xCF, 0x00, 0x00, 0x00)
}

const FRAME_ID_DCP_IDENT_REQ: u16 = 0xFEFE;
const FRAME_ID_DCP_IDENT_RES: u16 = 0xFEFF;

const SERVICE_ID_IDENTIFY: u8 = 0x05;
const SERVICE_TYPE_REQUEST: u8 = 0x00;
const SERVICE_TYPE_RESPONSE_SUCCESS: u8 = 0x01;

const OPTION_ALL_SELECTOR: u8 = 0xFF;
const SUBOPTION_ALL_SELECTOR: u8 = 0xFF;

const OPTION_DEVICE: u8 = 0x02;
const SUBOPTION_DEVICE_NAME_OF_STATION: u8 = 0x02;

/// Response delay factor sent in the request, units of 10ms (so a responder distributes its
/// answer somewhere in `[0, factor * 10ms)` to avoid every station on the segment answering the
/// multicast at once). `100` (a 1s window) is a commonly used default in other open
/// implementations; the exact value is a courtesy to the network, not a correctness requirement —
/// unverified against the spec's own recommended range.
const RESPONSE_DELAY_FACTOR: u16 = 100;

const ETHERNET_HEADER_LEN: usize = 14;
const DCP_HEADER_LEN: usize = 10; // ServiceID+ServiceType+Xid+ResponseDelay/Reserved+DataLength
const ALL_SELECTOR_BLOCK_LEN: usize = 4; // Option+Suboption+BlockLength(0), no value

/// A parsed Identify Response: the responding station's MAC (read from the Ethernet source
/// address per the dissector's own handling — not from a DCP block, which carries none) and,
/// where the Device Properties block was present and decodable, its Name of Station.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DcpIdentifyResponse {
    pub mac: MacAddress,
    pub name_of_station: Option<String>,
}

/// Build a DCP Identify Request: an all-selector query (get everything back), addressed to the
/// multicast group, carrying `xid` so the response can be matched back to this request.
pub fn build_identify_request(source_mac: MacAddr, xid: u32) -> Vec<u8> {
    let dcp_len = DCP_HEADER_LEN + ALL_SELECTOR_BLOCK_LEN;
    let mut buf = vec![0u8; ETHERNET_HEADER_LEN + 2 + dcp_len];

    {
        let mut eth = MutableEthernetPacket::new(&mut buf[..ETHERNET_HEADER_LEN]).unwrap();
        eth.set_destination(dcp_identify_multicast());
        eth.set_source(source_mac);
        eth.set_ethertype(EtherType::new(ETHERTYPE_PROFINET));
    }

    let mut w = ETHERNET_HEADER_LEN;
    buf[w..w + 2].copy_from_slice(&FRAME_ID_DCP_IDENT_REQ.to_be_bytes());
    w += 2;
    buf[w] = SERVICE_ID_IDENTIFY;
    w += 1;
    buf[w] = SERVICE_TYPE_REQUEST;
    w += 1;
    buf[w..w + 4].copy_from_slice(&xid.to_be_bytes());
    w += 4;
    buf[w..w + 2].copy_from_slice(&RESPONSE_DELAY_FACTOR.to_be_bytes());
    w += 2;
    buf[w..w + 2].copy_from_slice(&(ALL_SELECTOR_BLOCK_LEN as u16).to_be_bytes());
    w += 2;

    buf[w] = OPTION_ALL_SELECTOR;
    w += 1;
    buf[w] = SUBOPTION_ALL_SELECTOR;
    w += 1;
    buf[w..w + 2].copy_from_slice(&0u16.to_be_bytes()); // block length: no value data
    w += 2;

    debug_assert_eq!(w, buf.len());
    buf
}

/// Parse a candidate Identify Response frame.
///
/// Rejects (returns `None`) anything that is not: the right EtherType, the right frame ID
/// (`0xFEFF`), Identify/ResponseSuccess, and — critically — the `Xid` this exchange sent, which is
/// what stops an unrelated or stale reply on a shared segment from being read as an answer to this
/// request.
pub fn parse_identify_response(frame: &[u8], expected_xid: u32) -> Option<DcpIdentifyResponse> {
    let ethernet = EthernetPacket::new(frame)?;
    if ethernet.get_ethertype() != EtherType::new(ETHERTYPE_PROFINET) {
        return None;
    }
    let mac = MacAddress::new(ethernet.get_source().octets());
    let payload = ethernet.payload();

    if payload.len() < 2 + DCP_HEADER_LEN {
        return None;
    }

    let frame_id = u16::from_be_bytes([payload[0], payload[1]]);
    if frame_id != FRAME_ID_DCP_IDENT_RES {
        return None;
    }

    let service_id = payload[2];
    let service_type = payload[3];
    if service_id != SERVICE_ID_IDENTIFY || service_type != SERVICE_TYPE_RESPONSE_SUCCESS {
        return None;
    }

    let xid = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    if xid != expected_xid {
        return None;
    }

    let data_length = u16::from_be_bytes([payload[10], payload[11]]) as usize;
    let blocks_start = 2 + DCP_HEADER_LEN;
    let blocks_end = blocks_start.saturating_add(data_length).min(payload.len());
    let blocks = &payload[blocks_start..blocks_end];

    Some(DcpIdentifyResponse {
        mac,
        name_of_station: find_name_of_station(blocks),
    })
}

/// Walk the block/TLV section for the Device Properties → Name of Station block.
///
/// Best-effort: the exact byte layout inside a "Device" block beyond the option/suboption/length
/// header is a secondary detail this session could not confirm against the spec (some DCP blocks
/// carry a leading `BlockInfo` qualifier, some do not, and which applies to an Identify Response's
/// Name of Station block specifically is unverified). Reads the whole block value as the name;
/// flagged in the Work Summary as a claim to check against a real capture.
fn find_name_of_station(mut blocks: &[u8]) -> Option<String> {
    let mut found = None;
    while blocks.len() >= 4 {
        let option = blocks[0];
        let suboption = blocks[1];
        let block_len = u16::from_be_bytes([blocks[2], blocks[3]]) as usize;
        let value_start: usize = 4;
        let value_end = value_start.saturating_add(block_len).min(blocks.len());
        let value = &blocks[value_start..value_end];

        if option == OPTION_DEVICE && suboption == SUBOPTION_DEVICE_NAME_OF_STATION {
            found = std::str::from_utf8(value)
                .ok()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
        }

        // Blocks are padded to an even total length.
        let consumed = value_end + (block_len % 2);
        if consumed == 0 || consumed > blocks.len() {
            break;
        }
        blocks = &blocks[consumed..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_mac() -> MacAddr {
        MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55)
    }

    #[test]
    fn request_is_addressed_to_the_dcp_multicast_group_with_the_profinet_ethertype() {
        let packet = build_identify_request(source_mac(), 0x0F00_0001);
        let eth = EthernetPacket::new(&packet).unwrap();

        assert_eq!(eth.get_destination(), dcp_identify_multicast());
        assert_eq!(eth.get_source(), source_mac());
        assert_eq!(eth.get_ethertype(), EtherType::new(ETHERTYPE_PROFINET));
    }

    #[test]
    fn request_carries_identify_request_header_and_all_selector_block() {
        let xid = 0x0F12_3456;
        let packet = build_identify_request(source_mac(), xid);
        let eth = EthernetPacket::new(&packet).unwrap();
        let payload = eth.payload();

        assert_eq!(
            u16::from_be_bytes([payload[0], payload[1]]),
            FRAME_ID_DCP_IDENT_REQ
        );
        assert_eq!(payload[2], SERVICE_ID_IDENTIFY);
        assert_eq!(payload[3], SERVICE_TYPE_REQUEST);
        assert_eq!(
            u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            xid
        );
        let data_length = u16::from_be_bytes([payload[10], payload[11]]) as usize;
        assert_eq!(data_length, ALL_SELECTOR_BLOCK_LEN);

        let block = &payload[12..];
        assert_eq!(block[0], OPTION_ALL_SELECTOR);
        assert_eq!(block[1], SUBOPTION_ALL_SELECTOR);
        assert_eq!(u16::from_be_bytes([block[2], block[3]]), 0);
    }

    /// Build a well-formed Identify Response frame for parser tests, with a Device/NameOfStation
    /// block whose value bytes are `name` (no padding decision baked in — `name` chooses it).
    fn build_response(
        responder_mac: MacAddr,
        xid: u32,
        service_id: u8,
        service_type: u8,
        frame_id: u16,
        name: Option<&str>,
    ) -> Vec<u8> {
        let mut blocks = Vec::new();
        if let Some(name) = name {
            blocks.push(OPTION_DEVICE);
            blocks.push(SUBOPTION_DEVICE_NAME_OF_STATION);
            let bytes = name.as_bytes();
            blocks.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            blocks.extend_from_slice(bytes);
            if bytes.len() % 2 == 1 {
                blocks.push(0);
            }
        }

        let mut buf = vec![0u8; ETHERNET_HEADER_LEN];
        {
            let mut eth = MutableEthernetPacket::new(&mut buf).unwrap();
            eth.set_destination(source_mac());
            eth.set_source(responder_mac);
            eth.set_ethertype(EtherType::new(ETHERTYPE_PROFINET));
        }

        buf.extend_from_slice(&frame_id.to_be_bytes());
        buf.push(service_id);
        buf.push(service_type);
        buf.extend_from_slice(&xid.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes()); // reserved on a response
        buf.extend_from_slice(&(blocks.len() as u16).to_be_bytes());
        buf.extend_from_slice(&blocks);
        buf
    }

    #[test]
    fn a_well_formed_response_is_parsed_with_its_name_of_station() {
        let responder = MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF);
        let xid = 0x0F00_00AA;
        let packet = build_response(
            responder,
            xid,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            Some("press-line-3"),
        );

        let parsed = parse_identify_response(&packet, xid).unwrap();
        assert_eq!(parsed.mac, MacAddress::new(responder.octets()));
        assert_eq!(parsed.name_of_station, Some("press-line-3".to_string()));
    }

    #[test]
    fn a_response_with_no_name_of_station_block_still_parses() {
        let responder = MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF);
        let xid = 1;
        let packet = build_response(
            responder,
            xid,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            None,
        );

        let parsed = parse_identify_response(&packet, xid).unwrap();
        assert_eq!(parsed.name_of_station, None);
    }

    #[test]
    fn the_mac_comes_from_the_ethernet_source_address_not_a_block() {
        let responder = MacAddr::new(0x11, 0x22, 0x33, 0x44, 0x55, 0x66);
        let xid = 1;
        // Name of Station block deliberately holds an unrelated string, to prove the MAC is not
        // read from block data even when a block is present.
        let packet = build_response(
            responder,
            xid,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            Some("not-a-mac"),
        );

        let parsed = parse_identify_response(&packet, xid).unwrap();
        assert_eq!(parsed.mac, MacAddress::new(responder.octets()));
    }

    #[test]
    fn wrong_ethertype_is_rejected() {
        let mut packet = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            1,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            None,
        );
        let mut eth = MutableEthernetPacket::new(&mut packet).unwrap();
        eth.set_ethertype(EtherType::new(0x0800)); // IPv4, not PROFINET
        assert!(parse_identify_response(&packet, 1).is_none());
    }

    #[test]
    fn wrong_frame_id_is_rejected() {
        let packet = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            1,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_REQ, // a request's frame ID, not a response's
            None,
        );
        assert!(parse_identify_response(&packet, 1).is_none());
    }

    #[test]
    fn wrong_service_id_or_type_is_rejected() {
        let wrong_service = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            1,
            0x03, // Get, not Identify
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            None,
        );
        assert!(parse_identify_response(&wrong_service, 1).is_none());

        let wrong_type = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            1,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_REQUEST, // a request's type, not a response's
            FRAME_ID_DCP_IDENT_RES,
            None,
        );
        assert!(parse_identify_response(&wrong_type, 1).is_none());
    }

    #[test]
    fn mismatched_xid_is_rejected() {
        let packet = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            0x0F00_0001,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            None,
        );
        // Parsed against a different Xid than the one the frame carries — this is a stray or
        // stale reply, not an answer to our request.
        assert!(parse_identify_response(&packet, 0x0F00_0002).is_none());
    }

    #[test]
    fn truncated_frame_is_rejected_not_panicked_on() {
        let full = build_response(
            MacAddr::new(1, 2, 3, 4, 5, 6),
            1,
            SERVICE_ID_IDENTIFY,
            SERVICE_TYPE_RESPONSE_SUCCESS,
            FRAME_ID_DCP_IDENT_RES,
            Some("name"),
        );
        for cut in [0, 1, 14, 20, 25] {
            assert!(parse_identify_response(&full[..cut], 1).is_none());
        }
    }
}
