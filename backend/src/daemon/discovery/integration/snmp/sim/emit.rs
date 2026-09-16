//! Everything the deployment reads, rendered from the device definitions.
//!
//! Four consumers, one source: the agents' `snmpd` configs, the shell arrays `snmp-test-env.sh`
//! verifies against, the SQL that seeds the credentials a scan needs, and (via
//! [`super::SimDevice::data_files`]) the `pass` data files themselves. Nothing here is committed —
//! `make snmp-deploy` generates it and ships what it generated — so there is no second copy that
//! can drift from the structs.

use secrecy::ExposeSecret;

use super::transport::{Handler, Registration};
use super::{SimDevice, VLAN20};
use crate::daemon::discovery::integration::snmp::oids::{arp, if_mib, ip_mib, oid_parts};
use crate::server::credentials::r#impl::types::{CredentialType, SecretValue};

/// Where the deploy script puts things on the VM. Mirrored from `lxc/setup.sh`, which is the only
/// other place they appear.
pub const CONF_DIR: &str = "/etc/snmp-test";
pub const DATA_DIR: &str = "/etc/snmp-test/data";
const HANDLER: &str = "/etc/snmp-test/snmp-pass-handler.sh";
const HANDLER_UNSORTED: &str = "/etc/snmp-test/snmp-pass-handler-unsorted.sh";
const HANDLER_STUCK: &str = "/etc/snmp-test/snmp-pass-handler-stuck.sh";

/// The loopback back end holding `switch-cisco-01`'s VLAN 20 bridge table.
pub const CONTEXT_BACKEND_ADDR: &str = "127.0.0.1:16151";
const CONTEXT_BACKEND_COMMUNITY: &str = "ctxinternal";
const CONTEXT_NAME: &str = "vlan-20";
/// Cisco's community-indexing form of the same context, reachable from the command line and
/// deliberately not a seeded credential — so no v2c mapping can win against that device.
const CONTEXT_COMMUNITY: &str = "netdefault@20";

fn handler_path(handler: Handler) -> &'static str {
    match handler {
        Handler::Normal => HANDLER,
        Handler::Positional => HANDLER_UNSORTED,
        Handler::Stuck => HANDLER_STUCK,
    }
}

fn dotted(subtree: &[u64]) -> String {
    let body = subtree
        .iter()
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join(".");
    format!(".{body}")
}

/// Whether a registration has to outrank a built-in net-snmp module.
///
/// `mibII/interfaces` owns `ifNumber` and the IP module owns `ipAddrTable` and
/// `ipNetToMediaTable`; without `-p 1` those subtrees are answered from the VM's own kernel state
/// while the fixture's rows sit unserved. Derived from the subtree rather than remembered per
/// device, so a new device cannot forget it.
fn needs_priority(subtree: &[u64]) -> bool {
    [
        if_mib::IF_NUMBER_OBJECT,
        ip_mib::IP_ADDR_TABLE,
        arp::IP_NET_TO_MEDIA_TABLE,
    ]
    .iter()
    .any(|owned| subtree.starts_with(&oid_parts(owned)))
}

fn pass_line(device: &SimDevice, registration: &Registration, files: &[String]) -> String {
    let priority = if needs_priority(&registration.subtree) {
        "pass -p 1"
    } else {
        "pass"
    };
    let _ = device;
    format!(
        "{priority} {} /bin/bash {} {DATA_DIR}/{}.txt",
        dotted(&registration.subtree),
        handler_path(registration.handler),
        files[registration.file],
    )
}

