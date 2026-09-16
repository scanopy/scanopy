//! What to call a host, and which piece of evidence supplied the answer.
//!
//! [`HostName`](super::name::HostName) decides what is *stored* in `name`; the placement rule in
//! [`super::name`] says what may be stored there. This ladder decides what a host is *called*: the
//! stored name, or one of the identifiers the host carries in its own columns.
//!
//! **The one place names and identifiers are ordered against each other:**
//!
//! `vouched name > Hostname > SysName > ChassisId > guessed name > Address`
//!
//! A stored name is a guess when its source is ([`AttributeMethod::is_guess`]): a name inferred
//! from a detected service, a daemon's provisioning placeholder, or a legacy name nothing vouches
//! for. A guess must not hide a hostname the host actually reported, so it drops below the
//! identifiers and above only the address.
//!
//! The whole ladder is returned, not only its result, so the UI can say where a title came from
//! without walking the rungs itself.
//!
//! [`AttributeMethod::is_guess`]: crate::server::shared::attribution::AttributeMethod::is_guess

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::hosts::r#impl::base::Host;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::shared::attribution::{self, AttributeSource};

/// One rung of the display-name ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum HostNameRung {
    /// The host's stored name. First when someone vouched for it, after the identifiers when it
    /// is a guess.
    Name,
    /// The hostname the host resolved to or reported.
    Hostname,
    /// SNMP `sysName`, or the same field from a controller.
    SysName,
    /// The chassis id, the one identifier an LLDP far end is known by.
    ChassisId,
    /// The address discovery saw most recently, position breaking ties.
    Address,
}

/// What one rung of the ladder holds for a particular host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HostNameLadderEntry {
    pub rung: HostNameRung,
    /// The rung's value, or `None` when the host has nothing there. A value holding only
    /// whitespace counts as nothing.
    #[schema(required)]
    pub value: Option<String>,
    /// Where the value came from. `None` when there is no value, and for `Address`, whose column
    /// carries no provenance.
    #[schema(required)]
    pub source: Option<AttributeSource>,
}

impl HostNameLadderEntry {
    fn new(rung: HostNameRung, value: Option<String>, source: Option<AttributeSource>) -> Self {
        let source = value.as_ref().and(source);
        Self {
            rung,
            value,
            source,
        }
    }
}

/// The value a ladder resolves to and the rung it came from: the first entry holding a value.
pub fn resolve_name_ladder(ladder: &[HostNameLadderEntry]) -> Option<(String, HostNameRung)> {
    ladder
        .iter()
        .find_map(|entry| entry.value.clone().map(|value| (value, entry.rung)))
}

/// Whether a stored name with this source ranks as a guess on the ladder.
fn is_guessed_name(source: AttributeSource) -> bool {
    source.method().is_guess()
}

impl Host {
    /// Every rung of the display-name ladder for this host, in resolution order.
    ///
    /// **The one place the order is written.** [`display_name_sql`] repeats it in SQL for sorting,
    /// built from the same [`is_guessed_name`] rule.
    pub fn name_ladder<'a>(
        &self,
        addresses: impl IntoIterator<Item = &'a IPAddress>,
    ) -> [HostNameLadderEntry; 5] {
        fn non_blank(value: &str) -> Option<String> {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }

        let base = &self.base;
        let name = HostNameLadderEntry::new(
            HostNameRung::Name,
            (!base.name.is_blank()).then(|| base.name.to_string()),
            Some(base.name.source()),
        );
        let hostname = HostNameLadderEntry::new(
            HostNameRung::Hostname,
            attribution::text_of(&base.hostname)
                .as_deref()
                .and_then(non_blank),
            base.hostname.as_ref().map(|v| v.source()),
        );
        let sys_name = HostNameLadderEntry::new(
            HostNameRung::SysName,
            attribution::text_of(&base.sys_name)
                .as_deref()
                .and_then(non_blank),
            base.sys_name.as_ref().map(|v| v.source()),
        );
        let chassis_id = HostNameLadderEntry::new(
            HostNameRung::ChassisId,
            attribution::text_of(&base.chassis_id)
                .as_deref()
                .and_then(non_blank),
            base.chassis_id.as_ref().map(|v| v.source()),
        );
        // The address discovery saw most recently. After a DHCP lease moves, the host keeps its old
        // address row, and the title follows the lease. Position breaks ties, which covers every
        // address of a host nothing has scanned.
        let address = HostNameLadderEntry::new(
            HostNameRung::Address,
            addresses
                .into_iter()
                .min_by_key(|ip| (std::cmp::Reverse(ip.last_seen_at), ip.base.position))
                .map(|ip| ip.base.ip_address.to_string()),
            None,
        );

        if is_guessed_name(base.name.source()) {
            [hostname, sys_name, chassis_id, name, address]
        } else {
            [name, hostname, sys_name, chassis_id, address]
        }
    }

    /// What to call this host and which rung said so. `None` when nothing identifies it.
    pub fn resolved_name<'a>(
        &self,
        addresses: impl IntoIterator<Item = &'a IPAddress>,
    ) -> Option<(String, HostNameRung)> {
        resolve_name_ladder(&self.name_ladder(addresses))
    }

    /// What to call this host: its name, or the best identifying evidence we hold.
    ///
    /// `None` rather than `Some("")` when nothing identifies it. A `HostName::Unnamed` formats as
    /// the empty string, so returning it would put a name on the host that every consumer's `??`
    /// fallback then reads as present, a row or a node titled with nothing at all. Absence has to
    /// be expressible for those fallbacks to fire.
    ///
    /// On `Host` rather than on the topology context that first needed it, because the host list
    /// and the same host drawn in topology must not disagree about what it is called. One ladder,
    /// every surface.
    pub fn display_name<'a>(
        &self,
        addresses: impl IntoIterator<Item = &'a IPAddress>,
    ) -> Option<String> {
        self.resolved_name(addresses).map(|(value, _)| value)
    }
}

