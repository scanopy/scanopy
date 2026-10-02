//! SMB detection with an SMB2 NEGOTIATE, and the Windows version from the NTLMSSP challenge.
//!
//! NEGOTIATE is the first message of an SMB2 conversation and precedes SESSION_SETUP, so it needs no
//! credentials. The reply carries the protocol's own magic (`0xFE S M B` for SMB2 and 3, `0xFF S M
//! B` for the SMB1 servers that still answer), and both are accepted, because the definition claims
//! a file server rather than a dialect.
//!
//! On an SMB2 reply the same connection sends a SESSION_SETUP carrying an NTLMSSP NEGOTIATE that asks
//! for the server's version. The CHALLENGE that comes back states it (major, minor, build) before any
//! credential is involved, which is how `smb-os-discovery` reads it. Windows fills it with its own
//! release; Samba fills a build of 0, which writes no OS.

use anyhow::Error;
use async_trait::async_trait;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, DeviceIdentity, ProbeContext};
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::hosts::r#impl::attributes::HostOsValue;
use crate::server::hosts::r#impl::os::{HostOs, HostOsFamily};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// SMB2/3 protocol id.
const SMB2_MAGIC: [u8; 4] = [0xFE, b'S', b'M', b'B'];
/// SMB1 protocol id, still sent by servers that answer an SMB2 negotiate with a downgrade.
const SMB1_MAGIC: [u8; 4] = [0xFF, b'S', b'M', b'B'];
/// A NetBIOS session-service header: message type and a 3-byte length.
const NETBIOS_HEADER_LEN: usize = 4;

/// An SMB2 NEGOTIATE offering the two dialects every server understands.
fn negotiate_request() -> Vec<u8> {
    let mut header = vec![0u8; 64];
    header[..4].copy_from_slice(&SMB2_MAGIC);
    header[4] = 64; // StructureSize, fixed by the specification
    header[14] = 1; // CreditRequest
    // Command (offset 12) is 0 for NEGOTIATE, and everything else stays zero.

    let mut body = vec![0u8; 36];
    body[0] = 36; // StructureSize, fixed
    body[2] = 2; // DialectCount
    body[4] = 1; // SecurityMode: signing enabled
    // ClientGuid stays zero: servers do not require a real one to negotiate.

    let mut message = header;
    message.extend_from_slice(&body);
    message.extend_from_slice(&0x0202u16.to_le_bytes()); // SMB 2.0.2
    message.extend_from_slice(&0x0210u16.to_le_bytes()); // SMB 2.1

    let length = message.len();
    let mut packet = vec![
        0x00,
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
    ];
    packet.extend_from_slice(&message);
    packet
}

pub struct SmbProbe;

#[async_trait]
impl AppProbe for SmbProbe {
    fn port(&self) -> PortType {
        PortType::Samba
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Smb)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let addr = std::net::SocketAddr::new(ctx.ip, self.port().number());
        let mut stream = match tokio::time::timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                ctx.note_connect_error(&e);
                return Ok(AppProbeOutcome::NoAnswer);
            }
            Err(_) => return Ok(AppProbeOutcome::NoAnswer),
        };

        let negotiated = exchange(&mut stream, &negotiate_request()).await;
        let Some(dialect) = smb_dialect(&negotiated) else {
            return Ok(AppProbeOutcome::NoAnswer);
        };
        // Only SMB2 continues on to SESSION_SETUP; an SMB1 server is still a file server.
        let os = match dialect {
            SmbDialect::Smb2 => {
                windows_version(&exchange(&mut stream, &session_setup_request()).await)
            }
            SmbDialect::Smb1 => None,
        };
        let identity = os.map(|os| DeviceIdentity {
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            os: Some(Box::new(Attributed::new(
                HostOsValue(os),
                AttributeSource::Probe(ClientProbe::Smb),
            ))),
        });
        Ok(AppProbeOutcome::Answered { identity })
    }
}

/// Which SMB the reply speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmbDialect {
    Smb1,
    Smb2,
}

/// The SMB protocol id after the reply's NetBIOS header, if it carries one.
fn smb_dialect(bytes: &[u8]) -> Option<SmbDialect> {
    match bytes.get(NETBIOS_HEADER_LEN..NETBIOS_HEADER_LEN + 4)? {
        magic if magic == SMB2_MAGIC => Some(SmbDialect::Smb2),
        magic if magic == SMB1_MAGIC => Some(SmbDialect::Smb1),
        _ => None,
    }
}

/// Largest SMB message read. A NEGOTIATE or SESSION_SETUP reply is a few hundred bytes.
const MAX_REPLY_LEN: usize = 16 * 1024;

