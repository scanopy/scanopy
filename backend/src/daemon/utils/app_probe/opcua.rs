//! OPC UA detection over the binary transport.
//!
//! The transport opens with a `HEL` message answered by `ACK`. Both are short fixed-layout
//! messages and the exchange is session-less — it negotiates buffer sizes and holds no server
//! state — which is why this is safe to run by default.
//!
//! **An `ERR` reply counts as detection too.** A server that dislikes the endpoint URL answers
//! `Bad_TcpEndpointUrlInvalid` rather than `ACK`, and that is still an OPC UA-framed reply from an
//! OPC UA listener. Treating only `ACK` as proof would miss every server whose configured endpoint
//! URL is not the one derived from its address, which is common wherever a hostname rather than an
//! IP is configured.
//!
//! Detection only. Identification needs `GetEndpoints`, which needs `OpenSecureChannel` first —
//! algorithm negotiation, nonce exchange and sequence numbers. That is a protocol stack, not a
//! parser, and the no-OpenSSL/musl constraint compounds it.

use anyhow::Error;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext};
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Message header: 3-byte type, 1-byte chunk type, 4-byte total size.
const HEADER_LEN: usize = 8;
/// `F` — a final chunk, which every Hello and Acknowledge is.
const CHUNK_FINAL: u8 = b'F';

const MSG_HELLO: &[u8; 3] = b"HEL";
const MSG_ACK: &[u8; 3] = b"ACK";
const MSG_ERROR: &[u8; 3] = b"ERR";

/// Buffer sizes offered in the Hello. Any server accepts these; they bound what it may send back,
/// and nothing here reads a message large enough for the value to matter.
const BUFFER_SIZE: u32 = 65535;

pub struct OpcUaProbe;

#[async_trait]
impl AppProbe for OpcUaProbe {
    fn port(&self) -> PortType {
        PortType::OpcUa
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::OpcUa)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let port = self.port().number();
        let addr = std::net::SocketAddr::new(ctx.ip, port);

        let mut stream = match timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                ctx.note_connect_error(&e);
                return Ok(AppProbeOutcome::NoAnswer);
            }
            Err(_) => return Ok(AppProbeOutcome::NoAnswer),
        };

        let endpoint_url = format!("opc.tcp://{}:{}", ctx.ip, port);
        stream.write_all(&hello(&endpoint_url)).await?;

        let mut buf = [0u8; 256];
        let read = timeout(SCAN_TIMEOUT, stream.read(&mut buf)).await??;

        Ok(if is_opc_ua_reply(&buf[..read]) {
            // No identity: the Acknowledge carries buffer sizes and a protocol version, nothing
            // about the device.
            AppProbeOutcome::Answered { identity: None }
        } else {
            AppProbeOutcome::NoAnswer
        })
    }
}

/// A Hello message. Everything after the header is little-endian, and the endpoint URL is an OPC
/// UA string: a signed 32-bit length followed by UTF-8 bytes.
fn hello(endpoint_url: &str) -> Vec<u8> {
    let url = endpoint_url.as_bytes();
    let size = (HEADER_LEN + 20 + 4 + url.len()) as u32;

    let mut msg = Vec::with_capacity(size as usize);
    msg.extend_from_slice(MSG_HELLO);
    msg.push(CHUNK_FINAL);
    msg.extend_from_slice(&size.to_le_bytes());
    msg.extend_from_slice(&0u32.to_le_bytes()); // protocol version
    msg.extend_from_slice(&BUFFER_SIZE.to_le_bytes()); // receive buffer size
    msg.extend_from_slice(&BUFFER_SIZE.to_le_bytes()); // send buffer size
    msg.extend_from_slice(&0u32.to_le_bytes()); // max message size: no limit
    msg.extend_from_slice(&0u32.to_le_bytes()); // max chunk count: no limit
    msg.extend_from_slice(&(url.len() as i32).to_le_bytes());
    msg.extend_from_slice(url);
    msg
}

