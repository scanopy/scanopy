//! EtherNet/IP ListIdentity over unicast UDP.
//!
//! The one candidate protocol whose discovery packet returns full device identity: a 24-byte
//! request with no payload, answered with vendor ID, device type, product code, revision, status,
//! serial number and a product-name string.
//!
//! **Unicast only.** Broadcast ListIdentity is a sweep — one transmission, an unknown number of
//! asynchronous replies — which does not fit a per-address probe and belongs with the other sweep
//! phases. TCP 44818 carries the same identity behind a session registration this does not need.
//!
//! **No ODVA vendor table.** The vendor ID is recorded as the number the device reported. Shipping
//! a vendor table means vendoring a list that rots without a refresh script, and the product name
//! the device supplies is already human-readable and is what meets the "product version" bar.
//!
//! The frame is little-endian throughout **except the 16-byte socket address**, which is a
//! `sockaddr_in` in network byte order sitting in the middle of an otherwise little-endian item.

use anyhow::Error;
use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, DeviceIdentity, ProbeContext, identity_field,
};
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;

/// Encapsulation command: ListIdentity.
const COMMAND_LIST_IDENTITY: u16 = 0x0063;
/// The encapsulation header is a fixed 24 bytes.
const HEADER_LEN: usize = 24;
/// CPF item type: CIP Identity.
const ITEM_CIP_IDENTITY: u16 = 0x000C;
/// Item data before the product name: protocol version (2), socket address (16), vendor (2),
/// device type (2), product code (2), revision (2), status (2), serial (4).
const IDENTITY_FIXED_LEN: usize = 32;

/// Echoed back by the device, which is what ties the reply to this request.
const SENDER_CONTEXT: [u8; 8] = *b"scanopy1";

pub struct EtherNetIpProbe;

#[async_trait]
impl AppProbe for EtherNetIpProbe {
    fn port(&self) -> PortType {
        PortType::EtherNetIp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::EtherNetIp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let target = std::net::SocketAddr::new(ctx.ip, self.port().number());

        if socket
            .send_to(&list_identity_request(), target)
            .await
            .is_err()
        {
            return Ok(AppProbeOutcome::NoAnswer);
        }

        let mut buf = [0u8; 1024];
        let Ok(Ok((len, from))) = timeout(SCAN_TIMEOUT, socket.recv_from(&mut buf)).await else {
            return Ok(AppProbeOutcome::NoAnswer);
        };

        // The same source check `test_bacnet_service` makes: a reply from somewhere else is
        // somebody else's device.
        if from.ip() != ctx.ip {
            return Ok(AppProbeOutcome::NoAnswer);
        }

        Ok(match parse_reply(&buf[..len]) {
            Some(identity) => AppProbeOutcome::Answered {
                identity: Some(identity),
            },
            None => AppProbeOutcome::NoAnswer,
        })
    }
}

/// A ListIdentity request: the encapsulation header alone, with no command-specific data.
fn list_identity_request() -> [u8; HEADER_LEN] {
    let mut frame = [0u8; HEADER_LEN];
    frame[0..2].copy_from_slice(&COMMAND_LIST_IDENTITY.to_le_bytes());
    frame[2..4].copy_from_slice(&0u16.to_le_bytes()); // length: no payload
    frame[4..8].copy_from_slice(&0u32.to_le_bytes()); // session handle: none yet
    frame[8..12].copy_from_slice(&0u32.to_le_bytes()); // status: 0 on a request
    frame[12..20].copy_from_slice(&SENDER_CONTEXT);
    frame[20..24].copy_from_slice(&0u32.to_le_bytes()); // options
    frame
}

fn parse_reply(frame: &[u8]) -> Option<DeviceIdentity> {
    if frame.len() < HEADER_LEN {
        return None;
    }

    let command = u16::from_le_bytes([frame[0], frame[1]]);
    let length = u16::from_le_bytes([frame[2], frame[3]]) as usize;
    let status = u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]);

    if command != COMMAND_LIST_IDENTITY || frame[12..20] != SENDER_CONTEXT {
        return None;
    }
    // A non-zero status is the device refusing the command rather than describing itself.
    if status != 0 || frame.len() < HEADER_LEN + length {
        return None;
    }

    let payload = &frame[HEADER_LEN..HEADER_LEN + length];
    // Item count, then each item's type and length.
    if payload.len() < 2 {
        return None;
    }
    let item_count = u16::from_le_bytes([payload[0], payload[1]]) as usize;
    let mut items = &payload[2..];

    for _ in 0..item_count {
        if items.len() < 4 {
            break;
        }
        let item_type = u16::from_le_bytes([items[0], items[1]]);
        let item_len = u16::from_le_bytes([items[2], items[3]]) as usize;
        if items.len() < 4 + item_len {
            break;
        }
        let data = &items[4..4 + item_len];

        if item_type == ITEM_CIP_IDENTITY
            && let Some(identity) = parse_identity_item(data)
        {
            return Some(identity);
        }
        items = &items[4 + item_len..];
    }

    None
}

