// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The three `DV_ORDERED` reference-range attributes, as display metadata.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 gives every
//! `DV_ORDERED` a `normal_status`, a `normal_range` and
//! `other_reference_ranges`, and section 6.2.3 gives a `REFERENCE_RANGE` a
//! `meaning` and a `range`. None of them is entered: they are shown beside
//! the value so a reader can tell a result from the band it is read against.
//!
//! No operational template in `corpus/templates/ckm` constrains any of the
//! three, so every fixture here is synthetic content invented for this test
//! and carries no patient data.

use ferrochart_compile::derive::error::DeriveError;
use ferrochart_compile::{adl14, derive};
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::{FieldKind, ReferenceRanges};
use ferrochart_form::range::Range;
use ferrochart_form::value::ValueSet;

const RANGES: &str = include_str!("../fixtures/ferro_reference_ranges.opt");
const SHAPE: &str = include_str!("../fixtures/ferro_reference_range_shape.opt");
const UNTYPED: &str = include_str!("../fixtures/ferro_reference_range_untyped.opt");
const EXTRA: &str = include_str!("../fixtures/ferro_reference_range_extra.opt");

fn derived(xml: &str) -> FormDefinition {
    let template = adl14::from_xml(xml).expect("the fixture reads");
    derive::form(&template).expect("the fixture derives")
}

fn refusal(xml: &str) -> DeriveError {
    let template = adl14::from_xml(xml).expect("the fixture reads");
    derive::form(&template).expect_err("the fixture is refused")
}

/// The bands of the field whose terminal key step carries `node_id`.
fn bands(form: &FormDefinition, node_id: &str) -> Option<ReferenceRanges> {
    let field = form
        .fields()
        .find(|field| {
            field
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .is_some_and(|code| code.as_str() == node_id)
        })
        .unwrap_or_else(|| panic!("no field {node_id}"));
    field.reference_ranges.as_deref().cloned()
}

/// The magnitude of a quantity end of a band, with its unit.
fn magnitude(kind: &FieldKind) -> (String, Option<Range<f64>>) {
    match *kind {
        FieldKind::Quantity(ref field) => {
            let unit = field.units.first().expect("the end states one unit");
            (unit.units.clone(), unit.magnitude.clone())
        }
        ref other => panic!("the end is a {} rather than a quantity", other.name()),
    }
}

/// The whole numbers a count end of a band admits.
fn counts(kind: &FieldKind) -> Vec<Range<i64>> {
    match *kind {
        FieldKind::Count(ref field) => field.ranges.clone(),
        ref other => panic!("the end is a {} rather than a count", other.name()),
    }
}

fn point(value: f64) -> Range<f64> {
    Range::new(Some(value), Some(value), true, true)
}

/// The text a `REFERENCE_RANGE.meaning` states.
fn meaning(kind: Option<&FieldKind>) -> Vec<String> {
    match kind {
        Some(FieldKind::Text(field)) => field.options.clone(),
        Some(other) => panic!("the meaning is a {} rather than text", other.name()),
        None => panic!("the fixture states a meaning"),
    }
}

#[test]
fn a_normal_range_is_the_interval_the_template_states() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.1: `normal_range`
    // is a `DV_INTERVAL` over the class the value itself collects, so it
    // carries the value shape of that class rather than a numeric model of
    // its own.
    let form = derived(RANGES);
    let bands = bands(&form, "at0010").expect("the fixture states a normal range");
    assert_eq!(bands.normal_status, None);
    assert!(bands.other.is_empty());
    let range = bands
        .normal_range
        .expect("the fixture states a normal range");
    assert_eq!(range.element_rm_type.as_str(), "DV_QUANTITY");
    assert_eq!(
        magnitude(&range.lower),
        ("g/l".to_owned(), Some(point(120.0)))
    );
    assert_eq!(
        magnitude(&range.upper),
        ("g/l".to_owned(), Some(point(160.0)))
    );
}

#[test]
fn a_normal_status_is_a_coded_value_read_as_any_coded_field_is() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.1 types
    // `normal_status` as a `CODE_PHRASE` "coded by ordinals in series HHH,
    // HH, H, (nothing), L, LL, LLL", so the band carries a value set.
    let form = derived(RANGES);
    let bands = bands(&form, "at0011").expect("the fixture states a normal status");
    assert_eq!(bands.normal_range, None);
    assert!(bands.other.is_empty());
    let status = bands
        .normal_status
        .expect("the fixture states a normal status");
    match status.value_set {
        ValueSet::Enumerated(set) => {
            assert_eq!(set.terminology.as_str(), "openehr");
            let codes: Vec<String> = set
                .options
                .into_iter()
                .map(|option| option.code.code)
                .collect();
            assert_eq!(codes, ["N", "H", "L"]);
            // The openEHR normal-status codes are not the archetype's own, so
            // the template carries no rubric for them and a terminology
            // server supplies the display text.
            assert!(set.needs_display_lookup);
        }
        other => panic!("the status is {other:?} rather than an enumerated set"),
    }
}

