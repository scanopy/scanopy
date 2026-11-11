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

/// A switch whose one v3 user encrypts with DES, the only privacy protocol RFC 3414 defines.
pub fn device() -> SimDevice {
    SimDevice {
        name: "secure-switch-06",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#846",
            defect: "SNMPv3 AuthPriv with DES privacy — a daemon that offers only AES cannot be given a credential for it",
        },
        credential: CredentialType::SnmpV3 {
            security_name: "scanopydes".into(),
            security: SnmpV3Security::AuthPriv {
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: inline("desauthpass12345"),
                priv_protocol: SnmpV3PrivProtocol::Des,
                priv_password: inline("desprivpass12345"),
            },
            context_name: None,
        },
        community_versions: CommunityVersions::Both,
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 12.2(44)SE6"
                    .into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1208".into()),
            sys_name: Some("secure-switch-06".into()),
            sys_location: Some("Warehouse, Rack 4".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: Tables {
            if_table: Some(IfTable::new(vec![
                IfRow::port(1, "Fa0/1", Some("00:1a:2b:00:34:01".parse().unwrap())).alias("Uplink"),
                IfRow::port(2, "Fa0/2", Some("00:1a:2b:00:34:02".parse().unwrap())),
            ])),
            bridge: BridgeTable::derived(),
            ..Default::default()
        },
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::session::{SnmpContext, create_session};
    use crate::daemon::discovery::integration::snmp::sim::{self, udp::UdpAgent};
    use crate::daemon::discovery::integration::snmp::{query_system_info, walk_if_table};
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;

    /// The whole path a DES credential takes on the daemon: key localization, an encrypted
    /// request the agent can read, and replies the daemon can decrypt, for a single GET and for
    /// the walks of a table. One handshake: DES keys are never extended, so GH #846's second
    /// try has nothing to try.
    #[tokio::test]
    async fn a_des_credential_reads_the_device_in_one_handshake() {
        let device = sim::device("secure-switch-06");
        let agent = UdpAgent::start(&device).await;
        let CredentialQueryPayload::Snmp(credential) =
            device.credential.to_query_payload(None).unwrap()
        else {
            panic!("not an SNMP credential");
        };

        let mut session = create_session(
            agent.ip(),
            &credential,
            agent.port(),
            SnmpContext::Default,
            None,
        )
        .await
        .expect("the session opens");

        let system = query_system_info(&mut session, agent.ip()).await.unwrap();
        assert_eq!(system.sys_name, device.system.sys_name);
        let if_table = walk_if_table(&mut session, agent.ip()).await.unwrap();
        assert_eq!(if_table.entries.len(), 2);
        assert!(if_table.set_complete && if_table.attributes_complete);
        assert_eq!(agent.discoveries(), 1);
    }
}
