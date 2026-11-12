//! A simulated device answering SNMP over a real UDP socket: USM for a v3 device, a community
//! for the rest.
//!
//! [`super::transport::SimAgent`] stands in for snmpd below the session: it answers walks, and
//! never sees a protocol version, a security level, a key or a digest. That leaves the session
//! itself — the version a community credential lands on, the level a v3 credential asks for, the
//! hash it authenticates with, the cipher it encrypts with — reachable only on the lab VM. This
//! puts the same device data behind a front end that checks them, so a daemon test can open a real
//! [`create_session`] or run the real probe against it and fail when either is built wrong.
//!
//! Message security is snmp2's own, the library the daemon uses, so a defect shared by both ends
//! of snmp2 passes here. The lab VM (`make snmp-verify`, net-snmp) is the independent check.
//! What this does hold is everything Scanopy decides: which version a community credential is
//! tried over, which level a v3 credential maps to, which fields it carries, and that a request
//! over a version the device refuses, at the wrong level or with the wrong key is not answered.
//! Silence for a wrong privacy key is what net-snmp 5.9.4 does on the lab VM: it answers engine
//! discovery, then drops the encrypted request it cannot read (GH #846).
//!
//! [`create_session`]: crate::daemon::discovery::integration::snmp::session::create_session

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use secrecy::ExposeSecret;
use snmp2::v3::{Auth, Security};
use snmp2::{AsnReader, MessageType, Pdu, Value, Varbinds, Version};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

use super::transport::SimAgent;
use super::{CommunityVersions, SimDevice};
use crate::daemon::discovery::integration::snmp::oids::{oid_parts, system};
use crate::daemon::discovery::integration::snmp::session::{
    SnmpContext, SnmpSession, create_session,
};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
use crate::server::credentials::r#impl::types::{CredentialType, SecretValue, SnmpV3Security};

/// The agent's engine id: enterprise 8072 (net-snmp) in the RFC 3411 text format, then a name.
const ENGINE_ID: &[u8] = b"\x80\x00\x1f\x88\x04scanopy-sim";
/// `usmStatsUnknownEngineIDs.0`, the counter an agent reports in answer to engine discovery.
const USM_STATS_UNKNOWN_ENGINE_IDS: &str = "1.3.6.1.6.3.15.1.1.4.0";
/// `usmStatsNotInTimeWindows.0`, the counter an agent reports for an authenticated request whose
/// boots or time is outside its window.
const USM_STATS_NOT_IN_TIME_WINDOWS: &str = "1.3.6.1.6.3.15.1.1.2.0";
/// RFC 3414 §2.2.3: how far a request's time may stray from the agent's.
const TIME_WINDOW: i64 = 150;

/// The agent's snmpEngineBoots and snmpEngineTime. Frozen: no test runs long enough to need the
/// clock to move.
#[derive(Clone, Copy)]
pub struct EngineClock {
    pub boots: i64,
    pub time: i64,
    /// Whether the discovery Report carries `boots` and `time`, as net-snmp's does, or zeroes,
    /// which RFC 3414 §4 permits. Without them the first authenticated request is out of window.
    pub discovery_carries_clock: bool,
}

/// What every lab device runs on.
const LAB_CLOCK: EngineClock = EngineClock {
    boots: 1,
    time: 1000,
    discovery_carries_clock: true,
};

/// How long every answer takes to come back.
///
/// GH #848's switch answered each request in 5 to 10ms, and that gap is what the defect needs: an
/// answer still on its way when the client sends its next request is read by that request. On
/// loopback an answer arrives before the client has built the next request, so the session's
/// pre-send drain removes it and the fault never shows.
const ANSWER_LATENCY: Duration = Duration::from_millis(5);

/// A running agent. Stops when dropped.
pub struct UdpAgent {
    addr: SocketAddr,
    task: JoinHandle<()>,
    discoveries: Arc<AtomicUsize>,
    slow_rows_served: Arc<AtomicUsize>,
}

impl Drop for UdpAgent {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl UdpAgent {
    /// Serve `device`'s data on a loopback port, to its own credential.
    pub async fn start(device: &SimDevice) -> Self {
        Self::start_with_clock(device, LAB_CLOCK).await
    }