/// The CIP Identity item. Little-endian except the socket address at bytes 2..18.
fn parse_identity_item(data: &[u8]) -> Option<DeviceIdentity> {
    if data.len() < IDENTITY_FIXED_LEN + 1 {
        return None;
    }

    // data[0..2]   encapsulation protocol version
    // data[2..18]  sockaddr_in, network byte order — not read: we already know the address
    let vendor_id = u16::from_le_bytes([data[18], data[19]]);
    // data[20..22] device type, data[22..24] product code — recorded through the product name
    let revision_major = data[24];
    let revision_minor = data[25];
    // data[26..28] status
    let serial_number = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

    let name_len = data[32] as usize;
    let product_name = data
        .get(33..33 + name_len)
        .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
        .filter(|name| !name.is_empty());

    let probe = AttributeSource::Probe(ClientProbe::EtherNetIp);
    Some(DeviceIdentity {
        // The number, not a name. Recording it as an explicit CIP vendor reference is honest about
        // what it is; resolving it would need the ODVA registry, which is deliberately not shipped.
        //
        // Hence `CipVendorId` rather than the probe: this string is our construction from an
        // identifier, not something the device said, so it ranks as the inference it is and cannot
        // displace a manufacturer name read off the device — which the probe rung, being the
        // device's own protocol, otherwise would.
        manufacturer: identity_field(
            Some(format!("CIP vendor {vendor_id}")),
            AttributeSource::CipVendorId,
        ),
        model: identity_field(product_name, probe),
        // CIP serial numbers are conventionally written as eight hex digits.
        serial_number: identity_field(Some(format!("{serial_number:08X}")), probe),
        firmware_revision: identity_field(
            Some(format!("{revision_major}.{revision_minor}")),
            probe,
        ),
        // The identity object names a product, not the OS running it.
        os: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_item(vendor: u16, serial: u32, revision: (u8, u8), name: &str) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_le_bytes()); // encapsulation protocol version
        // sockaddr_in, big-endian: family, port, address, then eight zero bytes.
        data.extend_from_slice(&2i16.to_be_bytes());
        data.extend_from_slice(&44818u16.to_be_bytes());
        data.extend_from_slice(&u32::from(std::net::Ipv4Addr::new(192, 0, 2, 10)).to_be_bytes());
        data.extend_from_slice(&[0u8; 8]);
        data.extend_from_slice(&vendor.to_le_bytes());
        data.extend_from_slice(&14u16.to_le_bytes()); // device type
        data.extend_from_slice(&158u16.to_le_bytes()); // product code
        data.push(revision.0);
        data.push(revision.1);
        data.extend_from_slice(&0x0060u16.to_le_bytes()); // status
        data.extend_from_slice(&serial.to_le_bytes());
        data.push(name.len() as u8);
        data.extend_from_slice(name.as_bytes());
        data.push(0x03); // state
        data
    }

    fn reply(items: &[(u16, Vec<u8>)], context: [u8; 8], status: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(items.len() as u16).to_le_bytes());
        for (item_type, data) in items {
            payload.extend_from_slice(&item_type.to_le_bytes());
            payload.extend_from_slice(&(data.len() as u16).to_le_bytes());
            payload.extend_from_slice(data);
        }

        let mut frame = Vec::new();
        frame.extend_from_slice(&COMMAND_LIST_IDENTITY.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        frame.extend_from_slice(&0u32.to_le_bytes());
        frame.extend_from_slice(&status.to_le_bytes());
        frame.extend_from_slice(&context);
        frame.extend_from_slice(&0u32.to_le_bytes());
        frame.extend_from_slice(&payload);
        frame
    }

    #[test]
    fn a_list_identity_reply_yields_vendor_product_name_serial_and_revision() {
        let item = identity_item(1, 0x0060_F4A2, (32, 11), "1769-L18ER-BB1B/A LOGIX5318ER");

        let identity = parse_reply(&reply(&[(ITEM_CIP_IDENTITY, item)], SENDER_CONTEXT, 0))
            .expect("a well-formed reply parses");

        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.manufacturer).as_deref(),
            Some("CIP vendor 1")
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.model).as_deref(),
            Some("1769-L18ER-BB1B/A LOGIX5318ER")
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.serial_number).as_deref(),
            Some("0060F4A2")
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.firmware_revision).as_deref(),
            Some("32.11")
        );
    }

    /// The echoed sender context is what ties a reply to this request rather than to a broadcast
    /// somebody else on the segment sent.
    #[test]
    fn a_reply_that_does_not_echo_our_sender_context_is_rejected() {
        let item = identity_item(1, 1, (1, 1), "Somebody Else");

        assert!(parse_reply(&reply(&[(ITEM_CIP_IDENTITY, item)], *b"someone1", 0)).is_none());
    }

    #[test]
    fn a_reply_carrying_an_error_status_is_not_an_identity() {
        let item = identity_item(1, 1, (1, 1), "PLC");

        assert!(
            parse_reply(&reply(&[(ITEM_CIP_IDENTITY, item)], SENDER_CONTEXT, 0x0001)).is_none()
        );
    }

    #[test]
    fn another_protocol_on_the_port_is_not_ethernet_ip() {
        assert!(parse_reply(b"HTTP/1.1 400 Bad Request\r\n\r\n").is_none());
        assert!(parse_reply(&[0u8; 24]).is_none());
        assert!(parse_reply(b"").is_none());
    }

    /// Devices pad the product name and some report none at all. An empty name is absence, and
    /// the vendor ID and serial the device did supply still stand.
    #[test]
    fn a_padded_or_empty_product_name_does_not_lose_the_rest() {
        let padded = identity_item(1, 0xABCD, (2, 5), "  PowerFlex 525  ");
        let identity =
            parse_reply(&reply(&[(ITEM_CIP_IDENTITY, padded)], SENDER_CONTEXT, 0)).expect("parses");
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.model).as_deref(),
            Some("PowerFlex 525")
        );

        let unnamed = identity_item(1, 0xABCD, (2, 5), "");
        let identity = parse_reply(&reply(&[(ITEM_CIP_IDENTITY, unnamed)], SENDER_CONTEXT, 0))
            .expect("parses");
        assert_eq!(identity.model, None);
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.serial_number).as_deref(),
            Some("0000ABCD")
        );
    }

    /// A device may list other item types first; the CIP Identity item is found rather than
    /// assumed to be at a fixed offset.
    #[test]
    fn the_identity_item_is_found_among_others() {
        let item = identity_item(283, 0x11223344, (5, 1), "Turck TBEN-S");

        let identity = parse_reply(&reply(
            &[(0x0100, vec![0xDE, 0xAD]), (ITEM_CIP_IDENTITY, item)],
            SENDER_CONTEXT,
            0,
        ))
        .expect("parses past an unknown item");

        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.manufacturer).as_deref(),
            Some("CIP vendor 283")
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.model).as_deref(),
            Some("Turck TBEN-S")
        );
    }

    /// An item claiming more bytes than arrived must not panic or read past the buffer.
    #[test]
    fn a_truncated_identity_item_is_rejected_rather_than_read_past() {
        let mut truncated = identity_item(1, 1, (1, 1), "PLC");
        truncated.truncate(IDENTITY_FIXED_LEN);

        assert!(
            parse_reply(&reply(&[(ITEM_CIP_IDENTITY, truncated)], SENDER_CONTEXT, 0)).is_none()
        );
    }

    /// A name length running past the end of the item keeps what the fixed fields already gave.
    #[test]
    fn a_name_length_past_the_end_keeps_the_fixed_fields() {
        let mut item = identity_item(1, 0x0000_0001, (1, 2), "PLC");
        item[32] = 200; // claim a 200-byte name

        let identity =
            parse_reply(&reply(&[(ITEM_CIP_IDENTITY, item)], SENDER_CONTEXT, 0)).expect("parses");

        assert_eq!(identity.model, None);
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.serial_number).as_deref(),
            Some("00000001")
        );
        assert_eq!(
            crate::server::shared::attribution::text_of(&identity.firmware_revision).as_deref(),
            Some("1.2")
        );
    }

    #[test]
    fn the_request_is_a_bare_encapsulation_header() {
        let frame = list_identity_request();

        assert_eq!(frame.len(), HEADER_LEN);
        assert_eq!(
            u16::from_le_bytes([frame[0], frame[1]]),
            COMMAND_LIST_IDENTITY
        );
        assert_eq!(
            u16::from_le_bytes([frame[2], frame[3]]),
            0,
            "ListIdentity carries no command-specific data"
        );
        assert_eq!(frame[12..20], SENDER_CONTEXT);
    }
}
