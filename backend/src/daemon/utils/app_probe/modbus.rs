//! Modbus TCP detection and Read Device Identification.
//!
//! One request does both jobs. Function `0x2B` / MEI `0x0E` is Read Device Identification, and the
//! three ways a listener can answer it separate cleanly:
//!
//! | Reply | Meaning |
//! |---|---|
//! | Function `0x2B` | Modbus, and here is the vendor, product code and revision |
//! | Function `0xAB` (`0x2B \| 0x80`) | **Modbus, `0x2B` unsupported** — an exception is still an answer |
//! | Anything else, or a frame that does not echo our transaction ID | Not Modbus |
//!
//! The middle row is the one that matters most: `0x2B` is optional in the specification and a
//! substantial share of field devices omit it, so the exception path is the common case rather
//! than the edge case. Getting it wrong in the "not Modbus" direction would lose most of the
//! devices this probe exists to find.
//!
//! **Unit 1, then unit 0, on one connection. No enumeration.** Some gateways answer as unit 0
//! ("self") and most field devices as unit 1. Enumerating units is a loop bound with no schema
//! behind it, and the documented consequence of not doing it is that a gateway fronting forty
//! serial field devices reports as one device.

use anyhow::Error;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, DeviceIdentity, ProbeContext, identity_field,
};
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;

/// Encapsulated Interface Transport.
const FUNCTION_READ_DEVICE_ID: u8 = 0x2B;
/// The exception bit set on the function code, per the Modbus specification.
const FUNCTION_EXCEPTION: u8 = FUNCTION_READ_DEVICE_ID | 0x80;
/// MEI type: Read Device Identification.
const MEI_READ_DEVICE_ID: u8 = 0x0E;
/// Read Device ID code 1: the basic stream, which is the only one every compliant device must
/// hold. Codes 2 and 3 are regular and extended and are optional on top.
const READ_DEVICE_ID_BASIC: u8 = 0x01;

/// The three basic objects, which are the only ones this asks for.
const OBJECT_VENDOR_NAME: u8 = 0x00;
const OBJECT_PRODUCT_CODE: u8 = 0x01;
const OBJECT_REVISION: u8 = 0x02;

/// Unit 1 first — most field devices. Unit 0 second, which some gateways use to mean "self".
const UNITS_TO_TRY: [u8; 2] = [1, 0];

/// MBAP header length: transaction id, protocol id, length, unit id.
const MBAP_LEN: usize = 7;

pub struct ModbusProbe;

#[async_trait]
impl AppProbe for ModbusProbe {
    fn port(&self) -> PortType {
        PortType::ModbusTcp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::ModbusTcp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let addr = std::net::SocketAddr::new(ctx.ip, self.port().number());

        // Both units on one connection where the device allows it, which is the multi-round-trip
        // case the stage exists to support.
        //
        // **A dead connection is not a verdict.** A server that does not serve the unit it was
        // asked about may reset rather than answer — pymodbus does exactly this — and giving up
        // there would mean unit 0 was never tried, defeating the point of trying both. So a
        // transport failure drops the connection and the next unit gets a fresh one; only a reply
        // that is positively *not* Modbus ends the exchange early.
        let mut stream: Option<TcpStream> = None;
        let mut detected = false;

        for (attempt, unit) in UNITS_TO_TRY.iter().enumerate() {
            if ctx.cancel.is_cancelled() {
                break;
            }

            if stream.is_none() {
                match timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
                    Ok(Ok(fresh)) => stream = Some(fresh),
                    Ok(Err(e)) => {
                        ctx.note_connect_error(&e);
                        break;
                    }
                    Err(_) => break,
                }
            }
            let Some(open) = stream.as_mut() else {
                break;
            };

            // Vary the transaction id per attempt so a reply to the first request cannot be
            // mistaken for a reply to the second.
            let transaction_id = 0x5CA1u16.wrapping_add(attempt as u16);
            match exchange(open, transaction_id, *unit).await {
                Ok(ModbusReply::Identity(identity)) => {
                    return Ok(AppProbeOutcome::Answered {
                        identity: Some(identity),
                    });
                }
                // Keep going: a gateway may answer unit 0 fully having refused unit 1.
                Ok(ModbusReply::NoIdentity) => detected = true,
                Ok(ModbusReply::NotModbus) => break,
                Err(_) => stream = None,
            }
        }