    /// Serve `device` from an engine whose clock reads `clock`.
    pub async fn start_with_clock(device: &SimDevice, clock: EngineClock) -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind a loopback port");
        let addr = socket.local_addr().expect("a bound socket has an address");
        let front = match &device.credential {
            CredentialType::SnmpCommunity { community } => Front::Community {
                community: exposed(community).into_bytes(),
                versions: device.community_versions,
            },
            CredentialType::SnmpV3 { .. } => Front::Usm(Box::new(agent_security(device, clock))),
            other => panic!("{} has a non-SNMP credential: {other:?}", device.name),
        };
        let slow_rows_served = Arc::new(AtomicUsize::new(0));
        let responder = Responder {
            front,
            clock,
            agent: device.agent(),
            device: device.clone(),
            slow_rows_served: slow_rows_served.clone(),
        };
        let discoveries = Arc::new(AtomicUsize::new(0));
        let counted = discoveries.clone();
        let task = tokio::spawn(async move {
            let mut buf = vec![0u8; 65_535];
            // One request at a time, the next one waiting in the socket: GH #848's capture shows
            // the retry answered only after the slow answer it was sent behind.
            while let Ok((len, peer)) = socket.recv_from(&mut buf).await {
                if request_engine(&buf[..len]).is_some_and(|(engine_id, ..)| engine_id.is_empty()) {
                    counted.fetch_add(1, Ordering::Relaxed);
                }
                let slow_before = responder.slow_rows_served.load(Ordering::Relaxed);
                if let Some(reply) = responder.respond(&buf[..len]) {
                    let slow_rows =
                        responder.slow_rows_served.load(Ordering::Relaxed) - slow_before;
                    tokio::time::sleep(
                        ANSWER_LATENCY + responder.agent.row_cost() * slow_rows as u32,
                    )
                    .await;
                    let _ = socket.send_to(&reply, peer).await;
                }
            }
        });
        Self {
            addr,
            task,
            discoveries,
            slow_rows_served,
        }
    }

    /// How many rows the device was slow to answer have gone out. Zero on a scan that never asked
    /// for a page reaching them.
    pub fn slow_rows_served(&self) -> usize {
        self.slow_rows_served.load(Ordering::Relaxed)
    }

    /// A session to this agent holding `credential`, opened the way a collection opens one.
    pub async fn session(&self, credential: &CredentialType) -> SnmpSession {
        let CredentialQueryPayload::Snmp(snmp) = credential
            .to_query_payload(None)
            .expect("an SNMP credential has a query payload")
        else {
            panic!("not an SNMP credential");
        };
        create_session(self.ip(), &snmp, self.port(), SnmpContext::Default, None)
            .await
            .expect("the agent answers its own credential")
    }

    /// How many engine-discovery requests the agent has received: one per v3 handshake a client
    /// started. A client that tries a second key extension starts a second handshake.
    pub fn discoveries(&self) -> usize {
        self.discoveries.load(Ordering::Relaxed)
    }

    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }
}

/// The USM user the device is configured with, the way snmpd's `createUser` line sets it up.
///
/// Its own mapping from level to `Auth` rather than the daemon's, so a credential the daemon maps
/// to the wrong level fails against this.
fn agent_security(device: &SimDevice, clock: EngineClock) -> Security {
    let CredentialType::SnmpV3 {
        security_name,
        security,
        ..
    } = &device.credential
    else {
        panic!("{} has no SNMPv3 credential to serve", device.name);
    };
    let name = security_name.as_bytes();
    let user = match security {
        SnmpV3Security::NoAuthNoPriv => Security::new(name, b"").with_auth(Auth::NoAuthNoPriv),
        SnmpV3Security::AuthNoPriv {
            auth_protocol,
            auth_password,
        } => Security::new(name, exposed(auth_password).as_bytes())
            .with_auth_protocol((*auth_protocol).into())
            .with_auth(Auth::AuthNoPriv),
        SnmpV3Security::AuthPriv {
            auth_protocol,
            auth_password,
            priv_protocol,
            priv_password,
        } => Security::new(name, exposed(auth_password).as_bytes())
            .with_auth_protocol((*auth_protocol).into())
            .with_auth(Auth::AuthPriv {
                cipher: (*priv_protocol).into(),
                privacy_password: exposed(priv_password).into_bytes(),
            })
            .with_key_extension_method(device.agent_key_extension()),
    };
    user.with_engine_id(ENGINE_ID)
        .expect("localize the device's keys")
        .with_engine_boots_and_time(clock.boots, clock.time)
}

