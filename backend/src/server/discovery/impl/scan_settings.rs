use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::shared::types::field_definition::{FieldDefinition, FieldType};

/// Scan performance settings. Lives on the discovery entity.
/// Numeric fields are `Option<T>` — `None` means "use daemon default".
/// The daemon unwraps with defaults at point of use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
pub struct ScanSettings {
    /// ARP retry rounds for non-responsive targets (default: 2 = 3 total attempts)
    #[serde(default)]
    pub arp_retries: Option<u32>,

    /// ARP packets per second (default: 50)
    #[serde(default)]
    pub arp_rate_pps: Option<u32>,

    /// Port scan probes per second (default: 500)
    #[serde(default)]
    pub scan_rate_pps: Option<u32>,

    /// Ports scanned concurrently per host (default: 200, clamped 16-1000)
    #[serde(default)]
    pub port_scan_batch_size: Option<usize>,

    /// Whether to probe raw-socket ports 9100-9107 (default: false).
    /// Disabled by default to prevent ghost printing on JetDirect printers.
    #[serde(default)]
    pub probe_raw_socket_ports: bool,

    /// On Windows, use Npcap broadcast ARP instead of SendARP (default: false)
    #[serde(default)]
    pub use_npcap_arp: bool,

    /// Run a full 65k port scan every N scans. Other scans use a light port set.
    /// Default: 3. Value of 0 means never full scan. Value of 1 means every scan is full.
    #[serde(default)]
    pub full_scan_interval: Option<u32>,

    /// Whether this specific scan run should do a full 65k port scan.
    /// Set by the server before dispatching to the daemon — not user-configurable.
    #[serde(default)]
    pub is_full_scan: bool,

    /// ARP scan cutoff prefix. Interfaced subnets larger than this prefix are
    /// truncated to this many IPs. Default: 15 (= /15, ~131K IPs).
    /// Lower values scan more IPs — increase arp_rate_pps accordingly.
    #[serde(default)]
    pub arp_scan_cutoff: Option<u8>,

    /// Whether an open TCP port alone is enough to record a host on a subnet the daemon reaches
    /// *through* something (default: false).
    ///
    /// On a subnet the daemon has an interface on, ARP answers first and this never applies. On a
    /// routed one every address arrives enumerated, and a middlebox that completes the TCP handshake
    /// for addresses with nothing behind them — a FortiGate session helper, a load balancer, a
    /// scrubbing appliance — makes every address look open. With this off, such an address is
    /// recorded only when something actually answered: a probe that spoke the protocol, an endpoint
    /// that replied, a credential that authenticated.
    ///
    /// Turning it on restores the older behaviour for hosts whose every open port is one Scanopy
    /// cannot interrogate. That is a real class of host — a bespoke TCP service on a routed subnet —
    /// which is why the escape hatch exists rather than the rule simply being absolute.
    #[serde(default)]
    pub trust_port_only_detections: bool,

    /// Hard ceiling on how long a single discovery run may take, in seconds
    /// (default: 21600 = 6h). When hit, the run force-completes and any hosts
    /// still queued are left un-scanned until the next run. Raise this for very
    /// large networks that legitimately need more than the default window.
    #[serde(default)]
    pub max_discovery_duration: Option<u32>,
}

pub mod defaults {
    pub fn arp_retries() -> u32 {
        2
    }
    pub fn arp_rate_pps() -> u32 {
        50
    }
    pub fn scan_rate_pps() -> u32 {
        500
    }
    pub fn port_scan_batch_size() -> usize {
        200
    }
    pub fn full_scan_interval() -> u32 {
        3
    }
    pub fn arp_scan_cutoff() -> u8 {
        15
    }
    /// 6 hours — matches the daemon's historical hardcoded ceiling.
    pub fn max_discovery_duration() -> u32 {
        21600
    }
}

impl ScanSettings {
    /// Returns (label, value, is_override) tuples for banner display.
    pub fn formatted_lines(&self) -> Vec<(&'static str, String, bool)> {
        vec![
            (
                "ARP rate:",
                format!(
                    "{} pps",
                    self.arp_rate_pps.unwrap_or(defaults::arp_rate_pps())
                ),
                self.arp_rate_pps.is_some(),
            ),
            (
                "ARP retries:",
                self.arp_retries
                    .unwrap_or(defaults::arp_retries())
                    .to_string(),
                self.arp_retries.is_some(),
            ),
            (
                "Port scan rate:",
                format!(
                    "{} pps",
                    self.scan_rate_pps.unwrap_or(defaults::scan_rate_pps())
                ),
                self.scan_rate_pps.is_some(),
            ),
            (
                "Port batch size:",
                self.port_scan_batch_size
                    .unwrap_or(defaults::port_scan_batch_size())
                    .to_string(),
                self.port_scan_batch_size.is_some(),
            ),
            (
                "Raw socket ports:",
                self.probe_raw_socket_ports.to_string(),
                self.probe_raw_socket_ports,
            ),
            (
                "Trust port-only detections:",
                self.trust_port_only_detections.to_string(),
                self.trust_port_only_detections,
            ),
            (
                "Npcap ARP:",
                self.use_npcap_arp.to_string(),
                self.use_npcap_arp,
            ),
            (
                "Full scan interval:",
                format!(
                    "every {} scans",
                    self.full_scan_interval
                        .unwrap_or(defaults::full_scan_interval())
                ),
                self.full_scan_interval.is_some(),
            ),
            (
                "ARP scan cutoff:",
                format!(
                    "/{}",
                    self.arp_scan_cutoff.unwrap_or(defaults::arp_scan_cutoff())
                ),
                self.arp_scan_cutoff.is_some(),
            ),
            (
                "Scan mode:",
                if self.is_full_scan {
                    "Full (65k ports)".to_string()
                } else {
                    "Light (discovery ports)".to_string()
                },
                self.is_full_scan,
            ),
        ]
    }

