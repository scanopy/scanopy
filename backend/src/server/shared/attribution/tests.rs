//! The ordering invariants and the applier's rules.
//!
//! Deliberately no per-variant assertions on `rank()` or `method()`. Those are exhaustive matches,
//! so the compiler already refuses a source that has not been classified, and restating each arm
//! here would break on every intended edit while catching no regression. What is tested is the
//! properties the ladder is supposed to have and the behaviour the applier is supposed to produce.

use super::*;
use crate::attributed_value;
use crate::server::services::r#impl::patterns::ClientProbe;

attributed_value! {
    /// A refreshable string, standing in for `model` and the rest of the mutable set.
    struct TestValue(String) as TestAttributed {
        key: "value",
        source_key: "value_source",
        schema_name: "TestValue",
        refreshable: true,
        blank: |v: &String| v.trim().is_empty(),
        schema: string_schema("A test value."),
    }
}

attributed_value! {
    /// An immutable string, standing in for `serial_number` and `manufacturer`.
    struct FixedValue(String) as FixedAttributed {
        key: "fixed",
        source_key: "fixed_source",
        schema_name: "FixedValue",
        refreshable: false,
        blank: |v: &String| v.trim().is_empty(),
        schema: string_schema("A test value that does not change."),
    }
}

fn at(value: &str, source: AttributeSource) -> TestAttributed {
    TestAttributed::new(TestValue(value.to_string()), source)
}

/// Two carriers flattened into one struct beside an ordinary field — the shape `HostBase` takes,
/// and the reason the carrier's `Deserialize` must call `deserialize_map` rather than
/// `deserialize_struct`. Reading goes through this rather than through `Attributed` directly
/// because `Attributed` deliberately has no blanket `Deserialize`: every field has to name
/// `optional` or `required`, so no field can silently acquire a policy it did not choose.
#[derive(Debug, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
struct Parent {
    plain: String,
    #[serde(flatten, deserialize_with = "super::optional")]
    value: Option<TestAttributed>,
    #[serde(flatten, deserialize_with = "super::optional")]
    fixed: Option<FixedAttributed>,
}

// ---------------------------------------------------------------------------
// The ladder
// ---------------------------------------------------------------------------

/// The axis the whole mechanism turns on: a person's intent outranks a machine's reading, whatever
/// the machine's vantage point. SNMP `sysName` is the case that proves it — a queried fact that
/// should still lose to a name someone typed into a controller, which is only reported.
#[test]
fn every_human_authored_source_outranks_every_machine_one() {
    let (human, machine): (Vec<_>, Vec<_>) = AttributeSource::all()
        .into_iter()
        // `Unspecified` is below everything by construction and belongs to neither side.
        .filter(|s| *s != AttributeSource::Unspecified)
        .partition(|s| s.authorship() == Authorship::Human);

    let weakest_human = human.iter().map(|s| s.rank()).min().expect("human sources");
    let strongest_machine = machine
        .iter()
        .map(|s| s.rank())
        .max()
        .expect("machine sources");

    assert!(
        weakest_human > strongest_machine,
        "authorship must dominate binding: weakest human rung {weakest_human} \
         does not outrank strongest machine rung {strongest_machine}"
    );
}

/// `Unspecified` is what an unreadable row and a pre-provenance payload both degrade to, so it has
/// to be unable to displace anything and to be displaceable by everything.
#[test]
fn unspecified_is_strictly_below_every_other_source() {
    let floor = AttributeSource::Unspecified.rank();
    for source in AttributeSource::all() {
        if source == AttributeSource::Unspecified {
            continue;
        }
        assert!(
            source.rank() > floor,
            "{source} ranks at or below Unspecified"
        );
    }
}

/// Nothing discovery reads may displace what a person typed into Scanopy. This is the rule that
/// makes `model` safe to refresh at all.
#[test]
fn manual_outranks_every_discovered_source() {
    let manual = AttributeSource::Manual.rank();
    for source in AttributeSource::all() {
        if source == AttributeSource::Manual {
            continue;
        }
        assert!(source.rank() < manual, "{source} ranks at or above Manual");
    }
}