/// How a credential is expressed in an `snmpd.conf`.
fn access_lines(device: &SimDevice) -> Vec<String> {
    match &device.credential {
        // VACM rather than `rocommunity`, so the agent refuses v2c and v3 outright and the v1
        // code path is the only one that can read this device (GH #557).
        CredentialType::SnmpV1 { community } => vec![
            format!("com2sec v1sec default {}", expose(community)),
            "group   v1group v1 v1sec".to_string(),
            "view    all included .1".to_string(),
            "access  v1group \"\" v1 noauth exact all none none".to_string(),
        ],
        CredentialType::SnmpV2c { community } => {
            vec![format!("rocommunity {}", expose(community))]
        }
        CredentialType::SnmpV3 {
            security_name,
            auth_password,
            priv_password,
            context_name,
            ..
        } => {
            let mut lines = vec![
                format!("persistentDir {CONF_DIR}/state/{}", device.name),
                format!(
                    "createUser {security_name} SHA-256 \"{}\" AES \"{}\"",
                    expose(auth_password),
                    expose(priv_password)
                ),
                format!("rouser {security_name} priv"),
            ];
            if let Some(context) = context_name {
                // What lets the v3 user name that context at all. Without it the request is
                // authorised for the default context and answered from the wrong table.
                lines.push(format!("rouser {security_name} priv -V all {context}"));
                lines.push(format!(
                    "com2sec -Cn {context} v20sec default {CONTEXT_COMMUNITY}"
                ));
                lines.push("group v20group v2c v20sec".to_string());
                lines.push("view all included .1".to_string());
                lines.push(format!(
                    "access v20group {context} any noauth exact all none none"
                ));
            }
            lines
        }
        other => panic!("{} has a non-SNMP credential: {other:?}", device.name),
    }
}

fn expose(secret: &SecretValue) -> String {
    match secret {
        SecretValue::Inline { value } => value.expose_secret().to_string(),
        SecretValue::FilePath { path } => {
            panic!("a simulated device's credential must be inline, not a path on the VM: {path}")
        }
    }
}

/// Where a shimmed device's agent actually listens.
///
/// The shim owns `<ip>:161`, so the agent behind it moves to loopback. Derived from the address's
/// last octet rather than allocated, so it is stable across deploys and cannot collide with
/// another device's — the same reasoning as the context back end's fixed port, which this stays
/// clear of.
pub fn upstream_port(device: &SimDevice) -> u16 {
    16000 + u16::from(device.ip.octets()[3])
}

/// The arguments `snmp-bulk-refuser.py` runs with in front of this device, or `None` where nothing
/// sits in front of it and the agent binds its own address.
pub fn shim_args(device: &SimDevice) -> Option<String> {
    let mut args: Vec<String> = device
        .refuses_getbulk()
        .iter()
        .map(|oid| format!("--refuse {}", dotted(oid)))
        .chain(
            device
                .silenced()
                .iter()
                .map(|oid| format!("--silence {}", dotted(oid))),
        )
        .collect();
    if let Some(rejection) = device.rejects_getbulk {
        args.push(format!(
            "--reject-above {} --error-status {}",
            rejection.above_repetitions, rejection.error_status
        ));
    }
    (!args.is_empty()).then(|| args.join(" "))
}

/// One device's `snmpd.conf`.
pub fn snmpd_conf(device: &SimDevice) -> String {
    let files: Vec<String> = device.data_files().into_iter().map(|f| f.name).collect();
    let system = &device.system;
    // A device behind `snmp-bulk-refuser.py` has the shim on its own address, so the agent binds
    // loopback and is reachable only through it. Without this the two would contend for
    // `<ip>:161` and whichever started second would fail to bind.
    let mut lines = vec![if shim_args(device).is_none() {
        format!("agentAddress udp:{}:161", device.ip)
    } else {
        format!("agentAddress udp:127.0.0.1:{}", upstream_port(device))
    }];
    lines.extend(access_lines(device));

    for (key, value) in [
        ("sysdescr", system.sys_descr.clone()),
        ("syscontact", system.sys_contact.clone()),
        ("sysname", system.sys_name.clone()),
        ("syslocation", system.sys_location.clone()),
        (
            "sysobjectid",
            system.sys_object_id.as_ref().map(|oid| format!(".{oid}")),
        ),
    ] {
        if let Some(value) = value {
            lines.push(format!("{key} {value}"));
        }
    }
    if let Some(services) = system.sys_services {
        lines.push(format!("sysservices {services}"));
    }

    for registration in device.registrations() {
        lines.push(pass_line(device, &registration, &files));
    }

    // The bridge subtree, under that context name only, routed to the back end holding the
    // nine-entry table. Ask without the context and you get the one-entry table above, which is
    // exactly the reporter's symptom (GH #686).
    if device.tables.context_bridge.is_some() {
        lines.push(format!(
            "proxy -Cn {CONTEXT_NAME} -v2c -c {CONTEXT_BACKEND_COMMUNITY} {CONTEXT_BACKEND_ADDR} \
             {}",
            dotted(&oid_parts(
                crate::daemon::discovery::integration::snmp::oids::bridge::BRIDGE_MIB
            ))
        ));
    }

    lines.join("\n") + "\n"
}