/// Whether this is an OPC UA binary-transport reply to a Hello.
fn is_opc_ua_reply(reply: &[u8]) -> bool {
    if reply.len() < HEADER_LEN {
        return false;
    }
    let message_type: &[u8; 3] = &[reply[0], reply[1], reply[2]];
    if message_type != MSG_ACK && message_type != MSG_ERROR {
        return false;
    }
    if reply[3] != CHUNK_FINAL {
        return false;
    }

    // The declared size has to agree with the header's own framing, which is what stops a stray
    // "ACK" appearing at the head of some other protocol's banner from counting.
    let size = u32::from_le_bytes([reply[4], reply[5], reply[6], reply[7]]) as usize;
    (HEADER_LEN..=reply.len()).contains(&size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(message_type: &[u8; 3], body: &[u8]) -> Vec<u8> {
        let size = (HEADER_LEN + body.len()) as u32;
        let mut msg = Vec::new();
        msg.extend_from_slice(message_type);
        msg.push(CHUNK_FINAL);
        msg.extend_from_slice(&size.to_le_bytes());
        msg.extend_from_slice(body);
        msg
    }

    /// An Acknowledge body: protocol version, the two buffer sizes, max message size, max chunks.
    fn ack_body() -> Vec<u8> {
        let mut body = Vec::new();
        for value in [0u32, BUFFER_SIZE, BUFFER_SIZE, 0, 0] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        body
    }

    #[test]
    fn an_acknowledge_is_an_opc_ua_server() {
        assert!(is_opc_ua_reply(&message(MSG_ACK, &ack_body())));
    }

    /// `Bad_TcpEndpointUrlInvalid`. The server rejected our URL, which means it read it, which
    /// means it speaks OPC UA.
    #[test]
    fn an_error_reply_is_also_an_opc_ua_server() {
        let mut body = 0x8083_0000u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(-1i32).to_le_bytes()); // null reason string
        assert!(is_opc_ua_reply(&message(MSG_ERROR, &body)));
    }

    #[test]
    fn another_protocol_on_the_port_is_not_opc_ua() {
        assert!(!is_opc_ua_reply(b"HTTP/1.1 400 Bad Request\r\n\r\n"));
        assert!(!is_opc_ua_reply(b"SSH-2.0-OpenSSH_9.6\r\n"));
    }

    /// The three type bytes alone are not enough — the chunk type and a self-consistent size are
    /// what make it a frame rather than a coincidence.
    #[test]
    fn the_message_type_alone_is_not_enough() {
        let mut malformed = message(MSG_ACK, &ack_body());
        malformed[3] = b'C'; // an intermediate chunk, which an Acknowledge is never
        assert!(!is_opc_ua_reply(&malformed));

        let mut oversized = message(MSG_ACK, &ack_body());
        oversized[4..8].copy_from_slice(&9999u32.to_le_bytes());
        assert!(!is_opc_ua_reply(&oversized));
    }

    #[test]
    fn a_truncated_reply_is_not_a_frame() {
        assert!(!is_opc_ua_reply(b"ACK"));
        assert!(!is_opc_ua_reply(b""));
    }

    /// The Hello has to be well formed or no server answers it. Structural, not a byte-array
    /// comparison: the size field agreeing with the message, and the URL length agreeing with the
    /// URL that follows it.
    #[test]
    fn the_hello_is_a_well_formed_frame_carrying_the_endpoint_url() {
        let url = "opc.tcp://192.0.2.10:4840";
        let msg = hello(url);

        assert_eq!(&msg[0..3], MSG_HELLO);
        assert_eq!(msg[3], CHUNK_FINAL);
        assert_eq!(
            u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize,
            msg.len(),
            "the size field counts the whole message including the header"
        );

        let url_len = i32::from_le_bytes([msg[28], msg[29], msg[30], msg[31]]) as usize;
        assert_eq!(url_len, url.len());
        assert_eq!(&msg[32..], url.as_bytes());
    }
}