/// Announced and Reported tie for machine-emitted values, deliberately. Each fails the binding test
/// in a different, incommensurable way — the right speaker with no proof it spoke, versus proof and
/// the wrong speaker — so neither dominates and the applier refuses to pick between them.
#[test]
fn announced_and_reported_tie_for_machines_and_do_not_for_people() {
    // A Sonos announcing its own model, versus a router's ARP cache describing somebody else.
    assert_eq!(
        AttributeSource::LldpChassisId.rank(),
        AttributeSource::ForwardingTable.rank()
    );

    // Two human-authored values are not in dispute about a fact — both carry the same deliberate
    // intent — so the only question left is whether the channel can be trusted to have carried it,
    // and a forgeable link-local broadcast loses to an authenticated read.
    assert!(
        AttributeSource::DnsSdInstanceName.rank()
            < AttributeSource::Authored(ClientProbe::UnifiController).rank()
    );
}

/// A device's own product protocol beats a generic MIB, which beats a controller describing
/// something it manages. This is the ordering the whole item exists to establish: ENTITY-MIB
/// firmware has to be able to displace a controller's, and an industrial probe's to displace both.
#[test]
fn a_devices_own_protocol_outranks_a_mib_which_outranks_a_controller() {
    let native = AttributeSource::Probe(ClientProbe::EtherNetIp).rank();
    let queried = AttributeSource::Probe(ClientProbe::Snmp).rank();
    let reported = AttributeSource::Probe(ClientProbe::UnifiController).rank();

    assert!(reported < queried, "SNMP must outrank a controller");
    assert!(queried < native, "a native protocol must outrank SNMP");
}

/// PROFINET DCP is a bare `AttributeSource` variant, not `Probe(ClientProbe)` (`ClientProbe` is
/// scoped to TCP/UDP application probes reached over an already-open port; DCP is raw L2 with no
/// port at all), but it must still land at the same `Native` tier a device's own protocol
/// occupies via the `Probe` path above — outranking `ArpReply`, a third-party inference about the
/// same MAC, for the same reason EtherNetIp outranks SNMP: the protocol is the subject's own.
#[test]
fn profinet_dcp_outranks_arp_for_the_same_field() {
    assert!(
        AttributeSource::ArpReply.rank() < AttributeSource::ProfinetDcp.rank(),
        "a directed DCP identify exchange must outrank an inferred ARP reply"
    );
}

/// A value we synthesised from an identifier is an inference, whatever transport carried the
/// identifier. `"CIP vendor 1"` is our own construction, so it must not displace a manufacturer
/// name SNMP read off the device — even though EtherNet/IP outranks SNMP for what the device
/// actually said.
#[test]
fn a_synthesised_vendor_string_does_not_outrank_a_read_one() {
    assert!(
        AttributeSource::CipVendorId.rank() < AttributeSource::Probe(ClientProbe::Snmp).rank(),
        "a synthesised vendor string must not displace one read from the device"
    );
}

// ---------------------------------------------------------------------------
// The applier
// ---------------------------------------------------------------------------

/// The behaviour this item was taken for. First-write-wins meant the probe that happened to run
/// first owned the value for good; a stronger source now displaces a weaker one whenever it reads.
#[test]
fn a_stronger_source_displaces_a_weaker_one() {
    let mut slot = None;

    assert!(TestAttributed::apply(
        &mut slot,
        at(
            "Cisco Switch",
            AttributeSource::Probe(ClientProbe::UnifiController)
        ),
    ));
    assert!(TestAttributed::apply(
        &mut slot,
        at(
            "WS-C2960X-48FPD-L",
            AttributeSource::Probe(ClientProbe::Snmp)
        ),
    ));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("WS-C2960X-48FPD-L")
    );
}

/// The converse, and the reason a weak source can be trusted to write at all: having spoken once
/// does not let it overwrite something better later.
#[test]
fn a_weaker_source_never_displaces_a_stronger_one() {
    let mut slot = None;

    TestAttributed::apply(
        &mut slot,
        at(
            "WS-C2960X-48FPD-L",
            AttributeSource::Probe(ClientProbe::Snmp),
        ),
    );
    assert!(!TestAttributed::apply(
        &mut slot,
        at(
            "Cisco Switch",
            AttributeSource::Probe(ClientProbe::UnifiController)
        ),
    ));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("WS-C2960X-48FPD-L")
    );
}

