// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The whole committed CKM pack, keyed and replayed.
//!
//! The key of `docs/architecture.md` section 6.2 was measured rather than
//! assumed, and this is the measurement as a test: author a layout on every
//! group and field of every template the reader reads, replay it against the
//! form it was authored against, and assert that every entry comes back.
//! Anything a key cannot tell apart shows here as an entry needing a
//! decision rather than as layout bound to the wrong node.

use ferrochart_overlay::layout::Layout;
use ferrochart_overlay::replay::replay;
use ferrochart_overlay::store::{Author, Placement};

use crate::support::{derived, keys, templates};

#[test]
fn the_pack_is_present() {
    assert!(
        templates().len() >= 100,
        "expected the vendored CKM pack, found {} templates. \
         Run scripts/vendor/ckm-templates.sh",
        templates().len()
    );
}

#[test]
fn every_entry_authored_over_the_pack_replays_against_the_form_it_was_keyed_to() {
    let mut nodes = 0_usize;
    let mut forms = 0_usize;
    for path in templates() {
        let Some(definition) = derived(&path) else {
            continue;
        };
        forms += 1;
        let all = keys(&definition);
        let mut author = Author::new(&definition);
        for key in &all {
            let _ = author
                .set(key, Layout::new())
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
        let overlay = author.finish();
        let report = replay(&overlay, &definition);
        let summary = report.summary();
        assert_eq!(
            summary.matched,
            all.len(),
            "{}: {summary:?}",
            path.display()
        );
        assert_eq!(summary.needing_decision(), 0, "{}", path.display());
        assert_eq!(summary.disappeared, 0, "{}", path.display());
        assert_eq!(summary.appeared, 0, "{}", path.display());
        nodes += all.len();
    }
    // A ratchet on the committed pack: the pack is pinned per template by its
    // `cid` in corpus/templates/ckm/PROVENANCE.md, so keying fewer nodes than
    // this is a regression rather than a new baseline.
    assert!(forms >= 121, "only {forms} templates derived");
    assert!(nodes >= 4447, "only {nodes} nodes keyed");
}

#[test]
fn the_pack_carries_same_id_sibling_groups_and_every_one_of_them_is_reported() {
    let mut tied_nodes = 0_usize;
    let mut tied_forms = 0_usize;
    for path in templates() {
        let Some(definition) = derived(&path) else {
            continue;
        };
        let mut author = Author::new(&definition);
        let mut here = 0_usize;
        for key in keys(&definition) {
            let derived_flag = key.is_positional;
            let placement = author.set(&key, Layout::new()).unwrap();
            if let Placement::Positional { tied } = placement {
                assert!(tied > 1, "a positional placement names several nodes");
                here += 1;
            }
            // The compiler answers the same question when it builds the key
            // (#67), and the two must agree or one of them is wrong about the
            // definition in front of it.
            assert_eq!(
                derived_flag,
                matches!(placement, Placement::Positional { .. }),
                "{}: the derived key and the overlay disagree about positionality at {key}",
                path.display()
            );
        }
        let overlay = author.finish();
        for entry in overlay.entries() {
            assert_eq!(
                entry.key.is_positional,
                entry.anchor.is_some(),
                "{}: an entry carries an anchor exactly when it is positional",
                path.display()
            );
        }
        tied_nodes += here;
        if here > 0 {
            tied_forms += 1;
        }
    }
    // The collision the measurement on issue #8 found is in this pack too, so
    // the ordinal and the anchor are load-bearing rather than theoretical.
    assert!(
        tied_forms >= 4 && tied_nodes >= 102,
        "the pack carried {tied_nodes} colliding nodes in {tied_forms} templates"
    );
}
