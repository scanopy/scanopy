//! The application-probe stage: unicast an address on a known port, speak the protocol, and
//! report what answered.
//!
//! Before this module a non-credentialed probe was reached through a hardcoded `match port` inside
//! `scan_udp_ports`, every probe returned `Result<Option<u16>, Error>` — a port number — and the
//! response body was discarded. BACnet read an I-Am carrying a vendor ID and device instance,
//! checked that byte 0 was `0x81`, and dropped it. Adding a probe meant editing that function, and
//! DHCP's "only on the gateway" rule lived inside one of its arms as an `if is_gateway`.
//!
//! **A probe is a method on the [`ServiceDefinition`] it belongs to.** That gives one registry
//! rather than two and makes the invariants that matter structural: a probe cannot exist without a
//! service definition, because a definition is the only thing that can hand one out; and the port
//! that gets probed is the port that gets scanned, because
//! [`probe_pattern`](crate::server::services::r#impl::patterns::probe_pattern) derives the
//! definition's discovery pattern from [`AppProbe::port`]. What stays outside is HTTP endpoint
//! probing, which is already declarative, and the credentialed integrations, which have their own
//! registry keyed on credential type — a split that means something, unlike the one this replaces.
//!
//! The probe owns its transport, because they differ: DNS and NTP use library clients (hickory,
//! sntpc), BACnet and EtherNet/IP bind a UDP socket, and Modbus opens a TCP stream and keeps it for
//! a second exchange.

pub mod amqp;
pub mod bacula;
pub mod beszel_agent;
pub mod cassandra;
pub mod checkmk;
pub mod dns_tcp;
pub mod docker_swarm;
pub mod ethernet_ip;
pub mod ftp;
pub mod h323;
pub mod ike;
pub mod kafka;
pub mod kerberos;
pub mod ldap;
#[cfg(test)]
mod live_servers;
pub mod mgcp;
#[cfg(test)]
mod middlebox;
pub mod modbus;
pub mod mongodb;
pub mod mqtt;
pub mod mssql;
pub mod mysql;
pub mod nfs;
pub mod nut;
pub mod opcua;
pub mod openvpn;
pub mod oracle;
pub mod postgresql;
pub mod rdp;
pub mod redis;
pub mod rtsp;
pub mod sip;
pub mod smb;
pub mod ssh;
pub mod telnet;
pub mod tftp;
pub mod tls;
pub mod udp;
pub mod unbound_control;
pub mod zabbix;
pub mod zmtp;

use anyhow::Error;
use async_trait::async_trait;
use cidr::IpCidr;
use std::net::IpAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::daemon::utils::scanner::{SCAN_TIMEOUT, ScanConcurrencyController, batch_scan};
use crate::server::hosts::r#impl::attributes::{
    HostFirmwareRevisionAttributed, HostManufacturerAttributed, HostModelAttributed,
    HostOsAttributed, HostSerialNumberAttributed,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, AttributeValue, Attributed};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Everything a probe is allowed to know about its target.
///
/// Owned rather than borrowed: [`batch_scan`] requires the futures it drives to be `'static`, so a
/// context holding references could not be captured into one.
#[derive(Debug, Clone)]
pub struct ProbeContext {
    pub ip: IpAddr,
    /// The subnet the address was found on. DHCP needs it to compute a broadcast address.
    pub subnet_cidr: IpCidr,
    /// Whether this address is in the daemon's routing table.
    pub is_gateway: bool,
    pub cancel: CancellationToken,
    /// Reports FD exhaustion so the scan degrades instead of failing.
    ///
    /// `scan_endpoints` does not feed this and does not need to — reqwest pools its connections.
    /// A raw-byte probe opens its own socket per target, so it is in the same position as
    /// `scan_tcp_ports`, which is the one stage that does feed it today.
    pub scan_controller: Arc<ScanConcurrencyController>,
}

impl ProbeContext {
    /// Report a connect failure, so an FD-exhaustion error degrades the whole scan's concurrency
    /// rather than being read as "nothing is listening there".
    pub fn note_connect_error(&self, error: &std::io::Error) {
        self.scan_controller.check_and_handle_error(error);
    }
}

/// What a probe established, beyond "it did not work".
///
/// Two variants rather than more because a probe has exactly two things to say that the caller can
/// act on. A transport failure is an `Err` from [`AppProbe::run`] instead, so the stage can keep
/// applying the same critical-error classification the old dispatch did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppProbeOutcome {
    /// The protocol answered on this port.
    ///
    /// `identity` is `None` both for protocols that carry no identity at all (DNS, NTP, DHCP,
    /// BACnet, OPC UA) and for a device that answered detection but declined identification — a
    /// Modbus device without `0x2B` support. Those two cases need no distinction downstream: the
    /// service matches on the probe having answered, and the host simply has no `model`.
    Answered { identity: Option<DeviceIdentity> },
    /// Nothing answered, or what answered was not this protocol.
    NoAnswer,
}

