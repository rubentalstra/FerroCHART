// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One case per constraint kind, proving a form never admits what the
//! template refuses.
//!
//! `crates/ferrochart-compile/tests/it/derive.rs` holds the other end: a
//! constraint the derivation refuses to turn into a field at all. These are
//! the cases a derivation cannot catch, because the value only exists once a
//! clinician has entered one.
//!
//! Every value here is synthetic content invented for the test. No patient
//! data.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::key::NodeKey;
use ferrochart_form::validation::{FailureKind, ValidationReport};
use ferrochart_validate::template::TemplateValidator;
use serde_json::{Value, json};

/// Synthetic content invented for these tests. No patient data.
const GRID: &str = include_str!("../fixtures/ferro_validate_grid.opt");

/// The form and the gate the fixture yields.
fn grid() -> (FormDefinition, TemplateValidator) {
    let template = ferrochart_compile::adl14::from_xml(GRID).expect("the fixture reads");
    let definition = ferrochart_compile::derive::form(&template).expect("the fixture derives");
    let validator = TemplateValidator::from_opt14_xml(GRID).expect("the fixture flattens");
    (definition, validator)
}

/// A COMPOSITION carrying `items` under the fixture's item tree.
///
/// The envelope is what `ferrochart_compose::build` writes, restated here so a
/// case can put a value the builder would refuse into the document and see the
/// gate refuse it.
fn document(items: &[Value]) -> Value {
    json!({
        "_type": "COMPOSITION",
        "name": { "_type": "DV_TEXT", "value": "Ferro validation grid" },
        "archetype_node_id": "openEHR-EHR-OBSERVATION.ferro_validate.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "value": "openEHR-EHR-OBSERVATION.ferro_validate.v1" },
            "template_id": { "value": "openEHR-EHR-OBSERVATION.ferro_validate.v1.0.0" },
            "rm_version": "1.1.0",
        },
        "language": { "terminology_id": { "value": "ISO_639-1" }, "code_string": "en" },
        "territory": { "terminology_id": { "value": "ISO_3166-1" }, "code_string": "NL" },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": { "terminology_id": { "value": "openehr" }, "code_string": "433" },
        },
        "composer": { "_type": "PARTY_SELF" },
        "content": [entry(items)],
    })
}

/// The OBSERVATION the template describes, wrapped as the builder wraps it.
fn entry(items: &[Value]) -> Value {
    json!({
        "_type": "OBSERVATION",
        "name": { "_type": "DV_TEXT", "value": "Ferro validation grid" },
        "archetype_node_id": "openEHR-EHR-OBSERVATION.ferro_validate.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "value": "openEHR-EHR-OBSERVATION.ferro_validate.v1" },
            "rm_version": "1.1.0",
        },
        "language": { "terminology_id": { "value": "ISO_639-1" }, "code_string": "en" },
        "encoding": { "terminology_id": { "value": "IANA_character-sets" }, "code_string": "UTF-8" },
        "subject": { "_type": "PARTY_SELF" },
        "data": {
            "_type": "HISTORY",
            "name": { "_type": "DV_TEXT", "value": "History" },
            "archetype_node_id": "at0001",
            "origin": { "value": "2026-09-07T12:00:00Z" },
            "events": [{
                "_type": "POINT_EVENT",
                "name": { "_type": "DV_TEXT", "value": "Any event" },
                "archetype_node_id": "at0002",
                "time": { "value": "2026-09-07T12:00:00Z" },
                "data": {
                    "_type": "ITEM_TREE",
                    "name": { "_type": "DV_TEXT", "value": "Tree" },
                    "archetype_node_id": "at0003",
                    "items": items,
                },
            }],
        },
    })
}

/// One `ELEMENT` carrying `value`.
fn element(node_id: &str, label: &str, value: &Value) -> Value {
    json!({
        "_type": "ELEMENT",
        "name": { "_type": "DV_TEXT", "value": label },
        "archetype_node_id": node_id,
        "value": value.clone(),
    })
}

/// The mandatory node every case carries, so the case under test is the only
/// thing the gate has to say.
fn required() -> Value {
    element(
        "at0015",
        "Required note",
        &json!({ "_type": "DV_TEXT", "value": "a synthetic note" }),
    )
}

/// The report the gate gives for `items`.
fn judge(items: &[Value]) -> (FormDefinition, ValidationReport) {
    let (definition, validator) = grid();
    let report = validator
        .validate_json(&definition, &document(items))
        .expect("the document is judged");
    (definition, report)
}

/// The key of the field whose terminal step carries `node_id`.
fn key(definition: &FormDefinition, node_id: &str) -> NodeKey {
    definition
        .fields()
        .find(|field| {
            field
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .is_some_and(|code| code.as_str() == node_id)
        })
        .unwrap_or_else(|| panic!("no field {node_id}"))
        .key
        .clone()
}

/// Asserts that `node_id`'s field carries a failure of `kind`.
fn refused(items: &[Value], node_id: &str, kind: FailureKind) {
    let (definition, report) = judge(items);
    let wanted = key(&definition, node_id);
    let found: Vec<_> = report.at(&wanted).map(|failure| failure.kind).collect();
    assert!(
        found.contains(&kind),
        "{node_id} carries {found:?}, not {kind:?}; the whole report is {:#?}",
        report.failures
    );
}

#[test]
fn a_document_the_template_admits_passes() {
    // The control. Without it every case below would pass on a gate that
    // refuses everything.
    let (_, report) = judge(&[required()]);
    assert!(report.is_valid(), "{:#?}", report.failures);
}

