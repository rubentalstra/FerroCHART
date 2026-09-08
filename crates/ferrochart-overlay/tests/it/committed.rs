// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The overlay the browser battery drives, checked against the template it
//! was authored over.
//!
//! `e2e/overlays/family-history-summary-item-r2.json` is the one authored
//! layout this repository ships, and `scripts/ui-e2e.sh` serves it so a
//! journey can drive a form that grows as it is answered. Nothing keeps it in
//! step with the vendored template pack except this: a re-vendored template
//! that renames or drops a node fails here rather than quietly serving a
//! layout that decides nothing.
//!
//! Synthetic content invented for this repository. No patient data.

use std::path::{Path, PathBuf};

use ferrochart_form::layout::Visibility;
use ferrochart_overlay::replay::{Outcome, ReferenceOutcome, replay};
use ferrochart_overlay::store::Overlay;

use crate::support;

/// The template the committed overlay was authored over.
const TEMPLATE: &str = "family-history-summary-item-r2.opt";

/// The layout the browser battery serves.
fn committed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../e2e/overlays/family-history-summary-item-r2.json")
}

/// The committed overlay, read.
fn overlay() -> Overlay {
    let path = committed();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} does not read: {error}", path.display()));
    Overlay::from_json(&text).expect("the committed overlay reads")
}

#[test]
fn the_committed_overlay_still_resolves_against_its_template() {
    let overlay = overlay();
    let definition = support::form(TEMPLATE);
    assert_eq!(overlay.template().id, definition.template_id);

    let report = replay(&overlay, &definition);
    for entry in report.entries() {
        assert!(
            matches!(entry.outcome, Outcome::Matched { .. }),
            "the committed overlay's entry at {} is {:?}",
            entry.key,
            entry.outcome
        );
        for reference in &entry.references {
            assert!(
                matches!(reference.outcome, ReferenceOutcome::Resolved { .. }),
                "the committed overlay reads a node the template no longer has: {reference:?}"
            );
        }
    }
}

#[test]
fn the_committed_overlay_hides_the_death_questions_until_they_are_answered() {
    // The reason the fixture exists: a form that asks every question at once
    // is the design issue #201 was filed against, and this layout is what
    // answers it.
    let overlay = overlay();
    let gated = overlay
        .entries()
        .iter()
        .filter(|entry| entry.layout.visibility != Visibility::Always)
        .count();
    assert!(gated >= 2, "only {gated} entries are conditional");
}