fn exposed(secret: &SecretValue) -> String {
    match secret {
        SecretValue::Inline { value } => value.expose_secret().to_string(),
        SecretValue::FilePath { .. } => panic!("a simulated device's credential must be inline"),
    }
}

/// A user with no keys, for the exchange that happens before any key can be localized.
fn discovery_security(boots: i64, time: i64) -> Security {
    Security::new(b"", b"")
        .with_auth(Auth::NoAuthNoPriv)
        .with_engine_id(ENGINE_ID)
        .expect("a keyless user needs no localization")
        .with_engine_boots_and_time(boots, time)
}

/// What a request asked for, copied out of the parsed PDU, which borrows the security it was
/// decrypted with.
struct Request {
    message_type: MessageType,
    oids: Vec<Vec<u64>>,
    /// GETBULK's max-repetitions, which travels in the error-index slot.
    max_repetitions: u32,
}

/// How the device authenticates a request.
enum Front {
    /// The device's v3 user. Boxed: `Security` is ten times the size of the community variant.
    Usm(Box<Security>),
    /// The device's community, over the versions it answers.
    Community {
        community: Vec<u8>,
        versions: CommunityVersions,
    },
}

struct Responder {
    front: Front,
    clock: EngineClock,
    agent: SimAgent,
    device: SimDevice,
    slow_rows_served: Arc<AtomicUsize>,
}

impl Responder {
    /// The reply to one datagram, or `None` where snmpd would stay silent: a request over a
    /// version the device refuses, with the wrong community, one it cannot authenticate or
    /// decrypt, or one at a level the user is not configured for.
    fn respond(&self, datagram: &[u8]) -> Option<Vec<u8>> {
        match &self.front {
            Front::Usm(security) => self.respond_usm(security, datagram),
            Front::Community {
                community,
                versions,
            } => self.respond_community(community, *versions, datagram),
        }
    }

    fn respond_community(
        &self,
        community: &[u8],
        versions: CommunityVersions,
        datagram: &[u8],
    ) -> Option<Vec<u8>> {
        let pdu = Pdu::from_bytes(datagram).ok()?;
        let answers = match pdu.version().ok()? {
            Version::V1 => versions != CommunityVersions::V2cOnly,
            Version::V2C => versions != CommunityVersions::V1Only,
            _ => false,
        };
        if !answers || pdu.community != community {
            return None;
        }
        let request = Request {
            message_type: pdu.message_type,
            oids: pdu.varbinds.clone().map(|(oid, _)| oid_vec(&oid)).collect(),
            max_repetitions: pdu.error_index,
        };
        let varbinds = encode_varbinds(&self.answer(&request));

        // The reply reuses the request's own PDU, which carries its version, community and id.
        let mut reply = pdu;
        reply.message_type = MessageType::Response;
        reply.error_status = 0;
        reply.error_index = 0;
        reply.varbinds = Varbinds::from_bytes(&varbinds);
        reply.to_bytes().ok()
    }

    fn respond_usm(&self, user: &Security, datagram: &[u8]) -> Option<Vec<u8>> {
        let (engine_id, boots, time) = request_engine(datagram)?;
        if engine_id.is_empty() {
            return self.discovery_report(datagram);
        }

        // Parsing checks the digest and decrypts in place, zeroing the digest as it goes, so each
        // parse gets a fresh copy of the datagram and of the user. The user is set to the
        // request's own clock: snmp2 judges timeliness as a manager would, and the agent's
        // judgement (RFC 3414 §3.2 step 7a) is made below instead.
        let request = {
            let bytes = datagram.to_vec();
            let mut security = user.clone().with_engine_boots_and_time(boots, time);
            let pdu = Pdu::from_bytes_with_security(&bytes, Some(&mut security)).ok()?;
            Request {
                message_type: pdu.message_type,
                oids: pdu.varbinds.clone().map(|(oid, _)| oid_vec(&oid)).collect(),
                max_repetitions: pdu.error_index,
            }
        };

        // Only an authenticated request is held to the time window.
        let authenticated = !matches!(
            self.device.credential,
            CredentialType::SnmpV3 {
                security: SnmpV3Security::NoAuthNoPriv,
                ..
            }
        );
        let in_window = !authenticated
            || (boots == self.clock.boots
                && boots < i64::from(i32::MAX)
                && (time - self.clock.time).abs() <= TIME_WINDOW);
        let (message_type, varbinds) = if in_window {
            (
                MessageType::Response,
                encode_varbinds(&self.answer(&request)),
            )
        } else {
            (
                MessageType::Report,
                encode_varbinds(&[(
                    oid_parts(USM_STATS_NOT_IN_TIME_WINDOWS),
                    tlv(0x41, &unsigned(1)),
                )]),
            )
        };

        // The reply reuses the request's own PDU, which carries its request id and the v3 version
        // `Pdu` will not let a caller set.
        let bytes = datagram.to_vec();
        let mut security = user.clone().with_engine_boots_and_time(boots, time);
        let mut reply = Pdu::from_bytes_with_security(&bytes, Some(&mut security)).ok()?;
        reply.message_type = message_type;
        reply.error_status = 0;
        reply.error_index = 0;
        reply.varbinds = Varbinds::from_bytes(&varbinds);
        reply.to_bytes_with_security(Some(user)).ok()
    }

