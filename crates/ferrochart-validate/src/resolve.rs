// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A validation message becomes a failure a renderer can place.
//!
//! This is the adapter `docs/architecture.md` section 9 said was unnecessary.
//! It is necessary: the crate documentation records the three path languages
//! and the measurement that settled it.

use ferrochart_form::validation::{FailureKind, FailureSource, ValidationFailure};
use openehr_its::rm_instance::{ValidationKind, ValidationMessage};
use serde_json::Value;

use crate::index::KeyIndex;
use crate::path;

/// Turns one validation message into a failure, keyed where the path resolves.
///
/// `instance` is the whole composition, which is what a positional path is
/// read against. `prefix` is prepended to the message's own path, for a
/// message the archetype-conformance pass reported against a sub-tree.
#[must_use]
pub fn failure(
    message: &ValidationMessage,
    index: &KeyIndex,
    instance: &Value,
    prefix: &str,
) -> ValidationFailure {
    let reported = format!("{prefix}{}", trim_root(&message.path));
    let normalized = path::against_instance(&reported, instance);
    ValidationFailure {
        key: index.resolve(&normalized).cloned(),
        path: reported,
        message: message.message.clone(),
        kind: kind(message.kind),
        source: FailureSource::Template,
    }
}

/// The root path, which `openehr_its::rm_instance` normalizes to `/`.
///
/// Prefixing it would produce a path with a trailing separator that names no
/// step, so the root contributes nothing and the prefix stands alone.
fn trim_root(path: &str) -> &str {
    if path == "/" { "" } else { path }
}

/// The report vocabulary, restated as the published contract's own type.
#[expect(
    clippy::match_wildcard_for_single_variants,
    reason = "ValidationKind is #[non_exhaustive], so the wildcard covers the variants upstream has yet to add as well as Other"
)]
fn kind(kind: ValidationKind) -> FailureKind {
    match kind {
        ValidationKind::WrongType => FailureKind::WrongType,
        ValidationKind::Required => FailureKind::Required,
        ValidationKind::Occurrences => FailureKind::Occurrences,
        ValidationKind::Cardinality => FailureKind::Cardinality,
        ValidationKind::RangeError => FailureKind::RangeError,
        ValidationKind::PatternError => FailureKind::PatternError,
        ValidationKind::CodedValue => FailureKind::CodedValue,
        ValidationKind::Terminology => FailureKind::Terminology,
        ValidationKind::Invariant => FailureKind::Invariant,
        ValidationKind::Unexpected => FailureKind::Unexpected,
        // NOTE: the arm is a wildcard on purpose: `ValidationKind` is
        // `#[non_exhaustive]`, so a variant added upstream must become the
        // category that claims nothing rather than fail the build.
        _ => FailureKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::validation::{FailureKind, FailureSource};
    use openehr_its::rm_instance::{ValidationKind, ValidationMessage};
    use serde_json::json;

    use super::failure;
    use crate::index::KeyIndex;

    fn message(path: &str, kind: ValidationKind) -> ValidationMessage {
        ValidationMessage {
            path: path.to_owned(),
            message: "a synthetic failure".to_owned(),
            kind,
        }
    }

    #[test]
    fn a_message_that_resolves_to_nothing_keeps_its_reported_path() {
        let built = failure(
            &message("/content[0]", ValidationKind::Invariant),
            &KeyIndex::default(),
            &json!({}),
            "",
        );
        assert!(built.key.is_none());
        assert_eq!(built.path, "/content[0]");
        assert_eq!(built.kind, FailureKind::Invariant);
        assert_eq!(built.source, FailureSource::Template);
    }

    #[test]
    fn a_prefixed_message_carries_the_prefix_in_the_path_it_reports() {
        let built = failure(
            &message("/data[at0001]", ValidationKind::Required),
            &KeyIndex::default(),
            &json!({}),
            "/content[openEHR-EHR-OBSERVATION.x.v1]",
        );
        assert_eq!(
            built.path,
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]"
        );
    }

    #[test]
    fn the_root_path_contributes_no_step_to_a_prefixed_message() {
        let built = failure(
            &message("/", ValidationKind::WrongType),
            &KeyIndex::default(),
            &json!({}),
            "/content[openEHR-EHR-OBSERVATION.x.v1]",
        );
        assert_eq!(built.path, "/content[openEHR-EHR-OBSERVATION.x.v1]");
    }
}
