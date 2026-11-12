//! TP-Link member-port strings (GH #847): `1/0/1-3,1/0/7`, with LAG entries among them.

use super::tplink::{TplinkPortItem, TplinkPortList, is_tplink_agent};

fn names(list: &str) -> Vec<String> {
    TplinkPortList::from(list).port_names().collect()
}

fn without_a_port(list: &str) -> Vec<String> {
    TplinkPortList::from(list)
        .without_a_port()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_single_port_names_itself() {
    assert_eq!(names("1/0/4"), ["1/0/4"]);
}

#[test]
fn a_range_names_every_port_in_it_inclusive() {
    assert_eq!(names("1/0/1-3"), ["1/0/1", "1/0/2", "1/0/3"]);
    // Two digits on one side only: the range is numeric, not a string comparison.
    assert_eq!(names("1/0/8-10"), ["1/0/8", "1/0/9", "1/0/10"]);
}

#[test]
fn a_mixed_list_keeps_its_order() {
    assert_eq!(names("1/0/1-3,1/0/7"), ["1/0/1", "1/0/2", "1/0/3", "1/0/7"]);
    assert_eq!(
        names("1/0/5-6, 2/0/8-9"),
        ["1/0/5", "1/0/6", "2/0/8", "2/0/9"]
    );
}

#[test]
fn an_empty_string_names_nothing() {
    assert!(names("").is_empty());
    assert!(without_a_port("").is_empty());
    assert_eq!(TplinkPortList::from(" , "), TplinkPortList::default());
}

#[test]
fn a_lag_entry_names_no_port_and_is_kept_for_the_count() {
    assert_eq!(names("1/0/1-2,LAG1,1/0/7"), ["1/0/1", "1/0/2", "1/0/7"]);
    assert_eq!(
        without_a_port("1/0/1-2,LAG1,lag2-3,1/0/7"),
        ["LAG1", "lag2-3"]
    );
    assert_eq!(names("LAG1"), Vec::<String>::new());
}

#[test]
fn an_item_that_is_no_port_range_is_unreadable_rather_than_guessed_at() {
    for item in ["7", "1/0/x", "1/0/3-1", "1/0/1-", "/4", "1/0/1-2-3"] {
        assert_eq!(
            TplinkPortList::from(item).0,
            [TplinkPortItem::Unreadable(item.to_string())],
            "{item}"
        );
    }
}

#[test]
fn a_range_wider_than_any_switch_is_not_expanded() {
    assert!(names("1/0/1-4000000000").is_empty());
    assert_eq!(without_a_port("1/0/1-4000000000"), ["1/0/1-4000000000"]);
}

#[test]
fn only_an_agent_under_tplinks_enterprise_number_is_a_tplink_agent() {
    assert!(is_tplink_agent(Some("1.3.6.1.4.1.11863.5.214")));
    // Cisco, an enterprise number that merely starts with the same digits, and no answer at all.
    assert!(!is_tplink_agent(Some("1.3.6.1.4.1.9.1.1208")));
    assert!(!is_tplink_agent(Some("1.3.6.1.4.1.118630.1")));
    assert!(!is_tplink_agent(Some("1.3.6.1.2.1.11863")));
    assert!(!is_tplink_agent(None));
}