    /// RFC 3414 §4: an unauthenticated Report carrying this engine's id, and its boots and time
    /// unless the clock says to leave them zero.
    fn discovery_report(&self, datagram: &[u8]) -> Option<Vec<u8>> {
        let varbinds = encode_varbinds(&[(
            oid_parts(USM_STATS_UNKNOWN_ENGINE_IDS),
            tlv(0x41, &unsigned(1)),
        )]);
        let bytes = datagram.to_vec();
        let mut security = discovery_security(0, 0);
        let mut report = Pdu::from_bytes_with_security(&bytes, Some(&mut security)).ok()?;
        report.message_type = MessageType::Report;
        report.varbinds = Varbinds::from_bytes(&varbinds);
        let EngineClock {
            boots,
            time,
            discovery_carries_clock,
        } = self.clock;
        let (boots, time) = if discovery_carries_clock {
            (boots, time)
        } else {
            (0, 0)
        };
        report
            .to_bytes_with_security(Some(&discovery_security(boots, time)))
            .ok()
    }

    /// Each varbind's answer, as an encoded value.
    fn answer(&self, request: &Request) -> Vec<(Vec<u64>, Vec<u8>)> {
        match request.message_type {
            MessageType::GetRequest => request
                .oids
                .iter()
                .map(|oid| (oid.clone(), self.scalar(oid)))
                .collect(),
            MessageType::GetNextRequest => request
                .oids
                .iter()
                .flat_map(|oid| self.page(oid, 1))
                .collect(),
            // The daemon walks one column per request, so every varbind repeats; the rows come
            // column by column rather than interleaved, which no caller here can tell apart.
            MessageType::GetBulkRequest => request
                .oids
                .iter()
                .flat_map(|oid| self.page(oid, request.max_repetitions))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn page(&self, from: &[u64], max_repetitions: u32) -> Vec<(Vec<u64>, Vec<u8>)> {
        let page = self.agent.page(from, max_repetitions);
        self.slow_rows_served
            .fetch_add(self.agent.slow_rows_on(&page) as usize, Ordering::Relaxed);
        page.into_iter()
            .map(|(oid, value)| (oid, value_tlv(&value)))
            .collect()
    }

    /// The system group, which snmpd serves from its own configuration rather than a `pass` file.
    fn scalar(&self, oid: &[u64]) -> Vec<u8> {
        let system = &self.device.system;
        let text = |value: &Option<String>| value.as_deref().map(|v| tlv(0x04, v.as_bytes()));
        let found = if oid == oid_parts(system::SYS_NAME) {
            text(&system.sys_name)
        } else if oid == oid_parts(system::SYS_DESCR) {
            text(&system.sys_descr)
        } else if oid == oid_parts(system::SYS_LOCATION) {
            text(&system.sys_location)
        } else if oid == oid_parts(system::SYS_CONTACT) {
            text(&system.sys_contact)
        } else if oid == oid_parts(system::SYS_OBJECT_ID) {
            system
                .sys_object_id
                .as_deref()
                .map(|id| tlv(0x06, &oid_content(&oid_parts(id))))
        } else if oid == oid_parts(system::SYS_SERVICES) {
            system
                .sys_services
                .map(|services| tlv(0x02, &signed(i64::from(services))))
        } else {
            // A scalar the device serves from a `pass` file, such as `lldpLocChassisId.0`: the
            // row at exactly this OID, which is the first one after the object it is an
            // instance of.
            oid.split_last().and_then(|(_, object)| {
                self.agent
                    .page(object, 1)
                    .into_iter()
                    .find(|(row, _)| row.as_slice() == oid)
                    .map(|(_, value)| value_tlv(&value))
            })
        };
        // noSuchObject
        found.unwrap_or_else(|| tlv(0x80, &[]))
    }
}

/// `msgAuthoritativeEngineID`, `msgAuthoritativeEngineBoots` and `msgAuthoritativeEngineTime`
/// from a v3 message, without checking anything else about it. The engine id is empty on the
/// discovery request, which is how an agent tells one apart.
fn request_engine(datagram: &[u8]) -> Option<(&[u8], i64, i64)> {
    use snmp2::asn1::TYPE_SEQUENCE;

    let message = AsnReader::from_bytes(datagram)
        .read_raw(TYPE_SEQUENCE)
        .ok()?;
    let mut message = AsnReader::from_bytes(message);
    message.read_asn_integer().ok()?; // msgVersion
    message.read_raw(TYPE_SEQUENCE).ok()?; // msgGlobalData
    let params = message.read_asn_octetstring().ok()?;
    let params = AsnReader::from_bytes(params).read_raw(TYPE_SEQUENCE).ok()?;
    let mut params = AsnReader::from_bytes(params);
    let engine_id = params.read_asn_octetstring().ok()?;
    let boots = params.read_asn_integer().ok()?;
    let time = params.read_asn_integer().ok()?;
    Some((engine_id, boots, time))
}

fn oid_vec(oid: &snmp2::Oid) -> Vec<u64> {
    oid.iter().map(|arcs| arcs.collect()).unwrap_or_default()
}

// ============================================================================
// BER, for the varbinds of a reply. snmp2's encoder is crate-private; its decoder is what reads
// these back, on both ends of the session.
// ============================================================================

pub(crate) fn encode_varbinds(varbinds: &[(Vec<u64>, Vec<u8>)]) -> Vec<u8> {
    varbinds
        .iter()
        .flat_map(|(oid, value)| {
            tlv(
                0x30,
                &[tlv(0x06, &oid_content(oid)), value.clone()].concat(),
            )
        })
        .collect()
}

fn value_tlv(value: &Value) -> Vec<u8> {
    match value {
        Value::Integer(n) => tlv(0x02, &signed(*n)),
        Value::OctetString(bytes) => tlv(0x04, bytes),
        Value::Null => tlv(0x05, &[]),
        Value::ObjectIdentifier(oid) => tlv(0x06, oid.as_bytes()),
        Value::IpAddress(ip) => tlv(0x40, ip),
        Value::Counter32(n) => tlv(0x41, &unsigned(u64::from(*n))),
        Value::Unsigned32(n) => tlv(0x42, &unsigned(u64::from(*n))),
        Value::Timeticks(n) => tlv(0x43, &unsigned(u64::from(*n))),
        Value::Opaque(bytes) => tlv(0x44, bytes),
        Value::Counter64(n) => tlv(0x46, &unsigned(*n)),
        Value::NoSuchObject => tlv(0x80, &[]),
        Value::NoSuchInstance => tlv(0x81, &[]),
        Value::EndOfMibView => tlv(0x82, &[]),
        other => panic!("a simulated device serves no {other:?}"),
    }
}

pub(crate) fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = content.len();
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes = len.to_be_bytes();
        let first = bytes
            .iter()
            .position(|b| *b != 0)
            .unwrap_or(bytes.len() - 1);
        out.push(0x80 | (bytes.len() - first) as u8);
        out.extend_from_slice(&bytes[first..]);
    }
    out.extend_from_slice(content);
    out
}

