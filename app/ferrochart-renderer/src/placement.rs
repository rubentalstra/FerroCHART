// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Where a validation failure is drawn.
//!
//! `ferrochart-validate` keys every failure onto the form definition, so a
//! renderer places one with no adapter in between: the failure carries the
//! [`NodeKey`] of the item it belongs to, and a control already holds that
//! key. This module is the lookup a control calls, and the rule it applies.
//!
//! No specification governs the rule: our own design.
//!
//! # A failure names a field, not one repeat of it
//!
//! A [`Slot`] addresses one control: the key, one index per repeating
//! ancestor group, and which repeat of the field itself. A
//! [`ValidationFailure`] carries only the key, because
//! `ferrochart-validate` resolves a reported path onto the DEFINITION and the
//! definition has one item per field however many times the data repeats it.
//! So a failure lands on every control its field renders, and the only
//! narrowing is a count: a failure about HOW MANY times a node appears is
//! about the set rather than about a member of it, so it is drawn once, on
//! the first control, instead of under every sibling.
//!
//! # Nothing is dropped
//!
//! A failure whose path resolved to no item of the form has no key, and
//! [`unplaced`] is how a screen reaches it. Losing a refusal silently is the
//! defect class this project exists to prevent, so a screen that renders a
//! report renders that list too.

// TODO(#152): draw a refusal on the repeat it belongs to. The occurrence
// address exists at the seam and is dropped before the report is built.

// The lookup is what a control calls, and the controls are issue #137. Its
// own tests exercise every function, so the lint fires in one configuration
// and not the other, which is what `allow` is for here.
#![allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]

use ferrochart_form::key::NodeKey;
use ferrochart_form::validation::{FailureKind, ValidationFailure, ValidationReport};
use ferrochart_form::values::Slot;

/// Every failure the control at `slot` draws.
pub(crate) fn at<'r>(
    report: &'r ValidationReport,
    slot: &'r Slot,
) -> impl Iterator<Item = &'r ValidationFailure> {
    at_key(report, &slot.key, slot.occurrence)
}

/// Every failure the control for `key` at repeat `occurrence` draws.
pub(crate) fn at_key<'r>(
    report: &'r ValidationReport,
    key: &'r NodeKey,
    occurrence: usize,
) -> impl Iterator<Item = &'r ValidationFailure> {
    report
        .at(key)
        .filter(move |failure| occurrence == 0 || !counts(failure.kind))
}

/// Every failure that names no item of the form.
pub(crate) fn unplaced(report: &ValidationReport) -> impl Iterator<Item = &ValidationFailure> {
    report.unplaced()
}

