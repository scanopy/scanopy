//! An operating system named in what a scanned host sent of its own accord: its web servers'
//! `Server` headers and its mDNS `_device-info._tcp` record.
//!
//! Each match is offered through [`HostData::offer_os`], which combines them: two naming different
//! families leave the host with no matched OS. The SSH banner arrives the same way, from its probe.

use crate::daemon::discovery::service::network::mdns::types::DnsSdHost;
use crate::daemon::discovery::service::ops::HostData;
use crate::server::hosts::r#impl::attributes::HostOsValue;
use crate::server::hosts::r#impl::os::recog::RecogDatabase;
use crate::server::services::r#impl::endpoints::EndpointResponse;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// The DNS-SD service whose TXT record describes the device itself.
const DEVICE_INFO_SERVICE: &str = "_device-info._tcp";

/// Offer the OS each `Server` header and the device-info record name.
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

    let device_info = dns_sd.and_then(|host| {
        host.services
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(DEVICE_INFO_SERVICE))
            .map(|(_, txt)| txt)
    });
    for (key, value) in device_info.into_iter().flatten() {
        if let Some(os) = RecogDatabase::MdnsDeviceInfo.os(&format!("{key}={value}")) {
            host_data.offer_os(Attributed::new(
                HostOsValue(os),
                AttributeSource::DnsSdDeviceInfoMatch,
            ));
        }
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
