// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The gate over the whole committed template pack.
//!
//! Two things are measured here, and both are ratchets rather than round
//! numbers: how much of the pack passes the gate, and how much of what fails
//! reaches a field a renderer can show it on.

use std::collections::BTreeMap;

use ferrochart_form::validation::{FailureKind, FailureSource};
use serde_json::Value;

use crate::support;

#[test]
fn every_composition_the_builder_produces_is_judged_against_its_own_template() {
    // The product's promise, measured rather than asserted: a composition
    // FerroCHART built from a form FerroCHART derived from a template should
    // conform to that template, because a form is a projection of the
    // template and never admits what the template refuses (`CLAUDE.md`).
    //
    // A ratchet, so every number below is a finding rather than a target.
    let mut passed = 0_usize;
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut refused: Vec<String> = Vec::new();
    for (path, case) in support::cases() {
        let report = case
            .validator
            .validate(&case.definition, &case.composition)
            .expect("the composition is judged");
        if report.is_valid() {
            passed = passed.saturating_add(1);
            continue;
        }
        refused.push(format!("{}", path.display()));
        for failure in &report.failures {
            *kinds.entry(name(failure.kind)).or_default() += 1;
        }
    }

    assert_eq!(passed, 71, "the templates that build and pass the gate");
    assert_eq!(
        refused.len(),
        7,
        "the templates the gate refuses: {refused:?}"
    );

    // A template can declare two sibling constraints carrying the same node
    // id and the same pinned name, which no instance can tell apart. The
    // compiler folds such a group into one node before it derives a field, so
    // the document it builds writes one node where it used to write two, and
    // the bound it is judged against is the collective one.
    assert_eq!(
        kinds.get("occurrences").copied(),
        None,
        "sibling constraints an instance cannot separate"
    );

    // The filler enters nothing where the template's own value set is not
    // enumerated, because resolving one needs the terminology client rather
    // than the template. A mandatory field left empty is what the gate is
    // for, so these are the gate working on a test that under-fills, not a
    // compiler defect.
    assert_eq!(
        kinds.get("required").copied(),
        Some(14),
        "mandatory fields the filler cannot choose a value for"
    );
    assert_eq!(
        kinds.get("cardinality").copied(),
        Some(1),
        "a container left short by the same under-filling"
    );

    assert_eq!(
        kinds.keys().copied().collect::<Vec<_>>(),
        vec!["cardinality", "required"],
        "a refusal of a kind this ratchet does not account for"
    );
}

/// A stable name for a failure kind, so the ratchet reads as prose.
fn name(kind: FailureKind) -> &'static str {
    match kind {
        FailureKind::WrongType => "wrong_type",
        FailureKind::Required => "required",
        FailureKind::Occurrences => "occurrences",
        FailureKind::Cardinality => "cardinality",
        FailureKind::RangeError => "range",
        FailureKind::PatternError => "pattern",
        FailureKind::CodedValue => "coded_value",
        FailureKind::Terminology => "terminology",
        FailureKind::Invariant => "invariant",
        FailureKind::Unexpected => "unexpected",
        // NOTE: the arm is a wildcard on purpose: `FailureKind` is
        // `#[non_exhaustive]`, so a variant added later reaches the ratchet's
        // own catch-all assertion rather than failing the build here.
        _ => "other",
    }
}

#[test]
fn a_failure_reaches_the_field_it_belongs_to() {
    // The claim of `docs/architecture.md` section 9, as it actually holds.
    // Taking the value out of every ELEMENT is the sample that matters: it is
    // the failure a clinician causes, one per field, so what the resolution
    // has to do is put each one back on the field it came from.
    let mut placed = 0_usize;
    let mut total = 0_usize;
    for (path, case) in support::cases() {
        let mut broken = serde_json::to_value(&case.composition).expect("a composition serialises");
        strip_element_values(&mut broken);
        let report = case
            .validator
            .validate_json(&case.definition, &broken)
            .expect("the broken document is judged");
        for failure in &report.failures {
            total = total.saturating_add(1);
            assert_eq!(failure.source, FailureSource::Template);
            if failure.key.is_some() {
                placed = placed.saturating_add(1);
            } else {
                assert!(
                    !failure.path.is_empty(),
                    "{}: a failure with neither a key nor a path says nothing",
                    path.display()
                );
            }
        }
    }
    assert!(total > 0, "an element with no value fails somewhere");
    // A ratchet, not a round number. What does not resolve is a Reference
    // Model invariant reported on the envelope the template never describes,
    // and it is reported unplaced rather than dropped.
    assert!(
        placed.saturating_mul(10) >= total.saturating_mul(9),
        "fewer than nine failures in ten reach a field: {placed} of {total}"
    );
}

/// Takes the value out of every `ELEMENT` of a canonical-JSON document.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3:
/// `Is_null: is_null xor value /= Void`, so an element with neither a value
/// nor a null flavour is refused, and the refusal names the element.
fn strip_element_values(node: &mut Value) {
    match *node {
        Value::Object(ref mut members) => {
            if members.get("_type").and_then(Value::as_str) == Some("ELEMENT") {
                members.remove("value");
                return;
            }
            for member in members.values_mut() {
                strip_element_values(member);
            }
        }
        Value::Array(ref mut items) => {
            for item in items.iter_mut() {
                strip_element_values(item);
            }
        }
        _ => {}
    }
}