/// Whether a failure is about how many nodes there are.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.4 states occurrences on
/// the node and section 4.3.5 states cardinality on the container, and both
/// judge the SET of siblings, so neither belongs to one member of it.
fn counts(kind: FailureKind) -> bool {
    matches!(kind, FailureKind::Occurrences | FailureKind::Cardinality)
}

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{ArchetypeId, LocalCode, RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};
    use ferrochart_form::validation::{
        FailureKind, FailureSource, ValidationFailure, ValidationReport,
    };
    use ferrochart_form::values::Slot;

    use super::{at, unplaced};

    /// One step of a key, spelled out so two tests cannot disagree about what
    /// separates a field from its sibling.
    fn step(attribute: &str, node_id: &str, rm_type: &str) -> KeyStep {
        KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: Some(LocalCode::new(node_id)),
            archetype_id: None,
            rm_type: RmTypeName::new(rm_type),
            pinned_name: None,
            sibling_ordinal: 0,
        }
    }

    /// A field three groups down, which is the shape a real template gives
    /// every leaf: an archetype root, its data structure, and a cluster.
    fn nested_field() -> NodeKey {
        NodeKey::root()
            .child(KeyStep {
                rm_attribute: RmAttributeName::new("content"),
                node_id: None,
                archetype_id: Some(ArchetypeId::new(
                    "openEHR-EHR-OBSERVATION.blood_pressure.v2",
                )),
                rm_type: RmTypeName::new("OBSERVATION"),
                pinned_name: None,
                sibling_ordinal: 0,
            })
            .child(step("data", "at0001", "HISTORY"))
            .child(step("items", "at0004", "CLUSTER"))
            .child(step("items", "at0006", "ELEMENT"))
    }

    /// Its sibling under the same cluster.
    fn sibling_field() -> NodeKey {
        let mut key = nested_field();
        key.steps.pop();
        key.child(step("items", "at0007", "ELEMENT"))
    }

    fn failure(key: Option<NodeKey>, kind: FailureKind) -> ValidationFailure {
        ValidationFailure {
            path: key.as_ref().map_or_else(String::new, NodeKey::to_string),
            key,
            message: "a synthetic failure".to_owned(),
            kind,
            source: FailureSource::Template,
        }
    }

    fn slot(key: NodeKey, group_path: Vec<usize>, occurrence: usize) -> Slot {
        Slot {
            key,
            group_path,
            occurrence,
        }
    }

    #[test]
    fn a_failure_lands_on_the_nested_field_its_key_names() {
        let report = ValidationReport {
            failures: vec![
                failure(Some(nested_field()), FailureKind::Required),
                failure(Some(sibling_field()), FailureKind::RangeError),
            ],
        };
        let control = slot(nested_field(), vec![0], 0);
        let placed: Vec<_> = at(&report, &control).collect();
        assert_eq!(placed.len(), 1);
        assert_eq!(
            placed.first().map(|found| found.kind),
            Some(FailureKind::Required)
        );
    }

    #[test]
    fn a_sibling_under_the_same_cluster_draws_nothing_of_its_neighbours() {
        let report = ValidationReport {
            failures: vec![failure(Some(nested_field()), FailureKind::Required)],
        };
        assert_eq!(at(&report, &slot(sibling_field(), vec![0], 0)).count(), 0);
    }

    #[test]
    fn a_repeated_field_draws_the_failure_on_every_repeat_it_renders() {
        // The report keys onto the definition, which holds one item however
        // many times the data repeats it, so every control of the field shows
        // the refusal rather than one of them swallowing it.
        let report = ValidationReport {
            failures: vec![failure(Some(nested_field()), FailureKind::CodedValue)],
        };
        for occurrence in 0..3 {
            assert_eq!(
                at(&report, &slot(nested_field(), vec![1], occurrence)).count(),
                1,
                "repeat {occurrence} lost the failure"
            );
        }
    }

    #[test]
    fn a_count_is_drawn_once_rather_than_under_every_repeat() {
        let report = ValidationReport {
            failures: vec![
                failure(Some(nested_field()), FailureKind::Occurrences),
                failure(Some(nested_field()), FailureKind::Cardinality),
            ],
        };
        assert_eq!(at(&report, &slot(nested_field(), vec![], 0)).count(), 2);
        assert_eq!(at(&report, &slot(nested_field(), vec![], 1)).count(), 0);
    }

    #[test]
    fn a_count_and_a_value_failure_on_one_field_do_not_hide_each_other() {
        let report = ValidationReport {
            failures: vec![
                failure(Some(nested_field()), FailureKind::Occurrences),
                failure(Some(nested_field()), FailureKind::Required),
            ],
        };
        assert_eq!(at(&report, &slot(nested_field(), vec![], 0)).count(), 2);
        let third = slot(nested_field(), vec![], 2);
        let later: Vec<_> = at(&report, &third).collect();
        assert_eq!(later.len(), 1);
        assert_eq!(
            later.first().map(|found| found.kind),
            Some(FailureKind::Required)
        );
    }

    #[test]
    fn a_failure_that_names_no_field_is_reachable_rather_than_dropped() {
        let report = ValidationReport {
            failures: vec![
                failure(None, FailureKind::Invariant),
                failure(Some(nested_field()), FailureKind::Required),
            ],
        };
        assert_eq!(unplaced(&report).count(), 1);
        assert_eq!(at(&report, &slot(nested_field(), vec![], 0)).count(), 1);
    }

    #[test]
    fn every_failure_of_a_report_is_either_placed_or_unplaced() {
        let keys = [nested_field(), sibling_field()];
        let report = ValidationReport {
            failures: vec![
                failure(None, FailureKind::Other),
                failure(Some(nested_field()), FailureKind::Required),
                failure(Some(sibling_field()), FailureKind::WrongType),
            ],
        };
        let placed: usize = keys
            .iter()
            .map(|key| at(&report, &slot(key.clone(), Vec::new(), 0)).count())
            .sum();
        assert_eq!(
            placed + unplaced(&report).count(),
            report.failures.len(),
            "a refusal reached neither a field nor the unplaced list"
        );
    }
}