        Ok(if detected {
            AppProbeOutcome::Answered { identity: None }
        } else {
            AppProbeOutcome::NoAnswer
        })
    }
}

/// One request/response round trip on an open connection.
async fn exchange(
    stream: &mut TcpStream,
    transaction_id: u16,
    unit: u8,
) -> Result<ModbusReply, Error> {
    stream
        .write_all(&read_device_id_request(transaction_id, unit))
        .await?;

    let mut buf = [0u8; 512];
    let read = timeout(SCAN_TIMEOUT, stream.read(&mut buf)).await??;
    Ok(parse_reply(&buf[..read], transaction_id))
}

/// MBAP header plus a Read Device Identification PDU.
fn read_device_id_request(transaction_id: u16, unit: u8) -> [u8; MBAP_LEN + 4] {
    let pdu = [
        FUNCTION_READ_DEVICE_ID,
        MEI_READ_DEVICE_ID,
        READ_DEVICE_ID_BASIC,
        OBJECT_VENDOR_NAME,
    ];
    // The length field counts the unit id and the PDU, not itself or anything before it.
    let length = (pdu.len() + 1) as u16;

    let mut frame = [0u8; MBAP_LEN + 4];
    frame[0..2].copy_from_slice(&transaction_id.to_be_bytes());
    frame[2..4].copy_from_slice(&0u16.to_be_bytes()); // protocol id: 0 means Modbus
    frame[4..6].copy_from_slice(&length.to_be_bytes());
    frame[6] = unit;
    frame[7..].copy_from_slice(&pdu);
    frame
}

/// What came back on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModbusReply {
    /// Modbus, and it answered Read Device Identification.
    Identity(DeviceIdentity),
    /// Modbus, but it will not answer `0x2B`.
    NoIdentity,
    /// Nothing that speaks Modbus produced this.
    NotModbus,
}

fn parse_reply(frame: &[u8], expected_transaction_id: u16) -> ModbusReply {
    // MBAP header plus at least a function code.
    if frame.len() < MBAP_LEN + 1 {
        return ModbusReply::NotModbus;
    }

    let transaction_id = u16::from_be_bytes([frame[0], frame[1]]);
    let protocol_id = u16::from_be_bytes([frame[2], frame[3]]);
    let length = u16::from_be_bytes([frame[4], frame[5]]) as usize;

    // The echoed transaction id is the whole discriminator: any listener can accept a connection,
    // but echoing a value we chose is not something an unrelated protocol does by accident.
    if transaction_id != expected_transaction_id || protocol_id != 0 {
        return ModbusReply::NotModbus;
    }
    // The declared length has to agree with what arrived, or this is not a framed MBAP reply.
    // `<` rather than `!=` because a device may pipeline and we only read the first frame.
    if length < 2 || frame.len() < length + 6 {
        return ModbusReply::NotModbus;
    }

    let pdu = &frame[MBAP_LEN..length + 6];
    match pdu[0] {
        FUNCTION_EXCEPTION => ModbusReply::NoIdentity,
        FUNCTION_READ_DEVICE_ID => match parse_identity(pdu) {
            Some(identity) => ModbusReply::Identity(identity),
            // Well-framed Modbus that answered `0x2B` with something unreadable. Still Modbus.
            None => ModbusReply::NoIdentity,
        },
        _ => ModbusReply::NotModbus,
    }
}