#[test]
fn a_quantity_outside_the_range_the_template_states_is_refused() {
    // The template bounds the magnitude to 0..300 mm[Hg].
    refused(
        &[
            required(),
            element(
                "at0010",
                "Pressure",
                &json!({ "_type": "DV_QUANTITY", "magnitude": 900.0, "units": "mm[Hg]" }),
            ),
        ],
        "at0010",
        FailureKind::RangeError,
    );
}

#[test]
fn a_code_outside_the_value_set_the_template_states_is_refused() {
    // The template enumerates at0012 and at0013 and nothing else.
    refused(
        &[
            required(),
            element(
                "at0011",
                "Position",
                &json!({
                    "_type": "DV_CODED_TEXT",
                    "value": "Lying",
                    "defining_code": {
                        "terminology_id": { "value": "local" },
                        "code_string": "at0099",
                    },
                }),
            ),
        ],
        "at0011",
        FailureKind::CodedValue,
    );
}

#[test]
fn a_string_that_does_not_match_the_pattern_the_template_states_is_refused() {
    refused(
        &[
            required(),
            element(
                "at0014",
                "Reference",
                &json!({ "_type": "DV_TEXT", "value": "not a reference" }),
            ),
        ],
        "at0014",
        FailureKind::PatternError,
    );
}

#[test]
fn a_count_outside_the_range_the_template_states_is_refused() {
    refused(
        &[
            required(),
            element(
                "at0017",
                "Tally",
                &json!({ "_type": "DV_COUNT", "magnitude": 99 }),
            ),
        ],
        "at0017",
        FailureKind::RangeError,
    );
}

#[test]
fn a_data_value_of_the_wrong_class_is_refused() {
    // The template constrains at0010 to a DV_QUANTITY. A DV_TEXT there would
    // store a clinical measurement as prose.
    refused(
        &[
            required(),
            element(
                "at0010",
                "Pressure",
                &json!({ "_type": "DV_TEXT", "value": "high" }),
            ),
        ],
        "at0010",
        FailureKind::WrongType,
    );
}

#[test]
fn a_node_the_template_admits_once_is_refused_twice() {
    refused(
        &[
            required(),
            element(
                "at0016",
                "Single note",
                &json!({ "_type": "DV_TEXT", "value": "first" }),
            ),
            element(
                "at0016",
                "Single note",
                &json!({ "_type": "DV_TEXT", "value": "second" }),
            ),
        ],
        "at0016",
        FailureKind::Occurrences,
    );
}

#[test]
fn a_mandatory_node_the_document_leaves_out_is_refused() {
    let (_, report) = judge(&[element(
        "at0010",
        "Pressure",
        &json!({ "_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]" }),
    )]);
    let missing: Vec<_> = report
        .failures
        .iter()
        .filter(|failure| failure.kind == FailureKind::Required)
        .collect();
    assert!(!missing.is_empty(), "{:#?}", report.failures);
    assert!(
        missing
            .iter()
            .any(|failure| failure.path.contains("at0015")),
        "the refusal names the node that is missing: {missing:#?}"
    );
}

#[test]
fn a_container_holding_more_than_its_cardinality_admits_is_refused() {
    // The cluster's items are bounded to one or two, and three is not two.
    let member = |value: &str| {
        json!({
            "_type": "ELEMENT",
            "name": { "_type": "DV_TEXT", "value": "Bundle member" },
            "archetype_node_id": "at0021",
            "value": { "_type": "DV_TEXT", "value": value },
        })
    };
    let (_, report) = judge(&[
        required(),
        json!({
            "_type": "CLUSTER",
            "name": { "_type": "DV_TEXT", "value": "Bundle" },
            "archetype_node_id": "at0020",
            "items": [member("one"), member("two"), member("three")],
        }),
    ]);
    let found: Vec<_> = report
        .failures
        .iter()
        .filter(|failure| failure.kind == FailureKind::Cardinality)
        .collect();
    assert!(!found.is_empty(), "{:#?}", report.failures);
    // A cardinality violation is reported on the attribute rather than on a
    // node, so it lands on the nearest item a renderer can show it against.
    assert!(
        found.iter().any(|failure| failure.key.is_some()),
        "a cardinality refusal reaches no field: {found:#?}"
    );
}

#[test]
fn a_code_outside_its_openehr_terminology_group_is_refused() {
    // `ehr.html` section 5.4.1 binds `COMPOSITION.category` to the openEHR
    // `composition category` group, which no template constrains, so this is
    // the instance pass rather than the template pass.
    let (definition, validator) = grid();
    let mut document = document(&[required()]);
    document["category"]["defining_code"]["code_string"] = json!("999");
    let report = validator
        .validate_json(&definition, &document)
        .expect("the document is judged");
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.kind == FailureKind::Terminology),
        "{:#?}",
        report.failures
    );
}

#[test]
fn a_refusal_the_form_cannot_place_is_reported_rather_than_dropped() {
    // The envelope is not part of the template, so a failure on it resolves
    // to no field. It still reaches the report, with the path it was found
    // at, because a swallowed refusal is the failure mode this gate exists to
    // prevent.
    let (definition, validator) = grid();
    let mut document = document(&[required()]);
    document["category"]["defining_code"]["code_string"] = json!("999");
    let report = validator
        .validate_json(&definition, &document)
        .expect("the document is judged");
    let unplaced: Vec<_> = report.unplaced().collect();
    assert!(!unplaced.is_empty(), "{:#?}", report.failures);
    assert!(
        unplaced.iter().all(|failure| !failure.path.is_empty()),
        "an unplaced failure still names where it was found: {unplaced:#?}"
    );
}
