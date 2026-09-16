use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::base::DaemonMode,
    shared::{entities::EntityDiscriminants, storage::traits::SqlValue},
    users::r#impl::permissions::UserOrgPermissions,
};

use super::traits::Storable;

/// Builder pattern for common WHERE clauses with optional pagination and JOINs.
/// Generic over entity type T to automatically qualify column names with the table name.
#[derive(Clone)]
pub struct StorableFilter<T: Storable> {
    _marker: PhantomData<T>,
    conditions: Vec<String>,
    values: Vec<SqlValue>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
    joins: Vec<String>,
}

impl<T: Storable> Default for StorableFilter<T> {
    fn default() -> Self {
        Self::new()
    }
}

mod constructors;
mod filters;
mod lldp;
mod organization;
mod subnets;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::discovery::r#impl::base::Discovery;
    use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
    use crate::server::hosts::r#impl::base::Host;
    use crate::server::shared::types::entities::EntitySourceDiscriminants;
    use crate::server::shared::types::metadata::HasId;
    use crate::server::snapshots::types::base::Snapshot;
    use crate::server::tags::r#impl::base::Tag;
    use chrono::TimeZone;
    use strum::{IntoEnumIterator, VariantNames};

    /// `live_configs` matches serde tags as raw strings in JSONB, so no amount
    /// of Rust exhaustiveness covers it. Drive the check off the derived
    /// `VARIANTS` lists — they grow automatically — so whoever adds a variant
    /// must decide whether it is a discovery configuration a user owns.
    ///
    /// Both enums matter: a rescan's *run* type is `AdHoc`, so only its
    /// discovery type marks it transient.
    #[test]
    fn live_configs_classifies_every_run_and_discovery_type() {
        let sql = StorableFilter::<Discovery>::new_unfiltered()
            .live_configs()
            .to_where_clause();

        for name in RunType::VARIANTS {
            let expected_excluded = match *name {
                "Historical" => true,
                "Scheduled" | "AdHoc" => false,
                other => panic!(
                    "RunType::{other} is not classified by live_configs. Decide whether it \
                     is a discovery configuration a user owns (leave it in) or a server-managed \
                     row (add it to the filter), then update this test."
                ),
            };
            assert_eq!(
                sql.contains(&format!("'{name}'")),
                expected_excluded,
                "RunType::{name} exclusion mismatch in: {sql}"
            );
        }

        for name in DiscoveryType::VARIANTS {
            let expected_excluded = match *name {
                "Rescan" => true,
                "SelfReport" | "Network" | "Docker" | "Unified" => false,
                other => panic!(
                    "DiscoveryType::{other} is not classified by live_configs. Decide whether a \
                     row of this type is a configuration a user owns (leave it in) or a transient \
                     server-minted row (add it to the filter), then update this test."
                ),
            };
            assert_eq!(
                sql.contains(&format!("'{name}'")),
                expected_excluded,
                "DiscoveryType::{name} exclusion mismatch in: {sql}"
            );
        }
    }

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    #[test]
    fn id_or_lineage_in_emits_or_clause() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&ids);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.id = ANY($1)"),
            "expected id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.lineage_id = ANY($2)"),
            "expected lineage_id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains(" OR "),
            "expected OR-join in: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            2,
            "expected two array params (id_set, lineage_set)"
        );
    }

    #[test]
    fn id_or_lineage_in_empty_input_matches_nothing() {
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&[]);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("FALSE"),
            "empty id list should produce FALSE: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            0,
            "no params should be bound for empty input"
        );
    }

    #[test]
    fn taken_at_lt_emits_less_than_clause() {
        let cutoff = chrono::Utc::now();
        let filter = StorableFilter::<Snapshot>::new_unfiltered().taken_at_lt(cutoff);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("snapshots.taken_at < $1"),
            "expected `taken_at < $N` in: {}",
            where_clause
        );
        assert_eq!(filter.values().len(), 1);
    }

    // An empty cutoff set must match nothing. Falling through to "no
    // condition" would silently return every host as stale — the failure mode
    // that turns a filter bug into a wrong answer rather than an error.
    #[test]
    fn stale_by_network_with_no_cutoffs_matches_nothing() {
        for stale in [true, false] {
            let filter = StorableFilter::<Host>::new_unfiltered().stale_by_network(&[], stale);
            assert_eq!(filter.to_where_clause().trim(), "WHERE FALSE");
            assert!(filter.values().is_empty());
        }
    }

    // Each network contributes its own id + cutoff pair, so a host is compared
    // against its own network's window rather than a single global one.
    #[test]
    fn stale_by_network_binds_a_cutoff_per_network() {
        let cutoffs = vec![
            (uuid::Uuid::new_v4(), ts(0)),
            (uuid::Uuid::new_v4(), ts(1)),
            (uuid::Uuid::new_v4(), ts(2)),
        ];
        let filter = StorableFilter::<Host>::new_unfiltered().stale_by_network(&cutoffs, true);
        assert_eq!(
            filter.values().len(),
            cutoffs.len() * 2,
            "one network id and one cutoff bound per network"
        );
        let where_clause = filter.to_where_clause();
        assert_eq!(
            where_clause.matches("hosts.last_seen_at").count(),
            cutoffs.len(),
            "every network gets its own comparison: {where_clause}"
        );
    }

    // Staleness is only meaningful for entities discovery actually refreshes;
    // both directions of the filter must respect that, or a hand-created host
    // shows up as stale the moment it ages past the window. The guard must also
    // cover exactly what `is_from_discovery` covers: it once listed Discovery
    // and DiscoveryWithMatch only, so an inferred host never went stale here
    // while the digest reported it stale.
    #[test]
    fn stale_by_network_excludes_entities_discovery_never_refreshes() {
        let cutoffs = vec![(uuid::Uuid::new_v4(), ts(0))];
        for stale in [true, false] {
            let where_clause = StorableFilter::<Host>::new_unfiltered()
                .stale_by_network(&cutoffs, stale)
                .to_where_clause();
            assert!(
                where_clause.contains("hosts.source->>'type'"),
                "expected the discovery-managed guard in: {where_clause}"
            );
            for source in EntitySourceDiscriminants::iter() {
                assert_eq!(
                    where_clause.contains(&format!("'{}'", source.id())),
                    source.is_from_discovery(),
                    "{source:?} in the discovery-managed guard: {where_clause}"
                );
            }
        }
    }

    #[test]
    fn live_filter_emits_valid_to_is_null() {
        let filter = StorableFilter::<Tag>::new_unfiltered().live();
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected `valid_to IS NULL` in: {}",
            where_clause
        );
    }

    #[test]
    fn as_of_filter_emits_window_predicate() {
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().as_of(t);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_none_is_live() {
        // `at = None` (live view) must hide closed historical copies, i.e. behave
        // exactly like `.live()`. This is the read-path guard that stops snapshot
        // close-and-clone copies leaking into entity lists as empty-shell dupes.
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(None);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected live predicate in: {}",
            where_clause
        );
        assert!(
            !where_clause.contains("valid_from <="),
            "live view must not emit an as-of window: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_some_is_as_of() {
        // `at = Some(t)` (snapshot view) must read SCD2 state as of `t`.
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(Some(t));
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected as-of lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected as-of upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn created_between_emits_inclusive_range() {
        let f = StorableFilter::<Host>::new().created_between(ts(100), ts(200));
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("created_at"), "condition was: {c}");
        assert!(
            c.contains(">= $1") && c.contains("<= $2"),
            "condition was: {c}"
        );
        assert_eq!(f.values.len(), 2);
        match (&f.values[0], &f.values[1]) {
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => {
                assert_eq!(*a, ts(100));
                assert_eq!(*b, ts(200));
            }
            _ => panic!("expected two Timestamp values"),
        }
    }

    #[test]
    fn updated_between_uses_updated_at_column() {
        let f = StorableFilter::<Host>::new().updated_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("updated_at"));
    }

    #[test]
    fn valid_to_between_uses_valid_to_column() {
        let f = StorableFilter::<Host>::new().valid_to_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("valid_to"));
    }

    #[test]
    fn last_seen_between_uses_last_seen_at_column() {
        let f = StorableFilter::<Host>::new().last_seen_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("last_seen_at"));
    }

    #[test]
    fn between_helpers_advance_param_index() {
        let f = StorableFilter::<Host>::new()
            .created_between(ts(10), ts(20))
            .updated_between(ts(30), ts(40));
        assert_eq!(f.values.len(), 4);
        assert!(f.conditions[0].contains("$1") && f.conditions[0].contains("$2"));
        assert!(f.conditions[1].contains("$3") && f.conditions[1].contains("$4"));
    }

    // A multi-fragment search must cost exactly one bind, with every fragment
    // pointing at that same parameter. Getting this wrong shifts every
    // subsequent filter's placeholder and silently binds the wrong values.
    #[test]
    fn text_search_binds_one_param_shared_by_every_fragment() {
        let filter = StorableFilter::<Host>::new().text_search("web");
        let where_clause = filter.to_where_clause();

        assert_eq!(filter.values().len(), 1, "expected a single bound pattern");
        assert_eq!(
            where_clause.matches("$1").count(),
            Host::search_predicates().len(),
            "every fragment should reference $1: {where_clause}"
        );
        assert!(
            where_clause.contains(" OR "),
            "fragments should be OR-joined: {where_clause}"
        );
    }

    // The search parameter is appended after whatever the filter already
    // bound, so its placeholder index has to follow the existing values.
    #[test]
    fn text_search_placeholder_follows_earlier_values() {
        let network = Uuid::new_v4();
        let filter = StorableFilter::<Host>::new()
            .network_ids(&[network])
            .text_search("web");

        assert_eq!(filter.values().len(), 2);
        let search_condition = filter.conditions.last().unwrap();
        assert!(
            search_condition.contains("$2") && !search_condition.contains("$1"),
            "search should bind $2 after the network filter: {search_condition}"
        );
    }

    // A user typing `%` or `_` means those characters literally. Unescaped,
    // `%` turns the search into "match every row" — the query appears to work
    // while quietly ignoring what was typed.
    #[test]
    fn text_search_escapes_like_wildcards() {
        let filter = StorableFilter::<Host>::new().text_search("100%_x");

        match &filter.values()[0] {
            SqlValue::String(pattern) => assert_eq!(pattern, r"%100\%\_x%"),
            _ => panic!("expected a String pattern"),
        }
    }

    // Fail closed. An entity that hasn't opted into search, or a blank query,
    // must match nothing: falling through to "no condition" would answer the
    // request with the whole table and look like a successful search.
    #[test]
    fn text_search_matches_nothing_without_predicates_or_query() {
        // Snapshot declares no search predicates.
        let unsupported = StorableFilter::<Snapshot>::new().text_search("web");
        assert_eq!(unsupported.conditions, vec!["FALSE".to_string()]);
        assert!(unsupported.values().is_empty());

        for blank in ["", "   "] {
            let empty_query = StorableFilter::<Host>::new().text_search(blank);
            assert_eq!(empty_query.conditions, vec!["FALSE".to_string()]);
            assert!(empty_query.values().is_empty());
        }
    }

    #[test]
    fn user_permissions_in_emits_in_clause() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[UserOrgPermissions::Owner, UserOrgPermissions::Admin]);
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("permissions"));
        assert!(c.contains("IN ($1, $2)"), "condition was: {c}");
        assert_eq!(f.values.len(), 2);
    }

    #[test]
    fn user_permissions_in_empty_emits_false() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[]);
        assert_eq!(f.conditions, vec!["FALSE".to_string()]);
        assert_eq!(f.values.len(), 0);
    }
    /// `find_if_entry_by_mac` narrows to physical ports with this filter, and the ports it exists
    /// to find are the ones with no `if_type` at all — a port learned from a neighbour's
    /// advertisement, or a device known only at the link layer, never had an ifTable walked.
    ///
    /// The hazard is that the obvious predicate is silently wrong rather than broken: in Postgres
    /// `NULL NOT IN (...)` evaluates to NULL, not TRUE, so a bare `NOT IN` drops every row whose
    /// type was never read. No compile error, no failing test, and the rows lost are exactly the
    /// MAC-identified interfaces this narrowing is asked to return. Asserted on the emitted clause
    /// because that is the layer the bug lives at, and asserted here rather than at the one call
    /// site so a second caller cannot reintroduce it.
    #[test]
    fn physical_if_types_admits_an_interface_whose_type_was_never_read() {
        let clause =
            StorableFilter::<crate::server::interfaces::r#impl::base::Interface>::new_unfiltered()
                .physical_if_types()
                .to_where_clause();

        assert!(
            clause.contains("if_type IS NULL"),
            "an unread if_type must survive the physical-port narrowing, or a MAC-identified \
             port silently stops resolving. Clause was: {clause}"
        );
        assert!(
            clause.contains("NOT IN"),
            "the virtual families must still be excluded. Clause was: {clause}"
        );
    }

    /// The Hosts/Services tabs offer "Not Virtualized" / "Not Containerized",
    /// which mean *no parent* rather than a parent with that name. Selecting
    /// only that must therefore ask for NULLs, not match nothing.
    #[test]
    fn virtualization_service_in_expresses_absence_and_membership_separately() {
        let only_absent = StorableFilter::<Host>::new_unfiltered()
            .virtualization_service_in(&[], true)
            .to_where_clause();
        assert!(
            only_absent.contains("virtualization_service_id IS NULL"),
            "selecting only \"not virtualized\" must ask for NULLs. Clause was: {only_absent}"
        );

        let nothing = StorableFilter::<Host>::new_unfiltered()
            .virtualization_service_in(&[], false)
            .to_where_clause();
        assert_eq!(
            nothing.trim(),
            "WHERE FALSE",
            "selecting nothing at all must match nothing, not everything"
        );

        let both = StorableFilter::<Host>::new_unfiltered()
            .virtualization_service_in(&[Uuid::new_v4()], true);
        let clause = both.to_where_clause();
        assert!(
            clause.contains(" OR ") && clause.contains("IS NULL"),
            "a parent id alongside \"not virtualized\" must union the two. Clause was: {clause}"
        );
        assert_eq!(both.values().len(), 1);
    }

    /// A host running several matching services must stay one row: a JOIN would
    /// repeat it and inflate the paginated COUNT(*) that the tab displays.
    #[test]
    fn has_service_named_matches_hosts_once_and_ignores_closed_rows() {
        let filter = StorableFilter::<Host>::new_unfiltered()
            .has_service_named(&["ssh".to_string(), "http".to_string()]);
        let clause = filter.to_where_clause();

        assert!(
            clause.contains("IN (SELECT s.host_id FROM services s"),
            "must match through a subquery, not a row-multiplying JOIN. Clause was: {clause}"
        );
        assert!(
            clause.contains("s.valid_to IS NULL"),
            "superseded SCD2 service rows must not resurrect a host. Clause was: {clause}"
        );
        assert_eq!(filter.values().len(), 2);
    }

    /// `discovery_type` is JSONB and the tab filters on its discriminant, so the
    /// predicate has to read that tag out rather than compare whole documents.
    #[test]
    fn discovery_type_in_reads_the_json_discriminant() {
        let filter =
            StorableFilter::<Discovery>::new_unfiltered().discovery_type_in(&["Snmp".to_string()]);
        let clause = filter.to_where_clause();

        assert!(
            clause.contains("discovery_type->>'type' IN ($1)"),
            "expected a JSON tag comparison, got: {clause}"
        );
        assert_eq!(filter.values().len(), 1);
    }

    /// `source` is JSONB too; the host list filters on its tag, bound rather
    /// than inlined.
    #[test]
    fn source_type_in_reads_the_json_discriminant() {
        let filter = StorableFilter::<Host>::new_unfiltered().source_type_in(&[
            EntitySourceDiscriminants::Inferred,
            EntitySourceDiscriminants::Manual,
        ]);
        let clause = filter.to_where_clause();

        assert!(
            clause.contains("hosts.source->>'type' IN ($1, $2)"),
            "expected a JSON tag comparison, got: {clause}"
        );
        assert_eq!(filter.values().len(), 2);
    }

    /// Every inclusion filter added for server-side field filtering shares the
    /// house rule that an empty selection matches nothing rather than silently
    /// dropping the constraint — otherwise a filter could widen its result.
    #[test]
    fn empty_inclusion_filters_match_nothing() {
        let clauses = [
            StorableFilter::<Host>::new_unfiltered()
                .hidden_in(&[])
                .to_where_clause(),
            StorableFilter::<Host>::new_unfiltered()
                .has_service_named(&[])
                .to_where_clause(),
            StorableFilter::<Discovery>::new_unfiltered()
                .daemon_ids(&[])
                .to_where_clause(),
            StorableFilter::<Discovery>::new_unfiltered()
                .discovery_type_in(&[])
                .to_where_clause(),
            StorableFilter::<Host>::new_unfiltered()
                .source_type_in(&[])
                .to_where_clause(),
        ];

        for clause in clauses {
            assert_eq!(clause.trim(), "WHERE FALSE");
        }
    }
}