/// Send one NetBIOS-framed message and read one framed reply, within [`SCAN_TIMEOUT`] overall. An
/// empty result means nothing usable arrived.
async fn exchange(stream: &mut TcpStream, request: &[u8]) -> Vec<u8> {
    let io = async {
        stream.write_all(request).await.ok()?;
        let mut header = [0u8; NETBIOS_HEADER_LEN];
        stream.read_exact(&mut header).await.ok()?;
        let length =
            usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
        if length > MAX_REPLY_LEN {
            return None;
        }
        let mut reply = header.to_vec();
        reply.resize(NETBIOS_HEADER_LEN + length, 0);
        stream
            .read_exact(&mut reply[NETBIOS_HEADER_LEN..])
            .await
            .ok()?;
        Some(reply)
    };
    tokio::time::timeout(SCAN_TIMEOUT, io)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// NTLMSSP negotiate flags: Unicode, request target, NTLM, always sign, extended session security,
/// version, 128- and 56-bit. `NEGOTIATE_VERSION` (0x02000000) is the one that asks the server to
/// state its version.
const NTLMSSP_NEGOTIATE_FLAGS: u32 = 0xA208_8205;
const NTLMSSP_SIGNATURE: &[u8; 8] = b"NTLMSSP\0";
/// The NTLMSSP flag a CHALLENGE sets when its version field is filled.
const NTLMSSP_NEGOTIATE_VERSION: u32 = 0x0200_0000;

/// An NTLMSSP NEGOTIATE message asking for the server's version (MS-NLMP 2.2.1.1).
fn ntlmssp_negotiate() -> Vec<u8> {
    let mut message = NTLMSSP_SIGNATURE.to_vec();
    message.extend_from_slice(&1u32.to_le_bytes()); // MessageType: NEGOTIATE
    message.extend_from_slice(&NTLMSSP_NEGOTIATE_FLAGS.to_le_bytes());
    message.extend_from_slice(&[0u8; 8]); // DomainNameFields: none supplied
    message.extend_from_slice(&[0u8; 8]); // WorkstationFields: none supplied
    message.extend_from_slice(&[0u8; 8]); // Version: ours is not needed
    message
}

/// A DER tag and short-form length around `content`. Every element here is under 128 bytes.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag, content.len() as u8];
    out.extend_from_slice(content);
    out
}

/// The NTLMSSP NEGOTIATE wrapped in a SPNEGO NegTokenInit (RFC 4178), which is what an SMB2
/// SESSION_SETUP security buffer carries.
fn spnego_negotiate() -> Vec<u8> {
    const SPNEGO_OID: [u8; 6] = [0x2b, 0x06, 0x01, 0x05, 0x05, 0x02];
    const NTLMSSP_OID: [u8; 10] = [0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x02, 0x02, 0x0a];
    let mech_types = der(0xa0, &der(0x30, &der(0x06, &NTLMSSP_OID)));
    let mech_token = der(0xa2, &der(0x04, &ntlmssp_negotiate()));
    let neg_token_init = der(0xa0, &der(0x30, &[mech_types, mech_token].concat()));
    der(0x60, &[der(0x06, &SPNEGO_OID), neg_token_init].concat())
}

/// An SMB2 SESSION_SETUP carrying [`spnego_negotiate`] (MS-SMB2 2.2.5).
fn session_setup_request() -> Vec<u8> {
    const HEADER_LEN: usize = 64;
    const BODY_LEN: usize = 24;
    let security = spnego_negotiate();

    let mut header = vec![0u8; HEADER_LEN];
    header[..4].copy_from_slice(&SMB2_MAGIC);
    header[4] = 64; // StructureSize
    header[12] = 1; // Command: SESSION_SETUP
    header[14] = 1; // CreditRequest
    header[24] = 1; // MessageId: the second message on this connection

    let mut body = vec![0u8; BODY_LEN];
    body[0] = 25; // StructureSize, fixed by the specification (the buffer counts as one byte)
    body[3] = 1; // SecurityMode: signing enabled
    body[12..14].copy_from_slice(&((HEADER_LEN + BODY_LEN) as u16).to_le_bytes()); // offset
    body[14..16].copy_from_slice(&(security.len() as u16).to_le_bytes()); // length

    let mut message = header;
    message.extend_from_slice(&body);
    message.extend_from_slice(&security);

    let length = message.len();
    let mut packet = vec![
        0x00,
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
    ];
    packet.extend_from_slice(&message);
    packet
}