/// Nothing a scan reads displaces a value a person entered.
#[test]
fn nothing_displaces_a_manual_value() {
    let mut slot = None;
    TestAttributed::apply(&mut slot, at("typed-by-a-person", AttributeSource::Manual));

    for source in AttributeSource::all() {
        if source == AttributeSource::Manual {
            continue;
        }
        assert!(
            !TestAttributed::apply(&mut slot, at("read-by-a-scan", source)),
            "{source} displaced a manual value"
        );
    }

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("typed-by-a-person")
    );
}

/// Two sources at one rung that are not the same source must not flap the value depending on which
/// integration finished first in a given scan. The case is real: a device adopted by both a UniFi
/// controller and an Instant On portal has two human-authored names at the same rung.
#[test]
fn equal_rungs_from_different_sources_do_not_displace_each_other() {
    let unifi = AttributeSource::Authored(ClientProbe::UnifiController);
    let instant_on = AttributeSource::Authored(ClientProbe::InstantOn);
    assert_eq!(unifi.rank(), instant_on.rank(), "precondition: same rung");

    let mut slot = None;
    TestAttributed::apply(&mut slot, at("Core Switch", unifi));
    assert!(!TestAttributed::apply(
        &mut slot,
        at("Cupboard Switch", instant_on)
    ));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("Core Switch")
    );
}

/// The same source re-reading is how a firmware revision follows an upgrade, and how a controller
/// rename propagates on the next sync. Equal rung, same source, so it lands.
#[test]
fn the_same_source_may_refresh_its_own_value() {
    let snmp = AttributeSource::Probe(ClientProbe::Snmp);
    let mut slot = None;

    TestAttributed::apply(&mut slot, at("16.12.04", snmp));
    assert!(TestAttributed::apply(&mut slot, at("17.03.01", snmp)));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("17.03.01")
    );
}

/// An immutable value does not move on a re-read from the same source: a different serial number
/// means a different device, not a device whose serial changed.
#[test]
fn an_immutable_value_does_not_move_on_a_re_read() {
    let snmp = AttributeSource::Probe(ClientProbe::Snmp);
    let mut slot = None;

    FixedAttributed::apply(
        &mut slot,
        FixedAttributed::new(FixedValue("FOC1234X5YZ".into()), snmp),
    );
    assert!(!FixedAttributed::apply(
        &mut slot,
        FixedAttributed::new(FixedValue("FOC9999Z9ZZ".into()), snmp),
    ));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("FOC1234X5YZ")
    );
}

/// Immutable governs re-reads, not corrections. A stronger source may still fix a serial a weak one
/// got wrong — otherwise the first source to speak would own it for good, which is the behaviour
/// being removed.
#[test]
fn a_stronger_source_still_corrects_an_immutable_value() {
    let mut slot = None;

    FixedAttributed::apply(
        &mut slot,
        FixedAttributed::new(
            FixedValue("guessed".into()),
            AttributeSource::Probe(ClientProbe::UnifiController),
        ),
    );
    assert!(FixedAttributed::apply(
        &mut slot,
        FixedAttributed::new(
            FixedValue("FOC1234X5YZ".into()),
            AttributeSource::Probe(ClientProbe::Snmp),
        ),
    ));

    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("FOC1234X5YZ")
    );
}

/// `upsert_host` publishes an Updated event and triggers a topology rebuild off this return value,
/// so a scan that learns nothing new has to report no change rather than write silently.
#[test]
fn reapplying_an_identical_pair_reports_no_change() {
    let snmp = AttributeSource::Probe(ClientProbe::Snmp);
    let mut slot = None;

    assert!(TestAttributed::apply(&mut slot, at("WS-C2960X", snmp)));
    assert!(!TestAttributed::apply(&mut slot, at("WS-C2960X", snmp)));
}

/// The same value arriving from a better source is still a change worth recording: the value reads
/// the same, but it is now protected from the rungs in between.
#[test]
fn the_same_value_from_a_higher_rung_is_recorded() {
    let mut slot = None;

    TestAttributed::apply(
        &mut slot,
        at(
            "switch.lan",
            AttributeSource::Probe(ClientProbe::UnifiController),
        ),
    );
    assert!(TestAttributed::apply(
        &mut slot,
        at("switch.lan", AttributeSource::Probe(ClientProbe::Snmp)),
    ));
}