/// Two's complement, shortest form.
fn signed(n: i64) -> Vec<u8> {
    let bytes = n.to_be_bytes();
    let mut start = 0;
    while start < 7
        && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }
    bytes[start..].to_vec()
}

/// Unsigned, shortest form, with a leading zero where the top bit would read as a sign.
fn unsigned(n: u64) -> Vec<u8> {
    let bytes = n.to_be_bytes();
    let first = bytes.iter().position(|b| *b != 0).unwrap_or(7);
    let mut out = bytes[first..].to_vec();
    if out[0] & 0x80 != 0 {
        out.insert(0, 0);
    }
    out
}

fn oid_content(arcs: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    let (first, rest) = match arcs {
        [a, b, rest @ ..] => (a * 40 + b, rest),
        [a] => (a * 40, &[][..]),
        [] => return out,
    };
    for arc in std::iter::once(first).chain(rest.iter().copied()) {
        let mut chunk = vec![(arc & 0x7f) as u8];
        let mut rest = arc >> 7;
        while rest > 0 {
            chunk.push(((rest & 0x7f) as u8) | 0x80);
            rest >>= 7;
        }
        out.extend(chunk.iter().rev());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::snmp::queries::SnmpWalkTransport as _;
    use crate::daemon::discovery::integration::snmp::sim;
    use crate::server::credentials::r#impl::types::{SnmpV3AuthProtocol, SnmpV3PrivProtocol};

    /// What a daemon holding `credential` reads as the agent's `sysName`, or the error it got.
    ///
    /// A short deadline on the read: an agent that will not answer a request leaves the client
    /// waiting out its own timeout, and the test only needs to know nothing came back.
    async fn sys_name(agent: &UdpAgent, credential: &CredentialType) -> anyhow::Result<String> {
        let CredentialQueryPayload::Snmp(snmp) = credential.to_query_payload(None).unwrap() else {
            panic!("not an SNMP credential");
        };
        let silence = |_| anyhow::anyhow!("the agent did not answer");
        // Session creation authenticates too (RFC 3414 §4's second step), so it gets the same
        // short deadline.
        let mut session = tokio::time::timeout(
            Duration::from_secs(2),
            create_session(agent.ip(), &snmp, agent.port(), SnmpContext::Default, None),
        )
        .await
        .map_err(silence)??;
        let read = tokio::time::timeout(
            Duration::from_secs(2),
            session.get_scalar(&oid_parts(system::SYS_NAME)),
        )
        .await
        .map_err(silence)??;
        match read {
            Some(Value::OctetString(name)) => Ok(String::from_utf8_lossy(name).into_owned()),
            other => anyhow::bail!("sysName read back as {other:?}"),
        }
    }

    fn v3(security: SnmpV3Security<SecretValue>) -> CredentialType {
        CredentialType::SnmpV3 {
            security_name: "scanopyv3".into(),
            security,
            context_name: None,
        }
    }

    fn secret(value: &str) -> SecretValue {
        sim::devices::inline(value)
    }

    /// A copy of `secure-switch-01` whose one user is `credential`.
    fn switch_with(credential: CredentialType) -> SimDevice {
        let mut device = sim::device("secure-switch-01");
        device.credential = credential;
        device
    }

    /// Every v3 device in the lab answers a daemon holding the device's own credential. Derived
    /// from the device list, so a v3 device added later is held to it without being named here.
    #[tokio::test]
    async fn every_v3_lab_device_answers_its_own_credential() {
        let devices: Vec<_> = sim::lab()
            .into_iter()
            .filter(|d| matches!(d.credential, CredentialType::SnmpV3 { .. }))
            .collect();
        assert!(devices.len() >= 3, "the lab lost its v3 devices");

        for device in devices {
            let agent = UdpAgent::start(&device).await;
            let name = sys_name(&agent, &device.credential)
                .await
                .unwrap_or_else(|e| {
                    panic!("{} did not answer its own credential: {e:#}", device.name)
                });
            assert_eq!(Some(name), device.system.sys_name, "{}", device.name);
        }
    }

    /// The three levels and both protocol families, each against a user configured exactly so.
    #[tokio::test]
    async fn each_security_level_and_auth_protocol_reads_the_device() {
        let cases = [
            v3(SnmpV3Security::NoAuthNoPriv),
            v3(SnmpV3Security::AuthNoPriv {
                auth_protocol: SnmpV3AuthProtocol::Md5,
                auth_password: secret("md5authpass1"),
            }),
            v3(SnmpV3Security::AuthNoPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha256,
                auth_password: secret("shaauthpass1"),
            }),
            v3(SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Md5,
                auth_password: secret("md5authpass1"),
                priv_protocol: SnmpV3PrivProtocol::Aes128,
                priv_password: secret("md5privpass1"),
            }),
            // MD5's 16-byte key is too short for AES-256 and has to be extended.
            v3(SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Md5,
                auth_password: secret("md5authpass1"),
                priv_protocol: SnmpV3PrivProtocol::Aes256,
                priv_password: secret("md5privpass1"),
            }),
            v3(SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: secret("sha1authpass"),
                priv_protocol: SnmpV3PrivProtocol::Aes256,
                priv_password: secret("sha1privpass"),
            }),
        ];

        for credential in cases {
            let device = switch_with(credential.clone());
            let agent = UdpAgent::start(&device).await;
            let name = sys_name(&agent, &credential)
                .await
                .unwrap_or_else(|e| panic!("{credential:?} was not answered: {e:#}"));
            assert_eq!(name, "secure-switch-01", "{credential:?}");
        }
    }

    /// A credential that does not match the device's user reads nothing. Without this the tests
    /// above could pass against an agent that answered anyone.
    #[tokio::test]
    async fn a_credential_the_device_is_not_configured_for_reads_nothing() {
        let device_user = v3(SnmpV3Security::AuthNoPriv {
            auth_protocol: SnmpV3AuthProtocol::Md5,
            auth_password: secret("md5authpass1"),
        });
        let agent = UdpAgent::start(&switch_with(device_user)).await;

        let mismatches = [
            // Wrong password.
            v3(SnmpV3Security::AuthNoPriv {
                auth_protocol: SnmpV3AuthProtocol::Md5,
                auth_password: secret("not-the-password"),
            }),
            // Right password, wrong hash.
            v3(SnmpV3Security::AuthNoPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: secret("md5authpass1"),
            }),
            // Below the user's level.
            v3(SnmpV3Security::NoAuthNoPriv),
            // Above it.
            v3(SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Md5,
                auth_password: secret("md5authpass1"),
                priv_protocol: SnmpV3PrivProtocol::Aes128,
                priv_password: secret("md5privpass1"),
            }),
        ];

        for credential in mismatches {
            assert!(
                sys_name(&agent, &credential).await.is_err(),
                "{credential:?} read a device configured for another user"
            );
        }
    }

    /// GH #813: MikroTik RouterOS reports snmpEngineBoots = 0, which RFC 3414 §2.2.1 allows. The
    /// reporter's user: AuthPriv, SHA1 and AES128. Both through the liveness probe, which is
    /// where the reporter's discovery stopped, and through a collection's session.
    ///
    /// With and without the clock in the discovery Report: without it, only the authenticated
    /// second discovery step learns the agent's time.
    #[tokio::test]
    async fn an_engine_reporting_zero_boots_answers() {
        use crate::daemon::utils::scanner::{SnmpProbeOutcome, try_snmp_with_credential_on_port};

        let credential = v3(SnmpV3Security::AuthPriv {
            auth_protocol: SnmpV3AuthProtocol::Sha1,
            auth_password: secret("sha1authpass"),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: secret("sha1privpass"),
        });
        let CredentialQueryPayload::Snmp(snmp) = credential.to_query_payload(None).unwrap() else {
            panic!("not an SNMP credential");
        };
        let device = switch_with(credential.clone());

        for discovery_carries_clock in [true, false] {
            let clock = EngineClock {
                boots: 0,
                time: 5000,
                discovery_carries_clock,
            };
            let agent = UdpAgent::start_with_clock(&device, clock).await;

            match try_snmp_with_credential_on_port(agent.ip(), &snmp, agent.port()).await {
                SnmpProbeOutcome::Answered(..) => {}
                SnmpProbeOutcome::Failed(outcome, message) => panic!(
                    "probe with the clock in discovery = {discovery_carries_clock}: \
                     {outcome:?} {message}"
                ),
            }
            let name = sys_name(&agent, &credential).await.unwrap_or_else(|e| {
                panic!("read with the clock in discovery = {discovery_carries_clock}: {e:#}")
            });
            assert_eq!(name, "secure-switch-01");
        }
    }

    #[test]
    fn oid_encoding_matches_x690() {
        // 1.3.6.1.2.1.1.5.0 — sysName.0
        assert_eq!(
            oid_content(&[1, 3, 6, 1, 2, 1, 1, 5, 0]),
            vec![0x2b, 6, 1, 2, 1, 1, 5, 0]
        );
        // A multi-byte arc: 8072 = 0x3f 0x08 in base 128 → 0xbf 0x08.
        assert_eq!(oid_content(&[1, 3, 8072]), vec![0x2b, 0xbf, 0x08]);
        assert_eq!(signed(-1), vec![0xff]);
        assert_eq!(signed(128), vec![0x00, 0x80]);
        assert_eq!(unsigned(255), vec![0x00, 0xff]);
    }
}
