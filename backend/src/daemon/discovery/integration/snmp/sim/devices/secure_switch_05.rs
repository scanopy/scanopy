use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{
    CommunityVersions, Purpose, SimDevice, Tables,
};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::{
    CredentialType, SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpV3Security,
};

use super::inline;

/// A Catalyst 2960 configured `snmp-server user … v3 auth sha … priv aes 256 …`. IOS stretches
/// the 20-byte SHA-1 key to AES-256's 32 the Reeder way; the agent's side of that is
/// [`SimDevice::agent_key_extension`], since the credential has no field for it.
pub fn device() -> SimDevice {
    SimDevice {
        name: "secure-switch-05",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#846",
            defect: "SNMPv3 AES-256 with the Reeder key extension (Cisco `aes 256`) — a daemon that only extends keys the Blumenthal way is never answered",
        },
        credential: CredentialType::SnmpV3 {
            security_name: "scanopyreeder".into(),
            security: SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: inline("reederauthpass12345"),
                priv_protocol: SnmpV3PrivProtocol::Aes256,
                priv_password: inline("reederprivpass12345"),
            },
            context_name: None,
        },
        community_versions: CommunityVersions::Both,
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.0(2)SE11"
                    .into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1208".into()),
            sys_name: Some("secure-switch-05".into()),
            sys_location: Some("Warehouse, Rack 3".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: Tables {
            if_table: Some(IfTable::new(vec![
                IfRow::port(1, "Fa0/1", Some("00:1a:2b:00:33:01".parse().unwrap())).alias("Uplink"),
                IfRow::port(2, "Fa0/2", Some("00:1a:2b:00:33:02".parse().unwrap())),
            ])),
            bridge: BridgeTable::derived(),
            ..Default::default()
        },
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

/// GH #846. One AES-256 credential has to read an agent that extends its key either way, and
/// the second try must cost nothing where it cannot help. A handshake is one engine-discovery
/// request, which [`UdpAgent::discoveries`] counts: one means the daemon tried one key
/// extension, two means it tried both.
#[cfg(test)]
mod tests {
    use snmp2::v3::KeyExtension;
    use tokio::net::UdpSocket;

    use super::*;
    use crate::daemon::discovery::integration::snmp::session::{SnmpContext, create_session};
    use crate::daemon::discovery::integration::snmp::sim::{self, udp::UdpAgent};
    use crate::daemon::discovery::integration::snmp::{
        probe_port, query_system_info, walk_if_table,
    };
    use crate::daemon::discovery::service::warnings::AttemptOutcome;
    use crate::server::credentials::r#impl::mapping::{
        CredentialQueryPayload, SnmpQueryCredential,
    };

    /// An AuthPriv user named as the device's is, with the device's auth password.
    fn user(
        auth_protocol: SnmpV3AuthProtocol,
        priv_protocol: SnmpV3PrivProtocol,
        priv_password: &str,
    ) -> CredentialType {
        CredentialType::SnmpV3 {
            security_name: "scanopyreeder".into(),
            security: SnmpV3Security::AuthPriv {
                auth_protocol,
                auth_password: inline("reederauthpass12345"),
                priv_protocol,
                priv_password: inline(priv_password),
            },
            context_name: None,
        }
    }

    /// The credential as the server sends it to the daemon.
    fn as_sent(credential: &CredentialType) -> SnmpQueryCredential {
        match credential.to_query_payload(None).unwrap() {
            CredentialQueryPayload::Snmp(snmp) => snmp,
            _ => panic!("not an SNMP credential"),
        }
    }

    /// An agent that extends its key the Blumenthal way, configured with `credential`.
    async fn blumenthal_agent(credential: CredentialType) -> UdpAgent {
        let mut device = sim::device("secure-switch-01");
        device.credential = credential;
        UdpAgent::start(&device).await
    }

    #[tokio::test]
    async fn an_aes_256_credential_reads_an_agent_that_extends_its_key_the_reeder_way() {
        let device = sim::device("secure-switch-05");
        let agent = UdpAgent::start(&device).await;

        let mut session = create_session(
            agent.ip(),
            &as_sent(&device.credential),
            agent.port(),
            SnmpContext::Default,
            None,
        )
        .await
        .expect("the session opens on the second key extension");

        let system = query_system_info(&mut session, agent.ip()).await.unwrap();
        assert_eq!(system.sys_name, device.system.sys_name);
        assert_eq!(system.sys_descr, device.system.sys_descr);
        let if_table = walk_if_table(&mut session, agent.ip()).await.unwrap();
        assert_eq!(if_table.entries.len(), 2);
        assert!(if_table.set_complete && if_table.attributes_complete);
    }

    /// Both auth protocols whose key is too short for AES-256, against net-snmp's `AES-256`.
    #[tokio::test]
    async fn an_agent_that_extends_its_key_the_blumenthal_way_is_read_in_one_handshake() {
        for auth_protocol in [SnmpV3AuthProtocol::Md5, SnmpV3AuthProtocol::Sha1] {
            let credential = user(
                auth_protocol,
                SnmpV3PrivProtocol::Aes256,
                "reederprivpass12345",
            );
            let agent = blumenthal_agent(credential.clone()).await;

            create_session(
                agent.ip(),
                &as_sent(&credential),
                agent.port(),
                SnmpContext::Default,
                None,
            )
            .await
            .unwrap_or_else(|e| panic!("{auth_protocol:?}: {e:#}"));

            assert_eq!(agent.discoveries(), 1, "{auth_protocol:?}");
        }
    }

    /// The collection's sessions open with what the probe learned, so the refused handshake is
    /// paid once per host and not once per session.
    #[tokio::test]
    async fn a_session_opened_after_the_probe_starts_with_the_key_extension_it_found() {
        let device = sim::device("secure-switch-05");
        let agent = UdpAgent::start(&device).await;
        let credential = as_sent(&device.credential);

        let probed = probe_port(agent.ip(), &credential, agent.port())
            .await
            .unwrap_or_else(|(outcome, message)| panic!("probe: {outcome:?} {message}"));
        assert_eq!(probed.key_extension, Some(KeyExtension::Reeder));
        let during_probe = agent.discoveries();

        create_session(
            agent.ip(),
            &probed.credential,
            probed.port,
            SnmpContext::Default,
            probed.key_extension,
        )
        .await
        .expect("the later session opens");

        assert_eq!(agent.discoveries(), during_probe + 1);
    }

    /// An address where nothing answers engine discovery: offline, or no SNMPv3 agent. Its one
    /// probe sends one discovery request and waits out the probe cap, as it did before.
    #[tokio::test]
    async fn a_host_that_does_not_answer_discovery_is_not_tried_with_the_other_key_extension() {
        let silent = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = silent.local_addr().unwrap();
        let credential = as_sent(&device().credential);

        let outcome = probe_port(addr.ip(), &credential, addr.port()).await;

        assert!(matches!(outcome, Err((AttemptOutcome::TimedOut, _))));
        let mut received = 0;
        let mut buf = [0u8; 2048];
        while silent.try_recv_from(&mut buf).is_ok() {
            received += 1;
        }
        assert_eq!(received, 1, "requests sent to a host that never answered");
    }

    /// AES-128 keys are never extended, so there is no second method and no second handshake,
    /// even on the failure that triggers one for AES-256.
    #[tokio::test]
    async fn a_wrong_aes_128_privacy_password_is_tried_once() {
        let agent = blumenthal_agent(user(
            SnmpV3AuthProtocol::Sha1,
            SnmpV3PrivProtocol::Aes128,
            "reederprivpass12345",
        ))
        .await;
        let wrong = as_sent(&user(
            SnmpV3AuthProtocol::Sha1,
            SnmpV3PrivProtocol::Aes128,
            "not-the-privacy-password",
        ));

        let outcome = probe_port(agent.ip(), &wrong, agent.port()).await;

        assert!(outcome.is_err());
        assert_eq!(agent.discoveries(), 1);
    }

    /// Neither key extension helps a wrong password. Both are tried, and the probe reports one
    /// refusal, which is what raises the credential warning for the host.
    #[tokio::test]
    async fn a_wrong_aes_256_privacy_password_is_refused_after_both_key_extensions() {
        let agent = UdpAgent::start(&device()).await;
        let wrong = as_sent(&user(
            SnmpV3AuthProtocol::Sha1,
            SnmpV3PrivProtocol::Aes256,
            "not-the-privacy-password",
        ));

        let outcome = probe_port(agent.ip(), &wrong, agent.port()).await;

        assert!(
            matches!(outcome, Err((AttemptOutcome::Rejected, _))),
            "{:?}",
            outcome.err()
        );
        assert_eq!(agent.discoveries(), 2);
    }
}