/// The context back end's own config, for the one device that has one.
///
/// Loopback-only: it exists to be proxied, never to be scanned. It has no macvlan, no address on
/// the test subnet, and no entry in the verify table.
pub fn context_conf(device: &SimDevice) -> Option<String> {
    let files: Vec<String> = device.context_files().into_iter().map(|f| f.name).collect();
    if files.is_empty() {
        return None;
    }
    let mut lines = vec![
        format!("agentAddress udp:{CONTEXT_BACKEND_ADDR}"),
        format!("rocommunity {CONTEXT_BACKEND_COMMUNITY} 127.0.0.1"),
        format!("sysdescr {} VLAN 20 bridge context", device.name),
        format!("sysname {}-{VLAN20}", device.name),
        "sysservices 2".to_string(),
    ];
    for registration in device.context_registrations() {
        lines.push(pass_line(device, &registration, &files));
    }
    Some(lines.join("\n") + "\n")
}

/// The name of a device's context back-end unit, or `None`.
pub fn context_unit(device: &SimDevice) -> Option<String> {
    device
        .tables
        .context_bridge
        .as_ref()
        .map(|_| format!("{}-{VLAN20}", device.name))
}

/// The shell arrays `snmp-test-env.sh` sources.
///
/// Retires the third hand-maintained copy of the device list: addresses, versions, communities and
/// sysNames were all repeated there and could disagree with the agents they were verifying.
pub fn lab_env(devices: &[SimDevice]) -> String {
    let field =
        |f: &dyn Fn(&SimDevice) -> String| devices.iter().map(f).collect::<Vec<_>>().join(" ");
    let mut out = String::from(
        "# Generated from backend/src/daemon/discovery/integration/snmp/sim — do not edit.\n\
         # Regenerate with `make snmp-fixtures`.\n",
    );
    out.push_str(&format!("HOSTS=({})\n", field(&|d| d.ip.to_string())));
    out.push_str(&format!(
        "VERSIONS=({})\n",
        field(&|d| match d.credential {
            CredentialType::SnmpV1 { .. } => "v1".into(),
            CredentialType::SnmpV3 { .. } => "v3".into(),
            _ => "v2c".to_string(),
        })
    ));
    out.push_str(&format!(
        "COMMUNITIES=({})\n",
        field(&|d| match &d.credential {
            CredentialType::SnmpV1 { community } | CredentialType::SnmpV2c { community } =>
                expose(community),
            _ => "-".to_string(),
        })
    ));
    // The name the agent answers `sysName.0` with, which is not always the device's own label:
    // switch-omada-01 reports the literal `switch`, matching the reporter's device.
    out.push_str(&format!(
        "SYSNAMES=({})\n",
        field(&|d| format!(
            "\"{}\"",
            d.system.sys_name.clone().unwrap_or_else(|| d.name.into())
        ))
    ));
    out.push_str(&format!("UNITS=({})\n", field(&|d| d.name.to_string())));
    out.push_str(&format!(
        "V3_USERS=({})\n",
        field(&|d| match &d.credential {
            CredentialType::SnmpV3 { security_name, .. } => format!("\"{security_name}\""),
            _ => "\"\"".to_string(),
        })
    ));
    // One entry per device needing `snmp-bulk-refuser.py` in front of it, empty for the rest:
    // `unit|listen-ip|upstream-port|shim-args`. Everything the unit needs is here rather than in
    // the deploy script, for the same reason the rest of this file exists — a second copy of the
    // device list is a second thing that can disagree with the structs.
    out.push_str(&format!(
        "SHIMS=({})\n",
        field(&|d| {
            let Some(args) = shim_args(d) else {
                return "\"\"".to_string();
            };
            format!("\"{}|{}|{}|{}\"", d.name, d.ip, upstream_port(d), args)
        })
    ));
    // Addresses a device serves *beyond* its own — empty for every device but the one guest-
    // subnet fixture. `snmp-verify`'s leakage check reads this alongside `HOSTS` to build the
    // exact set an agent is allowed to report, so it can fail on any address outside it.
    out.push_str(&format!(
        "EXTRA_ADDRS=({})\n",
        field(&|d| {
            if d.tables.ip_addr.rows.is_empty() {
                return "\"\"".to_string();
            }
            let addrs: Vec<String> = d
                .tables
                .ip_addr
                .rows
                .iter()
                .map(|row| row.address.to_string())
                .collect();
            format!("\"{}\"", addrs.join(","))
        })
    ));
    // Whether a device deliberately serves no `ipAddrTable` at all — only `switch-mute-01`.
    // Without this the leakage check cannot tell "mute by design" from "the pass registration was
    // lost": both walk back nothing, and `snmpwalk -Ovq` prints "No Such Instance currently
    // exists at this OID" on stdout, which the check would otherwise compare against the allowed
    // addresses and report as a leak. Emitting the fact lets it assert the right thing in both
    // directions — silence required here, an own address required everywhere else.
    out.push_str(&format!(
        "SUPPRESSES_IPADDR=({})\n",
        field(&|d| if d.suppresses_ip_addr_table() {
            "1".to_string()
        } else {
            "0".to_string()
        })
    ));
    out
}

