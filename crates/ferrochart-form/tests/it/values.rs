// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The wire the entered values are published on.
//!
//! The values travel the same seam as the form definition: a renderer collects
//! them and a server judges and commits them, so what a renderer writes is
//! pinned here beside the definition it was collected against. The document
//! below is hand-built rather than derived, for the reason `wire.rs` gives.
//!
//! Two assertions, the pair `wire.rs` carries. The round trip catches a
//! document that no longer reads back, and the snapshot catches one that still
//! reads back here and no longer means the same thing to a consumer written
//! against the old bytes.
//!
//! The occurrence path is the part with a history: a value keyed by a flat
//! slot lands in the wrong repeat of a repeating group, so the document below
//! carries entries that differ only by their path.
//!
//! Read a snapshot change with `cargo insta review` before accepting it, and
//! decide whether it is a `FORMAT_VERSION` bump (`.claude/rules/testing.md`).

use ferrochart_form::ids::{LocalCode, RmAttributeName, RmTypeName};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::values::{Datum, Entered, FormValues, Slot, ValueEntry};

fn field(node_id: &str) -> NodeKey {
    NodeKey::root().child(KeyStep {
        rm_attribute: RmAttributeName::new("items"),
        node_id: Some(LocalCode::new(node_id)),
        archetype_id: None,
        rm_type: RmTypeName::new("ELEMENT"),
        pinned_name: None,
        sibling_ordinal: 0,
    })
}

/// One set of entries carrying every published shape that holds a number, a
/// null flavour, a nested interval, and two repeats of one field under two
/// occurrence paths.
///
/// Synthetic content invented for this test. No patient data.
fn values() -> FormValues {
    let mut values = FormValues::new();

    // The same field under two occurrence paths, which is what a flat slot
    // cannot tell apart.
    values.set_in(
        field("at0001"),
        vec![0],
        0,
        Entered::Value(Datum::Quantity {
            magnitude: 37.2,
            units: "Cel".to_owned(),
            precision: Some(1),
        }),
    );
    values.set_in(
        field("at0001"),
        vec![1],
        0,
        Entered::Value(Datum::Quantity {
            magnitude: 36.8,
            units: "Cel".to_owned(),
            precision: Some(1),
        }),
    );

    // Two repeats of one field under one path, which is the other index.
    values.set_in(
        field("at0002"),
        vec![1, 0],
        0,
        Entered::Value(Datum::Text("a synthetic entry".to_owned())),
    );
    values.set_in(
        field("at0002"),
        vec![1, 0],
        1,
        Entered::Value(Datum::Text("a second synthetic entry".to_owned())),
    );

    values.set(
        field("at0003"),
        Entered::Value(Datum::Coded {
            terminology: "local".to_owned(),
            code: "at0010".to_owned(),
            rubric: "a synthetic rubric".to_owned(),
        }),
    );
    values.set(
        field("at0004"),
        Entered::Value(Datum::Scale {
            terminology: "local".to_owned(),
            code: "at0011".to_owned(),
            rubric: "a synthetic symbol".to_owned(),
            value: 1.5,
        }),
    );
    values.set(
        field("at0005"),
        Entered::Value(Datum::Proportion {
            numerator: 1.0,
            denominator: 4.0,
            kind: 2,
            precision: Some(2),
        }),
    );
    values.set(
        field("at0006"),
        Entered::Value(Datum::Interval {
            lower: Some(Box::new(Datum::Count(1))),
            upper: Some(Box::new(Datum::Count(9))),
            lower_included: true,
            upper_included: false,
        }),
    );
    values.set(
        field("at0007"),
        Entered::Value(Datum::DateTime("2026-09-08T12:00:00Z".to_owned())),
    );

    // The other half of the ELEMENT invariant: a reason there is no value.
    values.set(
        field("at0008"),
        Entered::Null {
            code: LocalCode::new("271"),
            reason: Some("no information".to_owned()),
        },
    );

    values
}

#[test]
fn a_set_of_entered_values_round_trips_through_serde_json() {
    let entered = values();
    let json = serde_json::to_string(&entered).unwrap();
    let back: FormValues = serde_json::from_str(&json).unwrap();
    assert_eq!(back, entered);
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn the_occurrence_path_survives_the_round_trip() {
    // The two entries below differ in nothing but their path, so a wire that
    // dropped it would collapse them into one and lose a value.
    let entered = values();
    let json = serde_json::to_string(&entered).unwrap();
    let back: FormValues = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), entered.len());
    assert_eq!(
        back.get_in(&field("at0001"), &[1], 0),
        entered.get_in(&field("at0001"), &[1], 0)
    );
    assert_ne!(
        back.get_in(&field("at0001"), &[0], 0),
        back.get_in(&field("at0001"), &[1], 0)
    );
}

#[test]
fn the_values_write_the_same_bytes_every_time() {
    // The map is ordered, which is what makes the snapshot below a contract
    // rather than one recording of an arbitrary order.
    assert_eq!(
        serde_json::to_string(&values()).unwrap(),
        serde_json::to_string(&values()).unwrap()
    );
}

#[test]
fn one_entry_spells_its_slot_out_beside_its_value() {
    // The array element a renderer writes, so its member names are pinned
    // independently of the whole document.
    let entry = ValueEntry {
        key: field("at0001"),
        group_path: vec![2],
        occurrence: 1,
        entered: Entered::Value(Datum::Boolean(true)),
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["group_path"], serde_json::json!([2]));
    assert_eq!(json["occurrence"], serde_json::json!(1));
    // Externally tagged since format version 2: a variant nests under its own
    // name, so `Entered::Value(Datum::Boolean(true))` is `{"value":{"boolean":
    // true}}` rather than a tag beside a payload.
    assert_eq!(json["entered"]["value"]["boolean"], serde_json::json!(true));

    let slot = Slot {
        key: field("at0001"),
        group_path: vec![2],
        occurrence: 1,
    };
    let back: Slot = serde_json::from_value(serde_json::to_value(&slot).unwrap()).unwrap();
    assert_eq!(back, slot);
}

#[test]
fn the_entered_value_wire_is_unchanged() {
    insta::assert_snapshot!(
        "form_values_wire",
        serde_json::to_string_pretty(&values()).unwrap()
    );
}
