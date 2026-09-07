// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The replay against the largest real difference the corpus can produce.
//!
//! Every other replay test applies surgery to a derived form, because no
//! genuine revision pair can be obtained. The openEHR CKM reports a later
//! asset version for 176 of its 306 templates and serves none of them: its
//! export endpoint ignores a version parameter rather than honouring it, so an
//! earlier revision cannot be fetched. No archetype in the committed pack
//! appears at two major versions either. Both measured on issue #21.
//!
//! What the pack does hold is two separate CKM entries for one clinical
//! concept over the same root archetype. **They are not a revision of one
//! template**, and the numbers below show why the distinction matters: the two
//! differ so thoroughly that nothing at all survives between them.
//!
//! So this module does not prove that authored layout survives a revision.
//! The constructed revisions prove that, node by node. What this proves is the
//! property that holds when a difference is total: every entry is still
//! accounted for, nothing is silently rebound, and nothing is discarded. That
//! is the promise the design rests on, tested at a scale and with an
//! accidental shape no fixture reaches.

use std::collections::BTreeSet;

use ferrochart_form::layout::Layout;
use ferrochart_overlay::replay::{Outcome, replay};
use ferrochart_overlay::store::{Author, Overlay};

use crate::support::{form, keys};

/// One CKM entry for the concept.
const ONE: &str = "openehr-confirmed-covid-19-infection-report-v0.opt";

/// The other, which carries more of it.
const OTHER: &str = "openehr-confirmed-covid-19-infection-report-v0-2.opt";

/// A layout on every node of `template`, as a person who decorated the whole
/// form would leave it.
fn overlay_on_every_node(template: &str) -> Overlay {
    let definition = form(template);
    let mut author = Author::new(&definition);
    for key in keys(&definition) {
        // The placement is the store's answer about positionality; this test
        // is about the replay, so it is deliberately dropped.
        let _placed = author
            .set(&key, Layout::new())
            .expect("every key of a derived form places");
    }
    author.finish()
}

#[test]
fn the_two_entries_are_one_concept_over_one_root_archetype() {
    let one = form(ONE);
    let other = form(OTHER);
    // Different template identifiers, same root archetype. That is what makes
    // the difference between them revision-shaped, even though it is not a
    // revision.
    assert_ne!(one.template_id, other.template_id);
    assert_eq!(one.root.archetype_id, other.root.archetype_id);
}

#[test]
fn nothing_is_lost_when_the_difference_is_total() {
    let overlay = overlay_on_every_node(ONE);
    let authored = overlay.entries().len();
    let report = replay(&overlay, &form(OTHER));

    // Every authored entry is accounted for exactly once. This is the
    // assertion that matters: a replay that dropped an entry would still look
    // healthy in a summary.
    assert_eq!(report.entries().len(), authored);

    let summary = report.summary();
    assert_eq!(
        summary.matched
            + summary.disappeared
            + summary.moved
            + summary.ambiguous
            + summary.reordered
            + summary.retyped,
        authored
    );

    // A ratchet over real material. Nothing matches, because the two entries
    // differ in the pinned name of nearly every node and a name is part of the
    // key by the measurement on #8. The replay says so rather than rebinding a
    // layout onto a node that merely looks similar.
    assert_eq!(authored, 171);
    assert_eq!(summary.matched, 0);
    assert_eq!(summary.moved, 7);
    assert_eq!(summary.disappeared, 164);
    assert_eq!(summary.ambiguous, 0);
    assert_eq!(summary.reordered, 0);
    assert_eq!(summary.retyped, 0);
    assert_eq!(summary.appeared, 181);
}

#[test]
fn a_move_is_a_suggestion_rather_than_a_rebinding() {
    let overlay = overlay_on_every_node(ONE);
    let report = replay(&overlay, &form(OTHER));

    // Seven nodes are recognizable in the other entry under a different key.
    // Each is offered, and the overlay is unchanged until a person accepts it.
    let mut suggested = 0_usize;
    for entry in report.entries() {
        if let Outcome::Moved { ref to } = entry.outcome {
            assert_ne!(
                entry.key.steps, to.steps,
                "a move suggesting the key it came from"
            );
            suggested += 1;
        }
    }
    assert_eq!(suggested, 7);

    let held: BTreeSet<Vec<_>> = overlay
        .entries()
        .iter()
        .map(|entry| entry.key.steps.clone())
        .collect();
    for entry in report.entries() {
        assert!(
            held.contains(&entry.key.steps),
            "the overlay stopped holding {} after a replay",
            entry.key
        );
    }
}

#[test]
fn every_disappeared_entry_keeps_its_key() {
    let overlay = overlay_on_every_node(ONE);
    let report = replay(&overlay, &form(OTHER));

    // The retention rule at the scale where it counts: 164 entries whose nodes
    // are gone all keep their keys, so a later revision that restores a node
    // restores its layout.
    let held: BTreeSet<Vec<_>> = overlay
        .entries()
        .iter()
        .map(|entry| entry.key.steps.clone())
        .collect();
    let mut kept = 0_usize;
    for entry in report.entries() {
        if matches!(entry.outcome, Outcome::Disappeared) {
            assert!(held.contains(&entry.key.steps));
            kept += 1;
        }
    }
    assert_eq!(kept, 164);
}

#[test]
fn replaying_against_the_form_it_was_authored_on_changes_nothing() {
    // The control. Without it the numbers above could come from a replay that
    // matches nothing ever.
    let overlay = overlay_on_every_node(ONE);
    let report = replay(&overlay, &form(ONE));
    for entry in report.entries() {
        assert!(
            matches!(entry.outcome, Outcome::Matched { .. }),
            "{} is {} against its own form",
            entry.key,
            entry.outcome.name()
        );
    }
    assert!(report.appeared().is_empty());
    assert_eq!(report.summary().matched, 171);
}

#[test]
fn two_keys_that_print_the_same_do_not_always_match() {
    // Not a property worth having: it pins the defect on #83 so the fix has a
    // test that fails first. 40 distinct key strings occur in both forms and
    // none of them matches, because a printed key omits the pinned name and
    // the RM type, which section 6.2 measured as the discriminators for 41.0%
    // and 29.5% of colliding siblings.
    let one: BTreeSet<String> = keys(&form(ONE)).iter().map(ToString::to_string).collect();
    let other: BTreeSet<String> = keys(&form(OTHER)).iter().map(ToString::to_string).collect();
    assert_eq!(one.intersection(&other).count(), 40);
    assert_eq!(overlay_on_every_node(ONE).entries().len(), 171);
}