/// The Read Device Identification response PDU:
/// `2B 0E <code> <conformity> <more follows> <next object id> <count>` then `<id><len><value>`…
fn parse_identity(pdu: &[u8]) -> Option<DeviceIdentity> {
    const HEADER: usize = 7;
    if pdu.len() < HEADER || pdu[1] != MEI_READ_DEVICE_ID {
        return None;
    }

    let object_count = pdu[6] as usize;
    let mut objects = &pdu[HEADER..];

    let mut vendor_name = None;
    let mut product_code = None;
    let mut revision = None;

    for _ in 0..object_count {
        // Each object is at least an id and a length.
        if objects.len() < 2 {
            break;
        }
        let object_id = objects[0];
        let value_len = objects[1] as usize;
        if objects.len() < 2 + value_len {
            break;
        }
        // Device identification objects are ASCII by specification; a device that pads with
        // non-UTF-8 bytes should lose the object rather than the whole reply.
        let value = String::from_utf8_lossy(&objects[2..2 + value_len]).into_owned();
        match object_id {
            OBJECT_VENDOR_NAME => vendor_name = Some(value),
            OBJECT_PRODUCT_CODE => product_code = Some(value),
            OBJECT_REVISION => revision = Some(value),
            _ => {}
        }
        objects = &objects[2 + value_len..];
    }

    if vendor_name.is_none() && product_code.is_none() && revision.is_none() {
        return None;
    }

    // Modbus function 0x2B returns the vendor's own name for itself, over the device's own
    // protocol — the strongest reading there is for these fields.
    let source = AttributeSource::Probe(ClientProbe::ModbusTcp);
    Some(DeviceIdentity {
        manufacturer: identity_field(vendor_name, source),
        model: identity_field(product_code, source),
        serial_number: None,
        firmware_revision: identity_field(revision, source),
        // Modbus `0x2B` names a product, not the OS running it.
        os: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a Modbus `0x2B` reply claims: the vendor's own words, over the device's own protocol.
    const MODBUS: AttributeSource = AttributeSource::Probe(ClientProbe::ModbusTcp);

    const TXID: u16 = 0x5CA1;

    /// Build an MBAP frame around a PDU the way a device would.
    fn framed(transaction_id: u16, unit: u8, pdu: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&transaction_id.to_be_bytes());
        frame.extend_from_slice(&0u16.to_be_bytes());
        frame.extend_from_slice(&((pdu.len() + 1) as u16).to_be_bytes());
        frame.push(unit);
        frame.extend_from_slice(pdu);
        frame
    }

    fn object(id: u8, value: &str) -> Vec<u8> {
        let mut out = vec![id, value.len() as u8];
        out.extend_from_slice(value.as_bytes());
        out
    }

    fn identity_pdu(objects: &[Vec<u8>]) -> Vec<u8> {
        let mut pdu = vec![
            FUNCTION_READ_DEVICE_ID,
            MEI_READ_DEVICE_ID,
            READ_DEVICE_ID_BASIC,
            0x01, // conformity level
            0x00, // more follows: no
            0x00, // next object id
            objects.len() as u8,
        ];
        for o in objects {
            pdu.extend_from_slice(o);
        }
        pdu
    }

    #[test]
    fn a_device_answering_read_device_identification_yields_its_identity() {
        let pdu = identity_pdu(&[
            object(OBJECT_VENDOR_NAME, "Schneider Electric"),
            object(OBJECT_PRODUCT_CODE, "BMXP342020"),
            object(OBJECT_REVISION, "2.70"),
        ]);

        let reply = parse_reply(&framed(TXID, 1, &pdu), TXID);

        assert_eq!(
            reply,
            ModbusReply::Identity(DeviceIdentity {
                manufacturer: identity_field(Some("Schneider Electric".to_string()), MODBUS),
                model: identity_field(Some("BMXP342020".to_string()), MODBUS),
                serial_number: None,
                firmware_revision: identity_field(Some("2.70".to_string()), MODBUS),
                os: None,
            })
        );
    }

    /// The common case in the field, and the one that separates "not Modbus" from "Modbus that
    /// will not tell us what it is".
    #[test]
    fn an_exception_reply_is_modbus_without_an_identity() {
        // 0xAB, ILLEGAL FUNCTION.
        let reply = parse_reply(&framed(TXID, 1, &[FUNCTION_EXCEPTION, 0x01]), TXID);

        assert_eq!(reply, ModbusReply::NoIdentity);
    }

    #[test]
    fn a_reply_echoing_a_different_transaction_id_is_not_modbus() {
        let pdu = identity_pdu(&[object(OBJECT_VENDOR_NAME, "Somebody Else")]);

        let reply = parse_reply(&framed(TXID.wrapping_add(1), 1, &pdu), TXID);

        assert_eq!(reply, ModbusReply::NotModbus);
    }

    /// An HTTP server, an SSH banner, or anything else that accepts a connection on 502 and says
    /// something unrelated.
    #[test]
    fn a_reply_from_another_protocol_is_not_modbus() {
        let reply = parse_reply(b"HTTP/1.1 400 Bad Request\r\n\r\n", TXID);

        assert_eq!(reply, ModbusReply::NotModbus);
    }

    #[test]
    fn a_frame_shorter_than_its_declared_length_is_not_modbus() {
        let mut frame = framed(TXID, 1, &[FUNCTION_READ_DEVICE_ID, MEI_READ_DEVICE_ID]);
        // Claim far more payload than actually arrived.
        frame[4..6].copy_from_slice(&64u16.to_be_bytes());

        assert_eq!(parse_reply(&frame, TXID), ModbusReply::NotModbus);
    }

    /// A truncated object list must not take the whole reply down — the objects that did arrive
    /// are still the device's own answer about itself.
    #[test]
    fn an_object_running_past_the_end_keeps_what_parsed() {
        let mut pdu = identity_pdu(&[object(OBJECT_VENDOR_NAME, "Schneider Electric")]);
        pdu[6] = 2; // claim a second object that is not there
        pdu.extend_from_slice(&[OBJECT_PRODUCT_CODE, 40]); // length 40, no value

        let reply = parse_reply(&framed(TXID, 1, &pdu), TXID);

        assert_eq!(
            reply,
            ModbusReply::Identity(DeviceIdentity {
                manufacturer: identity_field(Some("Schneider Electric".to_string()), MODBUS),
                model: None,
                serial_number: None,
                firmware_revision: None,
                os: None,
            })
        );
    }

    /// `0x2B` answered with an empty object set is Modbus that told us nothing, not a parse
    /// failure and not a non-Modbus device.
    #[test]
    fn an_identity_reply_carrying_no_objects_is_modbus_without_an_identity() {
        let reply = parse_reply(&framed(TXID, 1, &identity_pdu(&[])), TXID);

        assert_eq!(reply, ModbusReply::NoIdentity);
    }

    /// The request has to be a well-formed MBAP frame or nothing answers it. Asserted
    /// structurally — the length field agreeing with the frame, the protocol id, the function and
    /// MEI type — rather than against a literal byte array.
    #[test]
    fn the_request_is_a_well_formed_mbap_read_device_id_frame() {
        let frame = read_device_id_request(TXID, 1);

        assert_eq!(u16::from_be_bytes([frame[0], frame[1]]), TXID);
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), 0, "protocol id");
        assert_eq!(
            u16::from_be_bytes([frame[4], frame[5]]) as usize,
            frame.len() - 6,
            "length counts the unit id and PDU only"
        );
        assert_eq!(frame[6], 1, "unit id");
        assert_eq!(frame[7], FUNCTION_READ_DEVICE_ID);
        assert_eq!(frame[8], MEI_READ_DEVICE_ID);
        assert_eq!(frame[9], READ_DEVICE_ID_BASIC);
    }
}
