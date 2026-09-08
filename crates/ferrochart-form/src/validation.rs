// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a renderer is told when a composition does not conform.
//!
//! The type lives here rather than beside the validator because a renderer
//! links this crate and nothing else of the tree
//! (`docs/architecture.md` section 11), and `docs/architecture.md` section 12
//! puts a validation result keyed by node path on the seam between FerroCHART
//! and a renderer. A report keyed by [`NodeKey`] is one a renderer resolves
//! against the [`crate::definition::FormDefinition`] it is already holding.
//!
//! No specification governs this shape: our own design. openEHR ITS-REST
//! Release-1.1.0 `overview.html` makes a CDR's own error detail optional,
//! conditional on the client having asked for a representation, and shapes it
//! as `DV_CODED_TEXT` entries in vendor-local codes, and `DV_CODED_TEXT` has
//! no path attribute, so the wire carries no pointer to the node that failed
//! and FerroCHART owns the field-level error itself.

use serde::{Deserialize, Serialize};

use crate::key::NodeKey;

/// Everything wrong with one composition.
///
/// An empty report is the only thing that permits a write.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Every failure found, in the order the validator reported them.
    ///
    /// A validator collects rather than stopping at the first failure, so a
    /// clinician is shown every field to correct rather than one at a time.
    pub failures: Vec<ValidationFailure>,
}

impl ValidationReport {
    /// Whether the composition conformed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.failures.is_empty()
    }

    /// Every failure a renderer can put on `key`.
    pub fn at<'r>(&'r self, key: &'r NodeKey) -> impl Iterator<Item = &'r ValidationFailure> {
        self.failures
            .iter()
            .filter(move |failure| failure.key.as_ref() == Some(key))
    }

    /// Every failure that names no field of the form.
    ///
    /// A failure lands here when the path the validator reported does not
    /// resolve to a node the form definition carries: the composition's
    /// envelope, an attribute rather than a node, or a node the derivation
    /// left out. It is shown beside the form rather than dropped.
    pub fn unplaced(&self) -> impl Iterator<Item = &ValidationFailure> {
        self.failures.iter().filter(|failure| failure.key.is_none())
    }
}

/// One thing wrong with a composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationFailure {
    /// The item of the form definition the failure belongs to, where the
    /// reported path resolves to one.
    pub key: Option<NodeKey>,
    /// The path the validator reported, verbatim.
    ///
    /// Kept whether or not [`ValidationFailure::key`] resolved, because it is
    /// what a person triaging a defect matches against the template.
    pub path: String,
    /// What is wrong, in prose.
    pub message: String,
    /// The category of the failure.
    pub kind: FailureKind,
    /// Where the judgement came from.
    pub source: FailureSource,
    /// Which occurrence of the field the failure belongs to, where the
    /// judgement knows.
    ///
    /// The composition builder walks a form with the occurrence of every
    /// repeating group above each field, so it knows which repeat refused a
    /// value. A template refusal carries a Reference Model path whose
    /// positional predicates do not translate to this address, so it states
    /// none rather than inventing one, and a renderer draws such a failure on
    /// every repeat of its field. An address that is sometimes right and
    /// sometimes invented would be worse than none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<FailureAt>,
}

/// Which occurrence of a field a [`ValidationFailure`] belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureAt {
    /// Which occurrence of each repeating ancestor group, outermost first.
    pub group_path: Vec<usize>,
    /// Which repeat of the field itself.
    pub occurrence: usize,
}

/// The category of a [`ValidationFailure`].
///
/// The vocabulary is the one `openehr-its` reports its `ValidationKind` in,
/// restated here as a serialisable type because this crate is the published
/// contract and links nothing else of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FailureKind {
    /// The node's Reference Model class does not conform to the constraint.
    WrongType,
    /// A node the template makes mandatory is absent.
    Required,
    /// A node appears more or fewer times than its occurrences permit.
    Occurrences,
    /// A container holds more or fewer children than its cardinality permits.
    Cardinality,
    /// A number is outside the range the template states.
    RangeError,
    /// A string does not match the pattern the template states.
    PatternError,
    /// A code is not one the template's value set admits.
    CodedValue,
    /// A code is not valid in the openEHR terminology group the Reference
    /// Model binds its attribute to.
    Terminology,
    /// A Reference Model class invariant failed.
    Invariant,
    /// A node no sibling constraint or open slot admits.
    Unexpected,
    /// Anything else the validator judged.
    Other,
}

/// Where a [`ValidationFailure`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FailureSource {
    /// FerroCHART validated the composition against its operational template
    /// before making any request.
    Template,
    /// The composition builder refused the entered values, so no document was
    /// ever judged.
    ///
    /// The builder works from the form definition alone, so a failure from
    /// this source is a value the form itself does not admit: a datum of the
    /// wrong shape, a required field left empty, or more occurrences than the
    /// field permits.
    Builder,
    /// A CDR refused the composition and FerroCHART read its error body.
    ///
    /// The reading is best-effort and vendor-specific: openEHR ITS-REST
    /// Release-1.1.0 publishes no path on an error entry, so a failure from
    /// this source may carry no key at all.
    Cdr,
}

#[cfg(test)]
mod tests {
    use super::{FailureKind, FailureSource, ValidationFailure, ValidationReport};
    use crate::ids::{LocalCode, RmAttributeName, RmTypeName};
    use crate::key::{KeyStep, NodeKey};

    fn key(node_id: &str) -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new("items"),
            node_id: Some(LocalCode::new(node_id)),
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    fn failure(at: Option<NodeKey>) -> ValidationFailure {
        ValidationFailure {
            key: at,
            path: "/items[at0001]/value".to_owned(),
            message: "a synthetic failure".to_owned(),
            kind: FailureKind::Required,
            source: FailureSource::Template,
            at: None,
        }
    }

    #[test]
    fn an_empty_report_is_the_only_valid_one() {
        assert!(ValidationReport::default().is_valid());
        assert!(
            !ValidationReport {
                failures: vec![failure(None)],
            }
            .is_valid()
        );
    }

    #[test]
    fn a_renderer_finds_a_failure_by_the_key_it_already_holds() {
        let wanted = key("at0001");
        let report = ValidationReport {
            failures: vec![failure(Some(wanted.clone())), failure(Some(key("at0002")))],
        };
        assert_eq!(report.at(&wanted).count(), 1);
        assert_eq!(report.unplaced().count(), 0);
    }

    #[test]
    fn a_failure_that_names_no_field_is_shown_rather_than_dropped() {
        let report = ValidationReport {
            failures: vec![failure(None)],
        };
        assert_eq!(report.unplaced().count(), 1);
    }
}
