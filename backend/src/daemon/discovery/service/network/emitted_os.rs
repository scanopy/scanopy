//! An operating system named in what a scanned host sent of its own accord: its web servers'
//! `Server` headers and its mDNS `_device-info._tcp` and `_airplay._tcp` records.
//!
//! Each match is offered through [`HostData::offer_os`], which combines them: two naming different
//! families leave the host with no matched OS. The SSH banner arrives the same way, from its probe.

use std::collections::BTreeMap;

use crate::daemon::discovery::service::network::mdns::types::DnsSdHost;
use crate::daemon::discovery::service::ops::HostData;
use crate::server::hosts::r#impl::attributes::HostOsValue;
use crate::server::hosts::r#impl::os::recog::RecogDatabase;
use crate::server::services::r#impl::endpoints::EndpointResponse;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// The DNS-SD service whose TXT record describes the device itself.
const DEVICE_INFO_SERVICE: &str = "_device-info._tcp";
/// The AirPlay receiver service, whose TXT record carries the device's model.
const AIRPLAY_SERVICE: &str = "_airplay._tcp";

/// Offer the OS each `Server` header and the device-info and AirPlay records name.
pub(super) fn offer_emitted_os(
    host_data: &mut HostData,
    endpoint_responses: &[EndpointResponse],
    dns_sd: Option<&DnsSdHost>,
) {
    for header in endpoint_responses
        .iter()
        .filter_map(|response| response.headers.get("server"))
    {
        // `apache_os` reads the platform Apache appends in parentheses, which `http_servers`
        // leaves to it.
        let os = RecogDatabase::HttpServer
            .os(header)
            .or_else(|| RecogDatabase::ApacheOs.os(header));
        if let Some(os) = os {
            host_data.offer_os(Attributed::new(
                HostOsValue(os),
                AttributeSource::HttpServerMatch,
            ));
        }
    }

    let Some(dns_sd) = dns_sd else {
        return;
    };
    // The device-info record describes the device itself; read every pair, which is how Recog's
    // database is keyed (`model=…`, `osxvers=…`).
    if let Some(txt) = service_txt(dns_sd, DEVICE_INFO_SERVICE) {
        let pairs = txt.iter().map(|(key, value)| format!("{key}={value}"));
        offer_mdns_matches(host_data, pairs, AttributeSource::DnsSdDeviceInfoMatch, txt);
    }
    // A Mac that shares nothing publishes no device-info record but still advertises AirPlay, and
    // that record's `model` is the same Apple model identifier (`model=Mac17,2` from a MacBook Pro,
    // captured 2026-10-02). Only `model` is read: the other AirPlay keys are protocol details.
    if let Some(txt) = service_txt(dns_sd, AIRPLAY_SERVICE) {
        let pairs = txt.get("model").map(|model| format!("model={model}"));
        offer_mdns_matches(host_data, pairs, AttributeSource::DnsSdAirPlayMatch, txt);
    }
}

/// The TXT pairs a host advertised for one DNS-SD service type, if it advertised it.
fn service_txt<'a>(dns_sd: &'a DnsSdHost, service: &str) -> Option<&'a BTreeMap<String, String>> {
    dns_sd
        .services
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(service))
        .map(|(_, txt)| txt)
}