    /// Log scan settings with tracing (convenience wrapper around formatted_lines).
    pub fn log_settings(&self) {
        tracing::info!("  ───────────────────────────────────────────────────────────");
        tracing::info!("  Scan Settings:");
        for (label, value, is_override) in self.formatted_lines() {
            let source = if is_override {
                "(override)"
            } else {
                "(default)"
            };
            tracing::info!("    {:<20}{} {}", label, value, source);
        }
    }

    pub fn field_definitions() -> Vec<FieldDefinition> {
        // Destructure to enforce completeness — compiler errors if a field is added
        // to ScanSettings but not represented here.
        let Self {
            arp_retries: _,
            arp_rate_pps: _,
            scan_rate_pps: _,
            port_scan_batch_size: _,
            probe_raw_socket_ports: _,
            use_npcap_arp: _,
            full_scan_interval: _,
            is_full_scan: _, // Server-set, not a UI field
            arp_scan_cutoff: _,
            max_discovery_duration: _,
            trust_port_only_detections: _,
        } = Self::default();

        vec![
            FieldDefinition {
                id: "scan_rate_pps",
                label: "Port Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("500"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "Probes per second during port scanning. Lower values reduce network impact.",
                ),
                options: None,
                default_value: Some("500"),
                inline_format: None,
                group: Some("Port Scanning"),
                file_name: None,
            },
            FieldDefinition {
                id: "arp_rate_pps",
                label: "ARP Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("50"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "ARP packets per second during host discovery. Keep low on networks with strict switch policies.",
                ),
                options: None,
                default_value: Some("50"),
                inline_format: None,
                group: Some("ARP"),
                file_name: None,
            },
            FieldDefinition {
                id: "arp_retries",
                label: "ARP Retries",
                field_type: FieldType::Number,
                placeholder: Some("2"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "Additional ARP rounds for non-responsive hosts. Total attempts = retries + 1.",
                ),
                options: None,
                default_value: Some("2"),
                inline_format: None,
                group: Some("ARP"),
                file_name: None,
            },
            FieldDefinition {
                id: "port_scan_batch_size",
                label: "Port Scan Batch Size",
                field_type: FieldType::Number,
                placeholder: Some("200"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some("Ports scanned concurrently per host. Range: 16-1000."),
                options: None,
                default_value: Some("200"),
                inline_format: None,
                group: Some("Port Scanning"),
                file_name: None,
            },
            FieldDefinition {
                id: "trust_port_only_detections",
                label: "Trust Port-Only Detections",
                field_type: FieldType::Boolean,
                placeholder: None,
                placeholder_by: None,
                secret: false,
                optional: false,
                help_text: Some(
                    "On subnets reached through a router, record a host when a port is open even if \
                     nothing answered. Off by default: a firewall that completes TCP handshakes on \
                     behalf of empty addresses would otherwise create a host at every one.",
                ),
                options: None,
                default_value: Some("false"),
                inline_format: None,
                group: Some("Detection"),
                file_name: None,
            },
            FieldDefinition {
                id: "probe_raw_socket_ports",
                label: "Probe Raw Socket Ports",
                field_type: FieldType::Boolean,
                placeholder: None,
                placeholder_by: None,
                secret: false,
                optional: false,
                help_text: Some("Scan ports 9100-9107. May cause ghost printing on some printers."),
                options: None,
                default_value: Some("false"),
                inline_format: None,
                group: Some("Detection"),
                file_name: None,
            },
            FieldDefinition {
                id: "arp_scan_cutoff",
                label: "ARP Scan Cutoff",
                field_type: FieldType::Number,
                placeholder: Some("15"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "Interfaced subnets larger than this CIDR prefix are truncated during ARP scanning. Lower values scan more IPs — increase ARP Scan Rate accordingly for large subnets.",
                ),
                options: None,
                default_value: Some("15"),
                inline_format: None,
                group: Some("ARP"),
                file_name: None,
            },
            FieldDefinition {
                id: "use_npcap_arp",
                label: "Use Npcap ARP (Windows)",
                field_type: FieldType::Boolean,
                placeholder: None,
                placeholder_by: None,
                secret: false,
                optional: false,
                help_text: Some(
                    "Use Npcap broadcast ARP instead of Windows SendARP. Requires Npcap installed.",
                ),
                options: None,
                default_value: Some("false"),
                inline_format: None,
                group: Some("ARP"),
                file_name: None,
            },
            FieldDefinition {
                id: "full_scan_interval",
                label: "Full Scan Interval",
                field_type: FieldType::Number,
                placeholder: Some("3"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "Run a full 65k port scan every N scans. Other scans use a lighter port set for faster results. Set to 0 for light scans only, 1 for every scan to be full.",
                ),
                options: None,
                default_value: Some("3"),
                inline_format: None,
                group: Some("Detection"),
                file_name: None,
            },
            FieldDefinition {
                id: "max_discovery_duration",
                label: "Max Discovery Duration (seconds)",
                field_type: FieldType::Number,
                placeholder: Some("21600"),
                placeholder_by: None,
                secret: false,
                optional: true,
                help_text: Some(
                    "Hard time limit for a single discovery run, in seconds (default 21600 = 6 hours). When reached, the run stops and any hosts not yet scanned are left for the next run. Raise this for very large networks that need a longer window.",
                ),
                options: None,
                default_value: Some("21600"),
                inline_format: None,
                group: Some("Detection"),
                file_name: None,
            },
        ]
    }
}