/// The hardware identity a probe read off a device.
///
/// No `Default`, following [`ControllerIdentity`](crate::daemon::discovery::integration::controller::ControllerIdentity):
/// a new probe cannot `..Default::default()` past a field, so "we forgot to record the model" is a
/// compile error rather than a silently empty column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    /// Not a plain string, because the four fields do not all come from the same place. A probe
    /// that reads a vendor *name* off the device and one that synthesises it from a numeric ID are
    /// making different claims, and only the value's own source can say which — see
    /// `ethernet_ip.rs`, whose `manufacturer` is our construction rather than the device's word.
    pub manufacturer: Option<HostManufacturerAttributed>,
    pub model: Option<HostModelAttributed>,
    pub serial_number: Option<HostSerialNumberAttributed>,
    /// Firmware or product revision, as the device words it — Modbus MajorMinorRevision,
    /// EtherNet/IP `<major>.<minor>`.
    pub firmware_revision: Option<HostFirmwareRevisionAttributed>,
    /// The operating system, when the protocol names it: the distribution an SSH banner carries,
    /// or the Windows build in an SMB challenge.
    /// Boxed: an OS reading is several times the size of the other fields, and inline it makes
    /// every enum carrying an identity (`AppProbeOutcome`, `ModbusReply`) needlessly large.
    pub os: Option<Box<HostOsAttributed>>,
}

impl DeviceIdentity {
    /// Fold this identity into the host currently being scanned.
    ///
    /// Each value carries the source that produced it, so the ordering against a credentialed
    /// read is stated rather than resting on the app-probe stage running last. An industrial
    /// protocol is the device's own, which outranks a generic MIB — so a Modbus vendor name does
    /// displace SNMP's, deliberately. What does not is a manufacturer we synthesised from a vendor
    /// ID: that is an inference, and it says so.
    pub fn enrich(&self, host_data: &mut HostData) {
        let Self {
            manufacturer,
            model,
            serial_number,
            firmware_revision,
            os,
        } = self.clone();

        if let Some(manufacturer) = manufacturer {
            Attributed::apply(&mut host_data.host.base.manufacturer, manufacturer);
        }
        if let Some(model) = model {
            Attributed::apply(&mut host_data.host.base.model, model);
        }
        if let Some(serial_number) = serial_number {
            Attributed::apply(&mut host_data.host.base.serial_number, serial_number);
        }
        if let Some(firmware_revision) = firmware_revision {
            Attributed::apply(
                &mut host_data.host.base.firmware_revision,
                firmware_revision,
            );
        }
        if let Some(os) = os {
            host_data.offer_os(*os);
        }
    }
}

/// Wrap one identity string with the source that produced it, dropping it if the device sent
/// nothing usable.
///
/// Devices routinely pad a fixed-width identity field with spaces or return it empty, and an empty
/// string is absence rather than a value. Trimming here rather than at each parse site keeps the
/// two probes from disagreeing about it; the applier then refuses whatever is still blank.
pub fn identity_field<T: AttributeValue + From<String>>(
    value: Option<String>,
    source: AttributeSource,
) -> Option<Attributed<T>> {
    let value = value?.trim().to_string();
    let carrier = Attributed::new(T::from(value), source);
    (!carrier.is_blank()).then_some(carrier)
}

/// A non-credentialed application probe.
#[async_trait]
pub trait AppProbe: Send + Sync {
    /// The port this probe speaks on. Also the port the definition's discovery pattern scans, so
    /// the two cannot disagree.
    fn port(&self) -> PortType;

    /// The evidence a successful probe contributes to service matching, for protocols whose
    /// definition matches on [`Pattern::ClientResponse`](crate::server::services::r#impl::patterns::Pattern::ClientResponse).
    ///
    /// `None` for the four probes migrated off the old dispatch: their definitions match on the
    /// port alone, exactly as they did before, and minting `ClientProbe` variants nothing consumes
    /// would manufacture the very problem `every_client_probe_variant_has_a_producer` exists to
    /// catch.
    fn client_probe(&self) -> Option<ClientProbe> {
        None
    }