/// The Windows release an NTLMSSP CHALLENGE in `reply` states, if it states one.
///
/// Found by its signature rather than by walking the SPNEGO wrapper: the CHALLENGE is the only
/// NTLMSSP message in the reply, and its layout is fixed (MS-NLMP 2.2.1.2). The version sits at
/// offset 48 and is meaningful only when the server set `NEGOTIATE_VERSION`. A build of 0 is not
/// a Windows release; Samba sends it.
fn windows_version(reply: &[u8]) -> Option<HostOs> {
    let start = reply
        .windows(NTLMSSP_SIGNATURE.len())
        .position(|w| w == NTLMSSP_SIGNATURE)?;
    let challenge = &reply[start..];
    let message_type = u32::from_le_bytes(challenge.get(8..12)?.try_into().ok()?);
    if message_type != 2 {
        return None;
    }
    let flags = u32::from_le_bytes(challenge.get(20..24)?.try_into().ok()?);
    if flags & NTLMSSP_NEGOTIATE_VERSION == 0 {
        return None;
    }
    let version = challenge.get(48..56)?;
    let (major, minor) = (version[0], version[1]);
    let build = u16::from_le_bytes([version[2], version[3]]);
    if build == 0 {
        return None;
    }
    Some(HostOs {
        version: Some(format!("{major}.{minor}.{build}")),
        ..HostOs::family_only(HostOsFamily::Windows)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(magic: [u8; 4]) -> Vec<u8> {
        let mut packet = vec![0x00, 0x00, 0x00, 0x40];
        packet.extend_from_slice(&magic);
        packet.extend_from_slice(&[0u8; 60]);
        packet
    }

    #[test]
    fn an_smb2_negotiate_response_is_smb() {
        assert_eq!(smb_dialect(&framed(SMB2_MAGIC)), Some(SmbDialect::Smb2));
    }

    /// A server that only speaks SMB1 answers with its own protocol id, and is still a file server.
    #[test]
    fn an_smb1_response_is_smb() {
        assert_eq!(smb_dialect(&framed(SMB1_MAGIC)), Some(SmbDialect::Smb1));
    }

    #[test]
    fn silence_or_another_protocol_is_not_smb() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // The magic has to sit after the NetBIOS header, not at the very start.
            &[0xFE, b'S', b'M', b'B', 0, 0, 0, 0][..],
            &[0x00, 0x00, 0x00, 0x40][..],
        ] {
            assert_eq!(smb_dialect(reply), None, "{reply:?}");
        }
    }

    #[test]
    fn the_negotiate_request_declares_its_length_and_offers_dialects() {
        let packet = negotiate_request();
        let declared =
            usize::from(packet[1]) << 16 | usize::from(packet[2]) << 8 | usize::from(packet[3]);
        assert_eq!(declared, packet.len() - NETBIOS_HEADER_LEN);
        assert_eq!(&packet[4..8], &SMB2_MAGIC);
        assert_eq!(
            packet[NETBIOS_HEADER_LEN + 64 + 2],
            2,
            "two dialects offered"
        );
    }

    /// An NTLMSSP CHALLENGE as MS-NLMP 2.2.1.2 lays it out, inside a little surrounding noise the
    /// way an SPNEGO wrapper surrounds it.
    fn challenge(flags: u32, version: [u8; 8]) -> Vec<u8> {
        let mut reply = vec![0xa1, 0x81, 0x00, 0x30];
        reply.extend_from_slice(NTLMSSP_SIGNATURE);
        reply.extend_from_slice(&2u32.to_le_bytes()); // MessageType: CHALLENGE
        reply.extend_from_slice(&[0u8; 8]); // TargetNameFields
        reply.extend_from_slice(&flags.to_le_bytes());
        reply.extend_from_slice(&[0x11; 8]); // ServerChallenge
        reply.extend_from_slice(&[0u8; 8]); // Reserved
        reply.extend_from_slice(&[0u8; 8]); // TargetInfoFields
        reply.extend_from_slice(&version);
        reply
    }

    #[test]
    fn a_windows_challenge_states_its_release() {
        // Windows Server 2022: 10.0, build 20348 (0x4F7C).
        let os = windows_version(&challenge(
            NTLMSSP_NEGOTIATE_FLAGS,
            [10, 0, 0x7C, 0x4F, 0, 0, 0, 0x0F],
        ))
        .expect("a filled version names Windows");
        assert_eq!(os.family, HostOsFamily::Windows);
        assert_eq!(os.version.as_deref(), Some("10.0.20348"));
    }

    #[test]
    fn samba_or_a_challenge_without_a_version_names_no_os() {
        assert_eq!(
            windows_version(&challenge(
                NTLMSSP_NEGOTIATE_FLAGS,
                [6, 1, 0, 0, 0, 0, 0, 0x0F]
            )),
            None
        );
        assert_eq!(
            windows_version(&challenge(
                NTLMSSP_NEGOTIATE_FLAGS & !NTLMSSP_NEGOTIATE_VERSION,
                [10, 0, 0x7C, 0x4F, 0, 0, 0, 0x0F],
            )),
            None
        );
        assert_eq!(windows_version(b"no ntlmssp here"), None);
    }

    #[test]
    fn the_session_setup_declares_its_length_and_points_at_its_security_buffer() {
        let packet = session_setup_request();
        let declared =
            usize::from(packet[1]) << 16 | usize::from(packet[2]) << 8 | usize::from(packet[3]);
        assert_eq!(declared, packet.len() - NETBIOS_HEADER_LEN);
        let body = &packet[NETBIOS_HEADER_LEN + 64..];
        let offset = usize::from(u16::from_le_bytes([body[12], body[13]]));
        let length = usize::from(u16::from_le_bytes([body[14], body[15]]));
        let security = &packet[NETBIOS_HEADER_LEN + offset..];
        assert_eq!(security.len(), length);
        assert_eq!(security[0], 0x60, "a GSS-API token");
        assert!(security.windows(8).any(|w| w == NTLMSSP_SIGNATURE));
    }
}