/// Scan settings that apply to a single-host rescan.
///
/// Deliberately narrower than [`ScanSettings`]: a rescan verifies a known host
/// against a known port set, so the full-scan mechanism (`is_full_scan`,
/// `full_scan_interval`) must not be expressible — promoting a rescan to a
/// 65,535-port sweep defeats the feature. The remaining omissions are settings
/// that cannot bind on a one-or-two address target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
pub struct RescanSettings {
    /// ARP retry rounds. Matters more here than in a sweep: for a rescan, "did
    /// it answer" is the entire answer, so a missed round reads as a dead host.
    #[serde(default)]
    pub arp_retries: Option<u32>,

    /// Port scan probes per second. Operators lower this for fragile devices or
    /// noisy IDS, and a rescan must respect that as much as a discovery does.
    #[serde(default)]
    pub scan_rate_pps: Option<u32>,

    /// Ports scanned concurrently per host.
    #[serde(default)]
    pub port_scan_batch_size: Option<usize>,

    /// Whether to probe raw-socket ports 9100-9107. Correctness-affecting: with
    /// this off the scanner drops those ports from its results, so a printer's
    /// known JetDirect port would look like it had disappeared.
    #[serde(default)]
    pub probe_raw_socket_ports: bool,

    /// On Windows, use Npcap broadcast ARP instead of SendARP.
    #[serde(default)]
    pub use_npcap_arp: bool,

    /// Carried into a rescan: a rescan of a host on a routed subnet runs the same evidence check,
    /// and it would be strange for a host to survive a scan and vanish on rescan, or the reverse.
    #[serde(default)]
    pub trust_port_only_detections: bool,
}

impl From<&ScanSettings> for RescanSettings {
    fn from(settings: &ScanSettings) -> Self {
        // Exhaustive destructure on purpose: a new `ScanSettings` field must be
        // classified here — carried into a rescan, or dropped with the reason
        // beside it — rather than silently never reaching one.
        let ScanSettings {
            arp_retries,
            scan_rate_pps,
            port_scan_batch_size,
            probe_raw_socket_ports,
            use_npcap_arp,
            trust_port_only_detections,
            arp_rate_pps: _,           // inert: can't bind on a 1-2 address list
            arp_scan_cutoff: _,        // inert: an explicit IP list can't hit the cap
            max_discovery_duration: _, // inert: the 6h ceiling never fires on one host
            full_scan_interval: _,     // full-scan mechanism — must not apply
            is_full_scan: _,
        } = settings;

        Self {
            arp_retries: *arp_retries,
            scan_rate_pps: *scan_rate_pps,
            port_scan_batch_size: *port_scan_batch_size,
            probe_raw_socket_ports: *probe_raw_socket_ports,
            use_npcap_arp: *use_npcap_arp,
            trust_port_only_detections: *trust_port_only_detections,
        }
    }
}

impl From<&RescanSettings> for ScanSettings {
    fn from(settings: &RescanSettings) -> Self {
        // Widening back for the daemon's scan pipeline, which is parameterised
        // by `ScanSettings`. Exhaustive for the same reason as above.
        let RescanSettings {
            arp_retries,
            scan_rate_pps,
            port_scan_batch_size,
            probe_raw_socket_ports,
            use_npcap_arp,
            trust_port_only_detections,
        } = settings;

        Self {
            arp_retries: *arp_retries,
            scan_rate_pps: *scan_rate_pps,
            port_scan_batch_size: *port_scan_batch_size,
            probe_raw_socket_ports: *probe_raw_socket_ports,
            use_npcap_arp: *use_npcap_arp,
            trust_port_only_detections: *trust_port_only_detections,
            // A rescan is never a full scan — that is the point of the narrower type.
            is_full_scan: false,
            full_scan_interval: None,
            arp_rate_pps: None,
            arp_scan_cutoff: None,
            max_discovery_duration: None,
        }
    }
}