    /// Whether this probe applies to this target at all. DHCP overrides it.
    fn applies(&self, _ctx: &ProbeContext) -> bool {
        true
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error>;
}

/// What one answered probe contributes back to the scan.
#[derive(Debug, Clone)]
pub struct AppProbeResult {
    pub port: PortType,
    pub client_probe: Option<ClientProbe>,
    pub identity: Option<DeviceIdentity>,
}

/// Connect to `port` on the context's address and read whatever the peer sends first.
///
/// For the protocols where the *server* opens the conversation: SSH sends its version string, FTP
/// and SMTP send a `220`, a Check_MK agent dumps its section list. `None` covers every way that can
/// fail to produce evidence — no connect, no bytes, a closed connection — because a probe only ever
/// distinguishes "the protocol answered" from "it did not".
///
/// A connect error is reported to the scan controller before being swallowed, so FD exhaustion
/// degrades the scan's concurrency rather than being read as "nothing is listening there".
pub(crate) async fn read_greeting(ctx: &ProbeContext, port: PortType, buf_len: usize) -> Vec<u8> {
    let addr = std::net::SocketAddr::new(ctx.ip, port.number());
    let mut stream = match tokio::time::timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            ctx.note_connect_error(&e);
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    let mut buf = vec![0u8; buf_len];
    match tokio::time::timeout(SCAN_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(read)) => {
            buf.truncate(read);
            buf
        }
        // A listener that accepts and then says nothing is the middlebox shape, and it is why this
        // returns empty rather than erroring: silence is a legitimate answer meaning "not this".
        _ => Vec::new(),
    }
}

/// Connect to `port`, send `request`, and read the reply.
///
/// For the protocols where the *client* opens the conversation: RTSP `OPTIONS`, a Redis `PING`, an
/// LDAP search, an SMB negotiate. Same `None`-shaped contract as [`read_greeting`].
pub(crate) async fn request_response(
    ctx: &ProbeContext,
    port: PortType,
    request: &[u8],
    buf_len: usize,
) -> Vec<u8> {
    let addr = std::net::SocketAddr::new(ctx.ip, port.number());
    let mut stream = match tokio::time::timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            ctx.note_connect_error(&e);
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    if tokio::time::timeout(SCAN_TIMEOUT, stream.write_all(request))
        .await
        .is_err()
    {
        return Vec::new();
    }

    let mut buf = vec![0u8; buf_len];
    match tokio::time::timeout(SCAN_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(read)) => {
            buf.truncate(read);
            buf
        }
        _ => Vec::new(),
    }
}

/// Connect to `port`, send `request`, and read until `want` bytes have arrived.
///
/// Sibling of [`request_response`], for a reply of known fixed size that a peer is entitled to send
/// in pieces. ZMTP is the case that needs it: RFC 23 lets a peer write the 10-octet signature, wait
/// to read ours, and only then send the remaining 54 — so a single `read` sees a tenth of the
/// greeting and a probe built on it reports "not this protocol" against a real server.
///
/// Returns what arrived when the peer stops early or the timeout expires, which the caller checks
/// against the length it needs. The whole read is bounded by one [`SCAN_TIMEOUT`], not one per
/// segment, so a peer that dribbles bytes cannot hold the scan open.
pub(crate) async fn request_exact(
    ctx: &ProbeContext,
    port: PortType,
    request: &[u8],
    want: usize,
) -> Vec<u8> {
    let addr = std::net::SocketAddr::new(ctx.ip, port.number());
    let mut stream = match tokio::time::timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            ctx.note_connect_error(&e);
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    };

    if tokio::time::timeout(SCAN_TIMEOUT, stream.write_all(request))
        .await
        .is_err()
    {
        return Vec::new();
    }

    let mut received = Vec::with_capacity(want);
    let read_all = async {
        let mut buf = vec![0u8; want];
        while received.len() < want {
            match stream.read(&mut buf).await {
                // The peer closed. What arrived is all there is.
                Ok(0) | Err(_) => break,
                Ok(n) => received.extend_from_slice(&buf[..n]),
            }
        }
    };
    let _ = tokio::time::timeout(SCAN_TIMEOUT, read_all).await;
    received.truncate(want);
    received
}

/// Send one datagram to `port` and read one back.
///
/// The UDP sibling of [`request_response`]. Only the source *address* is checked, deliberately not
/// the source port: TFTP answers a request to 69 from a freshly allocated ephemeral port, which is
/// the protocol working correctly rather than a stray packet.
pub(crate) async fn udp_request_response(
    ctx: &ProbeContext,
    port: PortType,
    request: &[u8],
    buf_len: usize,
) -> Vec<u8> {
    let target = std::net::SocketAddr::new(ctx.ip, port.number());
    let bind = if ctx.ip.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let Ok(socket) = tokio::net::UdpSocket::bind(bind).await else {
        return Vec::new();
    };
    if socket.send_to(request, target).await.is_err() {
        return Vec::new();
    }

    let mut buf = vec![0u8; buf_len];
    match tokio::time::timeout(SCAN_TIMEOUT, socket.recv_from(&mut buf)).await {
        Ok(Ok((read, from))) if from.ip() == ctx.ip => {
            buf.truncate(read);
            buf
        }
        _ => Vec::new(),
    }
}