#[test]
fn other_reference_ranges_keep_the_order_the_template_states_them_in() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.1 types
    // `other_reference_ranges` as a `List<REFERENCE_RANGE>`, and a list has
    // an order, so nothing here is sorted by meaning.
    let form = derived(RANGES);
    let bands = bands(&form, "at0012").expect("the fixture states two other ranges");
    assert_eq!(bands.normal_range, None);
    assert_eq!(bands.normal_status, None);
    let meanings: Vec<Vec<String>> = bands
        .other
        .iter()
        .map(|band| meaning(band.meaning.as_ref()))
        .collect();
    assert_eq!(
        meanings,
        [vec!["therapeutic".to_owned()], vec!["critical".to_owned()]]
    );
    let therapeutic = bands
        .other
        .first()
        .and_then(|band| band.range.as_ref())
        .expect("the first band states its range");
    assert_eq!(
        magnitude(&therapeutic.lower),
        ("mmol/l".to_owned(), Some(point(3.5)))
    );
    assert_eq!(
        magnitude(&therapeutic.upper),
        ("mmol/l".to_owned(), Some(point(5.0)))
    );
}

#[test]
fn one_value_carries_all_three_bands_at_once() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.1 states the three
    // attributes independently, so a value may carry any combination.
    let form = derived(RANGES);
    let bands = bands(&form, "at0013").expect("the fixture states all three");
    assert!(bands.normal_status.is_some());
    let range = bands
        .normal_range
        .expect("the fixture states a normal range");
    assert_eq!(range.element_rm_type.as_str(), "DV_COUNT");
    assert_eq!(
        counts(&range.lower),
        [Range::new(Some(150), Some(150), true, true)]
    );
    assert_eq!(
        counts(&range.upper),
        [Range::new(Some(400), Some(400), true, true)]
    );
    assert_eq!(bands.other.len(), 1);
    let critical = bands
        .other
        .first()
        .expect("the fixture states one other range");
    assert_eq!(meaning(critical.meaning.as_ref()), ["critical"]);
    assert_eq!(
        critical
            .range
            .as_ref()
            .map(|band| counts(&band.upper))
            .unwrap_or_default(),
        [Range::new(Some(20), Some(20), true, true)]
    );
}

#[test]
fn a_value_the_template_states_no_band_for_carries_none() {
    let form = derived(RANGES);
    assert_eq!(bands(&form, "at0014"), None);
    // The member is absent from the document rather than present and empty,
    // so a document written before the bands existed still parses as one.
    let carried = form
        .fields()
        .filter(|field| field.reference_ranges.is_some())
        .count();
    let document = serde_json::to_string(&form).expect("a form definition serializes");
    assert_eq!(document.matches("\"reference_ranges\"").count(), carried);
    assert_eq!(carried, 4);
}

#[test]
fn a_band_is_never_an_item_a_clinician_fills() {
    // A renderer tells display metadata from an entry field by where it
    // sits: the bands hang off the field, and no `normal_range`,
    // `normal_status` or `other_reference_ranges` node becomes a field, a
    // group or undetermined content of its own.
    let form = derived(RANGES);
    assert_eq!(form.fields().count(), 5);
    assert_eq!(form.undetermined().count(), 0);
    for path in form
        .fields()
        .map(|field| field.key.to_string())
        .chain(form.groups().map(|group| group.key.to_string()))
    {
        for attribute in ["normal_range", "normal_status", "other_reference_ranges"] {
            assert!(
                !path.contains(attribute),
                "{path} is keyed under {attribute}"
            );
        }
    }
}

#[test]
fn a_normal_status_stated_as_something_other_than_a_code_is_refused() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.1 types
    // `normal_status` as a `CODE_PHRASE`; showing free text as the status
    // would report a band the template never stated.
    match refusal(SHAPE) {
        DeriveError::MetadataShape {
            attribute,
            found,
            expected,
            ..
        } => {
            assert_eq!(attribute, "normal_status");
            assert_eq!(found, "text");
            assert_eq!(expected, "a CODE_PHRASE");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_band_whose_interval_names_no_element_type_is_refused() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.2 types both ends
    // of a `DV_INTERVAL` as the same `DV_ORDERED` descendant, and a template
    // naming neither the type parameter nor either end has not said which.
    match refusal(UNTYPED) {
        DeriveError::UntypedMetadataRange { attribute, .. } => {
            assert_eq!(attribute, "normal_range");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_reference_range_attribute_outside_meaning_and_range_is_refused() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.3 gives
    // `REFERENCE_RANGE` a `meaning` and a `range` and nothing else.
    match refusal(EXTRA) {
        DeriveError::UnmodelledAttribute {
            rm_type, attribute, ..
        } => {
            assert_eq!(rm_type, "REFERENCE_RANGE");
            assert_eq!(attribute, "value");
        }
        other => panic!("the refusal is {other:?}"),
    }
}