/// Markers `splice_device_table` replaces the content between, in `SNMP-TEST-ENV.md`.
pub const DEVICE_TABLE_BEGIN: &str = "<!-- BEGIN GENERATED DEVICE TABLE -->";
pub const DEVICE_TABLE_END: &str = "<!-- END GENERATED DEVICE TABLE -->";

/// The device table `SNMP-TEST-ENV.md` publishes: address, name, SNMP version and credential.
///
/// The same source `lab_env` reads, so the table and the shell arrays it verifies against cannot
/// disagree — the doc used to be a fourth hand-maintained copy of this list and was stale on
/// arrival in two branches running at once.
pub fn device_table(devices: &[SimDevice]) -> String {
    let mut out = String::from("| IP | Host | Version | Credential |\n|---|---|---|---|\n");
    for device in devices {
        let version = match device.credential {
            CredentialType::SnmpV1 { .. } => "v1",
            CredentialType::SnmpV3 { .. } => "v3",
            _ => "v2c",
        };
        let credential = match &device.credential {
            CredentialType::SnmpV1 { community } | CredentialType::SnmpV2c { community } => {
                format!("community `{}`", expose(community))
            }
            CredentialType::SnmpV3 { security_name, .. } => format!("user `{security_name}`"),
            other => panic!("{} has a non-SNMP credential: {other:?}", device.name),
        };
        out.push_str(&format!(
            "| {} | {} | {version} | {credential} |\n",
            device.ip, device.name
        ));
    }
    out
}