/// `Answered` with no identity when `detected`, `NoAnswer` otherwise.
///
/// Most probes establish presence and nothing else; this saves each of them writing the same
/// two-arm match.
pub(crate) fn presence(detected: bool) -> AppProbeOutcome {
    if detected {
        AppProbeOutcome::Answered { identity: None }
    } else {
        AppProbeOutcome::NoAnswer
    }
}

/// The probe registry *is* the service-definition registry.
pub fn all_app_probes() -> Vec<Box<dyn AppProbe>> {
    ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .flat_map(|d| d.app_probes())
        .collect()
}

/// Run every applicable probe against one address.
///
/// TCP probes gate on the connect scan having found the port open; UDP has no connect scan to gate
/// on, so its probes always run — which is what `scan_udp_ports` did for the same four probes.
pub async fn scan_app_probes(
    ctx: &ProbeContext,
    open_ports: &[PortType],
    batch_size: usize,
    scan_rate_pps: u32,
) -> Vec<AppProbeResult> {
    let probes: Vec<Box<dyn AppProbe>> = all_app_probes()
        .into_iter()
        .filter(|p| p.applies(ctx))
        .filter(|p| p.port().is_udp() || open_ports.contains(&p.port()))
        .collect();

    if probes.is_empty() {
        return Vec::new();
    }

    // UDP is slower and less reliable, capped the same way `scan_udp_ports` capped it.
    let probe_batch_size = std::cmp::min(batch_size, 10);
    let total = probes.len();
    let ip = ctx.ip;
    let ctx = ctx.clone();

    let results = batch_scan(
        probes,
        probe_batch_size,
        scan_rate_pps,
        ctx.cancel.clone(),
        move |probe| {
            let ctx = ctx.clone();
            async move {
                let port = probe.port();
                match probe.run(&ctx).await {
                    Ok(AppProbeOutcome::Answered { identity }) => {
                        tracing::trace!(ip = %ctx.ip, port = %port.number(), "Application probe answered");
                        Some(AppProbeResult {
                            port,
                            client_probe: probe.client_probe(),
                            identity,
                        })
                    }
                    Ok(AppProbeOutcome::NoAnswer) => None,
                    Err(e) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(
                                "Critical error running application probe {}:{}: {}",
                                ctx.ip,
                                port.number(),
                                e
                            );
                        }
                        None
                    }
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        probes_run = total,
        answered = results.len(),
        "Application probes complete"
    );

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::patterns::ClientProbe;

    /// Every probe is reachable only through a service definition, so this also asserts that each
    /// one has a definition behind it.
    #[test]
    fn every_registered_probe_declares_a_port_and_is_uniquely_ported() {
        let probes = all_app_probes();
        assert!(
            !probes.is_empty(),
            "the registry should expose the migrated UDP probes at minimum"
        );

        let mut ports: Vec<_> = probes.iter().map(|p| p.port()).collect();
        let before = ports.len();
        ports.sort_by_key(|p| (p.number(), p.protocol()));
        ports.dedup();
        assert_eq!(
            before,
            ports.len(),
            "two probes claiming one port would race for the same evidence slot"
        );
    }

    /// Devices routinely pad a fixed-width identity field with spaces or return it empty, and an
    /// empty string is absence rather than a value — it must not occupy a rung where it would
    /// block a real reading. `identity_field` is where both probes get that, so neither can
    /// forget it.
    #[test]
    fn blank_identity_fields_are_absence_not_values() {
        let source = AttributeSource::Probe(ClientProbe::ModbusTcp);

        let blank: Option<HostManufacturerAttributed> =
            identity_field(Some("   ".to_string()), source);
        assert_eq!(blank, None);

        let empty: Option<HostModelAttributed> = identity_field(Some(String::new()), source);
        assert_eq!(empty, None);

        let padded: Option<HostSerialNumberAttributed> =
            identity_field(Some("  FOC1234X5YZ  ".to_string()), source);
        assert_eq!(
            padded.map(|c| c.value().0.clone()),
            Some("FOC1234X5YZ".to_string())
        );
    }
}