/// A blank candidate is an absent value, not a value. It must never displace a real one however
/// highly it claims to rank — the failure that shipped a host labelled with an address it no longer
/// held was of this shape.
#[test]
fn a_blank_candidate_never_displaces_a_real_value() {
    let mut slot = None;
    TestAttributed::apply(
        &mut slot,
        at("Core Switch", AttributeSource::Probe(ClientProbe::Snmp)),
    );

    assert!(!TestAttributed::apply(
        &mut slot,
        at("   ", AttributeSource::Manual)
    ));
    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("Core Switch")
    );
}

/// The mirror: a blank incumbent is an absent value wearing a source, so it must not block a real
/// one. Without this a host whose name arrived empty at a high rung could never be named.
#[test]
fn a_blank_incumbent_does_not_block_a_real_value() {
    let mut slot = Some(at("", AttributeSource::Manual));

    assert!(TestAttributed::apply(
        &mut slot,
        at(
            "Core Switch",
            AttributeSource::Probe(ClientProbe::UnifiController)
        ),
    ));
    assert_eq!(
        slot.as_ref().map(|s| s.value().0.as_str()),
        Some("Core Switch")
    );
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// The wire and column shape: two flat keys per carrier, so a value stays a bare scalar at the top
/// level. Daemons at 0.17.11 and earlier POST the value alone, and `name` still has to be the
/// column `ORDER BY` and the free-text host search read.
#[test]
fn each_pair_serialises_as_two_flat_keys_beside_its_siblings() {
    let json = serde_json::to_value(Parent {
        plain: "untouched".into(),
        value: Some(at(
            "Core Switch",
            AttributeSource::Authored(ClientProbe::UnifiController),
        )),
        fixed: Some(FixedAttributed::new(
            FixedValue("FOC1234X5YZ".into()),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
    })
    .expect("serialises");

    assert_eq!(json["plain"], "untouched");
    assert_eq!(json["value"], "Core Switch");
    assert_eq!(json["value_source"]["Authored"], "UnifiController");
    assert_eq!(json["fixed"], "FOC1234X5YZ");
    assert_eq!(json["fixed_source"]["Probe"], "Snmp");
}

/// The mechanic the carrier's `deserialize_map` exists for: with several carriers flattened into
/// one struct, each has to see the whole remaining map and take only its own two keys. Reading
/// through `deserialize_struct` instead would let the first carrier consume the rest's keys, and
/// the later fields would silently read as absent.
#[test]
fn sibling_carriers_do_not_consume_each_others_keys() {
    let parent: Parent = serde_json::from_value(serde_json::json!({
        "plain": "untouched",
        "value": "Core Switch",
        "value_source": { "Authored": "UnifiController" },
        "fixed": "FOC1234X5YZ",
        "fixed_source": { "Probe": "Snmp" },
    }))
    .expect("deserialises");

    assert_eq!(parent.plain, "untouched");
    assert_eq!(
        parent.value.as_ref().map(|v| v.value().0.as_str()),
        Some("Core Switch")
    );
    assert_eq!(
        parent.value.as_ref().map(|v| v.source()),
        Some(AttributeSource::Authored(ClientProbe::UnifiController))
    );
    assert_eq!(
        parent.fixed.as_ref().map(|v| v.value().0.as_str()),
        Some("FOC1234X5YZ")
    );
    assert_eq!(
        parent.fixed.as_ref().map(|v| v.source()),
        Some(AttributeSource::Probe(ClientProbe::Snmp))
    );
}

/// A daemon predating provenance sends the value alone. It is real but unattributable, so it enters
/// at the bottom and cannot displace anything whose source we know.
#[test]
fn a_payload_without_a_source_is_unattributable_but_keeps_its_value() {
    let parent: Parent =
        serde_json::from_value(serde_json::json!({ "plain": "p", "value": "nas.lan" }))
            .expect("deserialises");

    let carrier = parent.value.expect("value present");
    assert_eq!(carrier.value().0, "nas.lan");
    assert_eq!(carrier.source(), AttributeSource::Unspecified);
}

/// An absent key and a blank value are the same thing: no usable value. A rung with nothing to
/// attribute is not a fact, so it must not reach the slot and claim a rank there.
#[test]
fn an_absent_or_blank_value_reads_as_nothing() {
    let absent: Parent =
        serde_json::from_value(serde_json::json!({ "plain": "p" })).expect("deserialises");
    assert_eq!(absent.value, None);

    let blank: Parent = serde_json::from_value(serde_json::json!({
        "plain": "p",
        "value": "   ",
        "value_source": "Manual",
    }))
    .expect("deserialises");
    assert_eq!(blank.value, None);
}

/// A source a newer binary wrote must not fail the row it sits in — a `from_row` error does not
/// lose a field, it loses the whole entity and therefore the whole page. Degrading keeps the value
/// and lets the next real reading correct the rung.
#[test]
fn an_unrecognised_source_degrades_rather_than_failing_the_row() {
    let unknown_variant: AttributeSource =
        serde_json::from_value(serde_json::json!("SomethingLaterAdded")).expect("degrades");
    assert_eq!(unknown_variant, AttributeSource::Unspecified);

    // The likelier case, since adding a probe is how this enum grows.
    let unknown_probe: AttributeSource =
        serde_json::from_value(serde_json::json!({ "Probe": "SomeNewProtocol" }))
            .expect("degrades");
    assert_eq!(unknown_probe, AttributeSource::Unspecified);

    // A variant this binary knows only as fieldless, carrying something in a newer one.
    let unknown_payload: AttributeSource =
        serde_json::from_value(serde_json::json!({ "SomethingLaterAdded": "Whatever" }))
            .expect("degrades");
    assert_eq!(unknown_payload, AttributeSource::Unspecified);
}

/// Tolerating unknown identifiers is not the same as tolerating anything: a payload that is not a
/// source at all is still an error, so the leniency above cannot hide a genuine bug.
#[test]
fn a_malformed_source_is_still_an_error() {
    // Neither of the two shapes a source has.
    assert!(serde_json::from_value::<AttributeSource>(serde_json::json!(7)).is_err());
    assert!(serde_json::from_value::<AttributeSource>(serde_json::json!(["Manual"])).is_err());
    // The right shape carrying something that is not a probe name.
    assert!(serde_json::from_value::<AttributeSource>(serde_json::json!({ "Probe": 7 })).is_err());
}

/// Round-trips through the shape the column holds, for every source the binary can write.
#[test]
fn every_source_survives_a_round_trip() {
    for source in AttributeSource::all() {
        let json = serde_json::to_value(source).expect("serialises");
        let back: AttributeSource = serde_json::from_value(json.clone()).expect("deserialises");
        assert_eq!(back, source, "{source} did not survive {json}");
    }
}

/// The UI reads the source→tier table out of `AttributeMethod`'s metadata, so every source has to
/// appear under exactly one method or a badge silently stops resolving.
#[test]
fn the_published_tier_table_covers_every_source_exactly_once() {
    use crate::server::shared::types::metadata::TypeMetadataProvider;
    use strum::IntoEnumIterator;

    let mut published: Vec<AttributeSource> = AttributeMethod::iter()
        .flat_map(|method| {
            let metadata = method.metadata();
            let sources = metadata["sources"].clone();
            serde_json::from_value::<Vec<AttributeSource>>(sources).expect("sources deserialise")
        })
        .collect();

    let mut expected = AttributeSource::all();
    published.sort_by_key(|s| s.to_string());
    expected.sort_by_key(|s| s.to_string());

    assert_eq!(published, expected);
}

/// Every source renders with a label, and the `{probe}` slot sits exactly where there is a probe
/// to fill it. A probe-carrying label without the slot would read the same for SNMP and Docker, and
/// a slot anywhere else would reach the operator as a literal `{probe}`.
#[test]
fn every_source_has_a_label_with_a_probe_slot_exactly_where_it_carries_a_probe() {
    use crate::server::shared::types::metadata::TypeMetadataProvider;

    for source in AttributeSource::all() {
        let discriminant = AttributeSourceDiscriminants::from(&source);
        let name = discriminant.name();
        assert!(!name.trim().is_empty(), "{source} has no label");

        let carries_probe = matches!(
            source,
            AttributeSource::Probe(_) | AttributeSource::Authored(_)
        );
        assert_eq!(
            name.contains("{probe}"),
            carries_probe,
            "{source} is labelled {name:?}"
        );
    }
}