/// Offer the OS each `key=value` pair names. A record that arrived and named nothing is logged with
/// what it held, so a device the fingerprints do not cover can be identified from the log rather
/// than guessed at.
fn offer_mdns_matches(
    host_data: &mut HostData,
    pairs: impl IntoIterator<Item = String>,
    source: AttributeSource,
    txt: &BTreeMap<String, String>,
) {
    let mut named = false;
    for pair in pairs {
        if let Some(os) = RecogDatabase::MdnsDeviceInfo.os(&pair) {
            named = true;
            host_data.offer_os(Attributed::new(HostOsValue(os), source));
        }
    }
    if !named {
        tracing::debug!(
            address = ?host_data.ip_addresses.first().map(|ip| ip.base.ip_address),
            %source,
            txt = ?txt,
            "mDNS record named no OS"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::hosts::r#impl::os::HostOsFamily;
    use crate::server::ports::r#impl::base::PortType;
    use crate::server::services::r#impl::endpoints::{ApplicationProtocol, Endpoint};

    fn host() -> HostData {
        HostData::new(
            Host::new(HostBase::default()),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn server_header(value: &str) -> EndpointResponse {
        EndpointResponse {
            endpoint: Endpoint {
                protocol: ApplicationProtocol::Http,
                ip: None,
                port_type: PortType::Http,
                path: "/".to_string(),
            },
            body: String::new(),
            headers: HashMap::from([("server".to_string(), value.to_string())]),
            status: 200,
        }
    }

    fn device_info(model: &str) -> DnsSdHost {
        DnsSdHost {
            services: BTreeMap::from([(
                DEVICE_INFO_SERVICE.to_string(),
                BTreeMap::from([("model".to_string(), model.to_string())]),
            )]),
            ..Default::default()
        }
    }

    /// An AirPlay record as a Mac or a speaker announces it: `model` plus protocol keys.
    fn airplay(model: &str) -> DnsSdHost {
        DnsSdHost {
            services: BTreeMap::from([(
                AIRPLAY_SERVICE.to_string(),
                BTreeMap::from([
                    ("model".to_string(), model.to_string()),
                    ("srcvers".to_string(), "980.77.5".to_string()),
                ]),
            )]),
            ..Default::default()
        }
    }

    /// The record a MacBook Pro advertised on a live LAN, with no device-info record beside it.
    #[test]
    fn a_mac_that_only_advertises_airplay_names_macos() {
        let mut host_data = host();
        offer_emitted_os(&mut host_data, &[], Some(&airplay("Mac17,2")));
        assert_eq!(family(&host_data), Some(HostOsFamily::MacOs));
        assert_eq!(
            host_data.host.base.os.as_ref().unwrap().source(),
            AttributeSource::DnsSdAirPlayMatch
        );
    }

    #[test]
    fn an_airplay_speaker_model_names_no_os() {
        let mut host_data = host();
        offer_emitted_os(&mut host_data, &[], Some(&airplay("Five")));
        assert_eq!(family(&host_data), None);
    }

    fn family(host_data: &HostData) -> Option<HostOsFamily> {
        host_data
            .host
            .base
            .os
            .as_ref()
            .map(|os| os.value().0.family)
    }

    #[test]
    fn a_server_header_names_the_platform() {
        let mut host_data = host();
        offer_emitted_os(&mut host_data, &[server_header("Microsoft-IIS/8.5")], None);
        assert_eq!(family(&host_data), Some(HostOsFamily::Windows));
        assert_eq!(
            host_data.host.base.os.as_ref().unwrap().source(),
            AttributeSource::HttpServerMatch
        );
    }

    #[test]
    fn a_mac_model_in_device_info_names_macos() {
        let mut host_data = host();
        offer_emitted_os(&mut host_data, &[], Some(&device_info("MacBookPro18,3")));
        assert_eq!(family(&host_data), Some(HostOsFamily::MacOs));
    }

    #[test]
    fn matches_that_disagree_leave_no_os_and_a_later_one_cannot_restore_it() {
        let mut host_data = host();
        offer_emitted_os(
            &mut host_data,
            &[server_header("Microsoft-IIS/8.5")],
            Some(&device_info("MacBookPro18,3")),
        );
        assert_eq!(family(&host_data), None);

        offer_emitted_os(&mut host_data, &[server_header("Microsoft-IIS/8.5")], None);
        assert_eq!(family(&host_data), None);
    }

    #[test]
    fn a_header_that_names_no_os_writes_none() {
        let mut host_data = host();
        offer_emitted_os(&mut host_data, &[server_header("nginx")], None);
        assert_eq!(family(&host_data), None);
    }
}