/// Rewrite the device table in `SNMP-TEST-ENV.md`, in place, between
/// [`DEVICE_TABLE_BEGIN`]/[`DEVICE_TABLE_END`].
///
/// Panics if the markers are missing — a doc that lost them would otherwise silently stop being
/// generated, which is the exact staleness this exists to prevent.
pub fn splice_device_table(path: &std::path::Path, devices: &[SimDevice]) {
    let doc =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let start = doc
        .find(DEVICE_TABLE_BEGIN)
        .unwrap_or_else(|| panic!("{} is missing {DEVICE_TABLE_BEGIN}", path.display()));
    let end = doc
        .find(DEVICE_TABLE_END)
        .unwrap_or_else(|| panic!("{} is missing {DEVICE_TABLE_END}", path.display()));
    assert!(
        end > start,
        "{} has the markers in the wrong order",
        path.display()
    );

    let mut spliced = String::new();
    spliced.push_str(&doc[..start]);
    spliced.push_str(DEVICE_TABLE_BEGIN);
    spliced.push('\n');
    spliced.push_str(&device_table(devices));
    spliced.push_str(&doc[end..]);

    std::fs::write(path, spliced).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// A credential's JSONB, straight from the type the backend stores.
///
/// Not a hand-written blob: `serde_json` over [`CredentialType`] is what makes it impossible for
/// the seeded credential to disagree with the schema the API validates against. It needs the
/// secrets exposed — a plain serialize redacts them, which would seed credentials that
/// authenticate against nothing.
fn credential_json(credential: &CredentialType) -> String {
    let mut value = serde_json::to_value(credential).expect("a credential serialises");
    let exposed =
        |secret: &SecretValue| serde_json::json!({"mode": "Inline", "value": expose(secret)});
    match credential {
        CredentialType::SnmpV1 { community } | CredentialType::SnmpV2c { community } => {
            value["community"] = exposed(community);
        }
        CredentialType::SnmpV3 {
            auth_password,
            priv_password,
            ..
        } => {
            value["auth_password"] = exposed(auth_password);
            value["priv_password"] = exposed(priv_password);
        }
        _ => {}
    }
    value.to_string()
}

/// A human name for a credential, used as its row key.
fn credential_name(credential: &CredentialType) -> String {
    match credential {
        CredentialType::SnmpV1 { community } => {
            format!("SNMP sim — {} (v1)", expose(community))
        }
        CredentialType::SnmpV2c { community } => {
            format!("SNMP sim — {} (v2c)", expose(community))
        }
        CredentialType::SnmpV3 {
            security_name,
            context_name,
            ..
        } => match context_name {
            Some(context) => format!("SNMP sim — {security_name} {context} (v3 AuthPriv)"),
            None => format!("SNMP sim — {security_name} (v3 AuthPriv)"),
        },
        other => panic!("not an SNMP credential: {other:?}"),
    }
}

/// The SQL that seeds every credential the lab needs, assigned to every network.
///
/// The device list in each comment is derived, so it cannot go stale the way the hand-maintained
/// one did.
pub fn credentials_sql(devices: &[SimDevice]) -> String {
    let mut ordered: Vec<(String, String, Vec<String>)> = Vec::new();
    for device in devices {
        let name = credential_name(&device.credential);
        let label = format!("{} {}", device.ip, device.name);
        match ordered.iter_mut().find(|(n, _, _)| *n == name) {
            Some((_, _, users)) => users.push(label),
            None => ordered.push((name, credential_json(&device.credential), vec![label])),
        }
    }

    let mut sql = String::from(
        "-- Seed the SNMP credentials needed to scan the SNMP simulation environment.\n\
         --\n\
         -- Generated from backend/src/daemon/discovery/integration/snmp/sim — do not edit.\n\
         -- Regenerate with `make snmp-fixtures`. The credential JSON comes from the backend's own\n\
         -- `CredentialType`, so it cannot disagree with the schema the API validates against.\n\
         --\n\
         -- The lab spreads its devices across several credentials on purpose, so a scan exercises\n\
         -- credential selection and the v1/v2c/v3 negotiation paths rather than one community\n\
         -- answering everything.\n\
         --\n\
         -- Each credential is assigned to every network (Broadcast scope). Broadcast is the only\n\
         -- option that works before a scan has run: PerHost assignment needs hosts to exist, and\n\
         -- the sim devices are exactly what the first scan discovers.\n\
         --\n\
         -- Idempotent. Credential ids are derived from (organization_id, name), so a re-run\n\
         -- updates the existing rows in place instead of accumulating duplicates.\n\n\
         BEGIN;\n\n\
         CREATE TEMPORARY TABLE seed_snmp_credentials (name TEXT PRIMARY KEY, credential_type JSONB)\n    \
         ON COMMIT DROP;\n\n\
         INSERT INTO seed_snmp_credentials (name, credential_type) VALUES\n",
    );
    let rows: Vec<String> = ordered
        .iter()
        .map(|(name, json, users)| {
            let comment = users
                .chunks(3)
                .map(|chunk| format!("    -- {}", chunk.join(", ")))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{comment}\n    ('{}',\n     '{}')",
                name.replace('\'', "''"),
                json.replace('\'', "''")
            )
        })
        .collect();
    sql.push_str(&rows.join(",\n"));
    sql.push_str(
        ";\n\n\
         -- One credential per (organization owning a network, credential). md5(...)::uuid gives a\n\
         -- stable id per organization, so two organizations each get their own copy rather than\n\
         -- fighting over one row, and a re-run finds the same ids.\n\
         INSERT INTO credentials (id, organization_id, name, credential_type, created_at, updated_at)\n\
         SELECT\n    \
         md5(org.id::text || c.name)::uuid,\n    org.id,\n    c.name,\n    c.credential_type,\n    \
         NOW(),\n    NOW()\n\
         FROM (SELECT DISTINCT organization_id AS id FROM networks) org\n\
         CROSS JOIN seed_snmp_credentials c\n\
         ON CONFLICT (id) DO UPDATE\n    \
         SET name = EXCLUDED.name,\n        credential_type = EXCLUDED.credential_type,\n        \
         updated_at = NOW();\n\n\
         INSERT INTO network_credentials (network_id, credential_id)\n\
         SELECT n.id, md5(n.organization_id::text || c.name)::uuid\n\
         FROM networks n\n\
         CROSS JOIN seed_snmp_credentials c\n\
         ON CONFLICT (network_id, credential_id) DO NOTHING;\n\n\
         -- A network count of 0 is the interesting case: the database has no networks yet, so\n\
         -- nothing was seeded and nothing will scan.\n\
         SELECT\n    \
         (SELECT COUNT(*) FROM networks) AS networks,\n    \
         (SELECT COUNT(*) FROM seed_snmp_credentials) AS credentials_per_network;\n\n\
         COMMIT;\n",
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::snmp::sim::{device, lab};

    /// `mibII/interfaces` owns `ifNumber`, so the fixture's registration has to outrank it —
    /// otherwise the scalar sits unserved and the device answers its interface count from the
    /// VM's kernel, contradicting itself on every scan.
    #[test]
    fn the_subtrees_net_snmp_owns_are_registered_at_higher_priority() {
        let conf = snmpd_conf(&device("switch-core-01"));
        assert!(
            conf.contains("pass -p 1 .1.3.6.1.2.1.2.1 "),
            "ifNumber must outrank mibII/interfaces:\n{conf}"
        );
        // ...and an ordinary table does not need to.
        assert!(conf.contains("pass .1.3.6.1.2.1.2.2 "));
    }

    /// A device behind `snmp-bulk-refuser.py` hands its public address to the shim and takes
    /// loopback.
    ///
    /// Both halves matter and neither is visible from the other file: if the agent kept
    /// `<ip>:161` the two would contend for it and whichever started second would fail to bind,
    /// and if `lab.env` named a different port the shim would forward into nothing. They are
    /// generated from one place so they cannot drift, and this is what says so. One device per
    /// shim mode: GH #668's getbulk drop, GH #685's silent column, and GH #710's error-status
    /// answer.
    #[test]
    fn a_device_behind_a_bulk_shim_moves_its_agent_behind_it() {
        for (name, args) in [
            ("switch-slowbulk-01", "--refuse .1.0.8802.1.1.2.1.4"),
            ("switch-quietcol-01", "--silence .1.0.8802.1.1.2.1.4.1.1.10"),
            ("switch-hikvision-01", "--reject-above 10 --error-status 5"),
        ] {
            let device = super::super::device(name);
            let port = upstream_port(&device);

            assert!(
                snmpd_conf(&device).starts_with(&format!("agentAddress udp:127.0.0.1:{port}\n")),
                "{name}: the agent has to leave the address the shim listens on"
            );

            let entry = format!("\"{name}|{}|{port}|{args}\"", device.ip);
            assert!(
                lab_env(&super::super::lab()).contains(&entry),
                "lab.env must carry the shim's whole invocation; expected {entry}"
            );
        }
    }

    /// Every other device is untouched: no shim, no loopback, no second process in front of it.
    #[test]
    fn a_device_that_serves_bulk_normally_keeps_its_own_address() {
        for device in super::super::lab() {
            if shim_args(&device).is_some() {
                continue;
            }
            assert!(
                snmpd_conf(&device).starts_with(&format!("agentAddress udp:{}:161\n", device.ip)),
                "{} should bind its own address directly",
                device.name
            );
        }
    }

    /// The v1 device must refuse v2c and v3, which `rocommunity` would not do — the whole point
    /// of GH #557 is that one device exercises the v1 path and only the v1 path.
    #[test]
    fn the_v1_device_is_locked_to_v1_by_vacm() {
        let conf = snmpd_conf(&device("legacy-switch-01"));
        assert!(conf.contains("com2sec v1sec default legacyv1"));
        assert!(conf.contains("group   v1group v1 v1sec"));
        assert!(
            !conf.contains("rocommunity"),
            "a rocommunity line would let v2c in:\n{conf}"
        );
    }

    /// Two agents, not one: `pass` registers into the default context and nothing else, so the
    /// per-VLAN table is reached by proxying to a second snmpd.
    #[test]
    fn the_context_device_proxies_its_bridge_mib_to_a_back_end() {
        let cisco = device("switch-cisco-01");
        let front = snmpd_conf(&cisco);
        assert!(
            front.contains("proxy -Cn vlan-20 -v2c -c ctxinternal 127.0.0.1:16151 .1.3.6.1.2.1.17")
        );
        assert!(front.contains("rouser scanopyctx priv -V all vlan-20"));

        let back = context_conf(&cisco).expect("the back end exists");
        assert!(back.contains("agentAddress udp:127.0.0.1:16151"));
        assert!(
            !back.contains("192.168.7."),
            "the back end must not be reachable on the test subnet:\n{back}"
        );
    }

    /// The seeded credential is the backend's own type, serialised. A hand-written blob is how the
    /// old SQL could drift from the schema the API validates against.
    #[test]
    fn seeded_credentials_carry_their_secrets_rather_than_the_redacted_placeholder() {
        let sql = credentials_sql(&lab());
        assert!(
            sql.contains(r#""community":{"mode":"Inline","value":"netdefault"}"#),
            "{sql}"
        );
        assert!(sql.contains(r#""auth_password":{"mode":"Inline","value":"authpass12345"}"#));
        assert!(
            !sql.contains("REDACTED"),
            "a redacted secret would seed a credential that authenticates against nothing"
        );
    }

    /// Every device reaches the verify table, and the credential columns line up with it.
    #[test]
    fn the_verify_table_covers_every_device() {
        let lab = lab();
        let env = lab_env(&lab);
        for line in env.lines().filter(|l| l.contains("=(")) {
            let inner = line.split_once("=(").unwrap().1.trim_end_matches(')');
            let count = shell_words(inner);
            assert_eq!(count, lab.len(), "{line} has the wrong number of entries");
        }
        // The Omada switch answers to the literal `switch`, not to its own label.
        assert!(env.contains("\"switch\""), "{env}");
    }

    /// Count shell words, respecting the quoting `lab_env` emits.
    fn shell_words(line: &str) -> usize {
        let mut count = 0;
        let mut in_quotes = false;
        let mut in_word = false;
        for c in line.chars() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                    if !in_word {
                        in_word = true;
                        count += 1;
                    }
                }
                ' ' if !in_quotes => in_word = false,
                _ if !in_word => {
                    in_word = true;
                    count += 1;
                }
                _ => {}
            }
        }
        count
    }
}