/// The name sources that rank as guesses, as SQL `jsonb` literals, for [`display_name_sql`].
///
/// Built from [`AttributeSource::all`] through the same [`is_guessed_name`] rule the ladder uses, so
/// the SQL cannot fall out of step with the Rust. Every guess tier holds only bare-string sources,
/// so each serialises to a quoted name.
static GUESSED_NAME_SOURCES_SQL: LazyLock<String> = LazyLock::new(|| {
    AttributeSource::all()
        .into_iter()
        .filter(|source| is_guessed_name(*source))
        .map(|source| {
            let json = serde_json::to_string(&source).expect("an AttributeSource serialises");
            format!("'{json}'::jsonb")
        })
        .collect::<Vec<_>>()
        .join(", ")
});

/// SQL mirror of [`Host::name_ladder`], for ordering and grouping a host list by the title it
/// actually renders.
///
/// A column labelled Name that sorts on `hosts.name` while showing the ladder's result puts every
/// nameless-but-titled host under the empty string, visibly ordering on something other than what
/// it draws. Sorting has to walk the same rungs, and the database is the only place that can do it
/// across a page it hasn't sent yet.
///
/// Takes the alias of the `hosts` row and the alias of its primary-address join; the caller must
/// put [`host_primary_address_join`](super::base::host_primary_address_join) for the same host
/// alias in scope.
///
/// **All five rungs, always.** Grouping is what makes a shorter variant wrong: a host titled by its
/// address would group under the empty string alongside a host with no title at all.
///
/// `NULLIF(…, '')` on every text rung because a host's `name` is stored as the empty string when
/// unnamed, not as NULL, so `COALESCE` alone would stop at the first rung every time.
pub fn display_name_sql(hosts: &str, primary_ip: &str) -> String {
    let guesses = GUESSED_NAME_SOURCES_SQL.as_str();
    format!(
        "COALESCE(\
         CASE WHEN {hosts}.name_source NOT IN ({guesses}) THEN NULLIF({hosts}.name, '') END, \
         NULLIF({hosts}.hostname, ''), \
         NULLIF({hosts}.sys_name, ''), \
         NULLIF({hosts}.chassis_id, ''), \
         CASE WHEN {hosts}.name_source IN ({guesses}) THEN NULLIF({hosts}.name, '') END, \
         host({primary_ip}.ip_address), '')"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::attributes::{HostHostnameValue, HostSysNameValue};
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::Attributed;

    fn hostname(value: &str, source: AttributeSource) -> Option<Attributed<HostHostnameValue>> {
        Some(Attributed::new(
            HostHostnameValue(value.to_string()),
            source,
        ))
    }

    /// The editor shows the evidence under a name, not only the name. A rung that lost still has
    /// to report what it holds and where it came from.
    #[test]
    fn a_winning_rung_leaves_the_rungs_below_it_readable() {
        let addresses = [crate::server::shared::types::examples::ip_address()];
        let mut host = crate::server::shared::types::examples::host();
        host.base.name = HostName::manual("Core Switch".to_string());
        host.base.hostname = hostname("switch.lan", AttributeSource::ReverseDns);
        host.base.sys_name = Some(Attributed::new(
            HostSysNameValue("core-sw-01".to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        ));
        host.base.chassis_id = None;

        let ladder = host.name_ladder(&addresses);
        let entry = |rung| ladder.iter().find(|e| e.rung == rung).unwrap();

        assert_eq!(
            resolve_name_ladder(&ladder),
            Some(("Core Switch".to_string(), HostNameRung::Name))
        );
        assert_eq!(
            entry(HostNameRung::SysName).source,
            Some(AttributeSource::Probe(ClientProbe::Snmp)),
            "a rung that lost keeps its provenance"
        );
        assert_eq!(
            entry(HostNameRung::Hostname).source,
            Some(AttributeSource::ReverseDns),
            "a hostname carries the source that produced it"
        );
        assert_eq!(
            entry(HostNameRung::ChassisId),
            &HostNameLadderEntry {
                rung: HostNameRung::ChassisId,
                value: None,
                source: None,
            },
            "an absent value has no source, even though the response defaults one to Unspecified"
        );
    }

    /// A guessed name yields to every identifier and outranks only the address. Walked for each
    /// kind of guess a host can hold, because each reaches `name` by a different path.
    #[test]
    fn a_guessed_name_yields_to_the_identifiers_and_beats_only_the_address() {
        let addresses = [crate::server::shared::types::examples::ip_address()];
        for guess in [
            HostName::from_service("SSH".to_string()),
            HostName::unattributed("office-daemon".to_string()),
        ] {
            let mut host = crate::server::shared::types::examples::host();
            host.base.name = guess.clone();
            host.base.sys_name = None;
            host.base.chassis_id = None;

            host.base.hostname = hostname("nas.lan", AttributeSource::DaemonSelfReport);
            assert_eq!(
                host.resolved_name(&addresses),
                Some(("nas.lan".to_string(), HostNameRung::Hostname)),
                "{guess:?} must not hide the hostname"
            );

            host.base.hostname = None;
            assert_eq!(
                host.resolved_name(&addresses),
                Some((guess.value().to_string(), HostNameRung::Name)),
                "{guess:?} still beats the bare address"
            );
        }
    }

    /// The address rung follows the lease: the address discovery saw last titles the host, and
    /// position decides only between addresses seen at the same moment.
    #[test]
    fn the_address_rung_follows_the_most_recently_seen_address() {
        let mut host = crate::server::shared::types::examples::host();
        host.base.name = HostName::unnamed();
        host.base.hostname = None;
        host.base.sys_name = None;
        host.base.chassis_id = None;

        let mut old = crate::server::shared::types::examples::ip_address();
        old.base.position = 0;
        let mut moved = old.clone();
        moved.base.ip_address = "192.168.1.101".parse().unwrap();
        moved.base.position = 1;
        moved.last_seen_at = old.last_seen_at + chrono::Duration::hours(1);

        let title = |addresses: &[IPAddress]| host.resolved_name(addresses).map(|(value, _)| value);
        assert_eq!(
            title(&[old.clone(), moved.clone()]).as_deref(),
            Some("192.168.1.101"),
            "the lease moved, so the newer address titles the host"
        );

        moved.last_seen_at = old.last_seen_at;
        assert_eq!(
            title(&[moved.clone(), old.clone()]).as_deref(),
            Some(old.base.ip_address.to_string().as_str()),
            "seen together, the lower position wins whatever order they arrive in"
        );
    }

    /// An unnamed host's stored name carries `Unspecified`, and the ladder must report the rung
    /// as empty rather than as an unattributed name.
    #[test]
    fn an_unnamed_host_has_an_empty_name_rung() {
        let mut host = crate::server::shared::types::examples::host();
        host.base.name = HostName::unnamed();

        let ladder = host.name_ladder(&[]);
        let name = ladder
            .iter()
            .find(|e| e.rung == HostNameRung::Name)
            .unwrap();
        assert_eq!(name.value, None);
        assert_eq!(name.source, None);
    }

    /// The SQL ranks exactly the sources the Rust ladder demotes. Checked against the serialised
    /// form each source is stored in, since that is what the SQL compares.
    #[test]
    fn the_sql_demotes_exactly_the_sources_the_ladder_demotes() {
        let sql = display_name_sql("h", "pi");
        for source in AttributeSource::all() {
            let literal = format!("'{}'::jsonb", serde_json::to_string(&source).unwrap());
            assert_eq!(
                GUESSED_NAME_SOURCES_SQL.contains(&literal),
                is_guessed_name(source),
                "{source} is listed in the SQL guess set iff the ladder demotes it"
            );
        }
        assert!(sql.contains(GUESSED_NAME_SOURCES_SQL.as_str()));
    }
}
