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
        } = Self::default();

        vec![
            FieldDefinition {
                id: "scan_rate_pps",
                label: "Port Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("500"),
                secret: false,
                optional: true,
                help_text: Some(
                    "Probes per second during port scanning. Lower values reduce network impact.",
                ),
                options: None,
                default_value: Some("500"),
                category: Some("Port Scanning"),
            },
            FieldDefinition {
                id: "arp_rate_pps",
                label: "ARP Scan Rate",
                field_type: FieldType::Number,
                placeholder: Some("50"),
                secret: false,
                optional: true,
                help_text: Some(
                    "ARP packets per second during host discovery. Keep low on networks with strict switch policies.",
                ),
                options: None,
                default_value: Some("50"),
                category: Some("ARP"),
            },
            FieldDefinition {
                id: "arp_retries",
                label: "ARP Retries",
                field_type: FieldType::Number,
                placeholder: Some("2"),
                secret: false,
                optional: true,
                help_text: Some(
                    "Additional ARP rounds for non-responsive hosts. Total attempts = retries + 1.",
                ),
                options: None,
                default_value: Some("2"),
                category: Some("ARP"),
            },
            FieldDefinition {
                id: "port_scan_batch_size",
                label: "Port Scan Batch Size",
                field_type: FieldType::Number,
                placeholder: Some("200"),
                secret: false,
                optional: true,
                help_text: Some("Ports scanned concurrently per host. Range: 16-1000."),
                options: None,
                default_value: Some("200"),
                category: Some("Port Scanning"),
            },
            FieldDefinition {
                id: "probe_raw_socket_ports",
                label: "Probe Raw Socket Ports",
                field_type: FieldType::Boolean,
                placeholder: None,
                secret: false,
                optional: false,
                help_text: Some("Scan ports 9100-9107. May cause ghost printing on some printers."),
                options: None,
                default_value: Some("false"),
                category: Some("Detection"),
            },
            FieldDefinition {
                id: "use_npcap_arp",
                label: "Use Npcap ARP (Windows)",
                field_type: FieldType::Boolean,
                placeholder: None,
                secret: false,
                optional: false,
                help_text: Some(
                    "Use Npcap broadcast ARP instead of Windows SendARP. Requires Npcap installed.",
                ),
                options: None,
                default_value: Some("false"),
                category: Some("ARP"),
            },
            FieldDefinition {
                id: "full_scan_interval",
                label: "Full Scan Interval",
                field_type: FieldType::Number,
                placeholder: Some("3"),
                secret: false,
                optional: true,
                help_text: Some(
                    "Run a full 65k port scan every N scans. Other scans use a lighter port set for faster results. Set to 0 for light scans only, 1 for every scan to be full.",
                ),
                options: None,
                default_value: Some("3"),
                category: Some("Detection"),
            },
        ]
    }
}
