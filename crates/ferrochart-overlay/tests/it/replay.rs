// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The replay, one case per outcome class of `docs/architecture.md`
//! section 6.5.
//!
//! Each case authors an overlay against a form derived from a committed CKM
//! operational template, then replays it against that form with one thing
//! about it changed, which is what a template revision does. The layouts are
//! synthetic content invented for the test.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;
use ferrochart_overlay::layout::{Condition, Layout, Visibility};
use ferrochart_overlay::replay::{Outcome, ReferenceOutcome, replay};
use ferrochart_overlay::store::{Author, Overlay};

use crate::support::{
    drop_last_child, duplicate, form, key_where, keys, parent_key, put, rename, retype,
    sibling_positions, swap, take, tied_key,
};

fn authored(text: &str) -> Layout {
    Layout {
        order: Some(1),
        label: Localized::in_language(LanguageTag::new("en"), text),
        ..Layout::new()
    }
}

/// An overlay decorating every group and field of `definition`.
fn whole(definition: &FormDefinition) -> Overlay {
    let mut author = Author::new(definition);
    for (position, key) in keys(definition).into_iter().enumerate() {
        let _ = author
            .set(&key, authored(&format!("Item {position}")))
            .unwrap();
    }
    author.finish()
}

/// An overlay decorating one node of `definition`.
fn one(definition: &FormDefinition, key: &NodeKey) -> Overlay {
    let mut author = Author::new(definition);
    let _ = author.set(key, authored("Authored")).unwrap();
    author.finish()
}

/// The outcome of the only entry of an overlay.
fn only_outcome(overlay: &Overlay, definition: &FormDefinition) -> Outcome {
    let report = replay(overlay, definition);
    assert_eq!(report.entries().len(), 1);
    report.entries().first().unwrap().outcome.clone()
}

/// A node below the root whose terminal node id appears nowhere else.
fn unique_field(definition: &FormDefinition) -> NodeKey {
    unique_node(definition, 2)
}

/// A node at least `depth` steps deep whose terminal node id appears nowhere
/// else.
fn unique_node(definition: &FormDefinition, depth: usize) -> NodeKey {
    let all = keys(definition);
    all.iter()
        .find(|key| {
            if key.steps.len() < depth {
                return false;
            }
            let Some(step) = key.terminal() else {
                return false;
            };
            let Some(node_id) = step.node_id.as_ref() else {
                return false;
            };
            all.iter()
                .filter(|other| {
                    other
                        .terminal()
                        .and_then(|candidate| candidate.node_id.as_ref())
                        == Some(node_id)
                })
                .count()
                == 1
        })
        .expect("the form holds a node with a node id of its own")
        .clone()
}

#[test]
fn replaying_against_the_same_definition_matches_every_entry() {
    let definition = form("aedes-indices-jm.opt");
    let overlay = whole(&definition);
    let report = replay(&overlay, &definition);
    let summary = report.summary();
    assert_eq!(summary.matched, keys(&definition).len());
    assert_eq!(summary.needing_decision(), 0);
    assert_eq!(summary.disappeared, 0);
    assert_eq!(summary.appeared, 0);
    assert!(report.is_same_template());
}

#[test]
fn a_node_the_revision_removed_is_reported_as_disappeared_and_its_layout_is_kept() {
    let definition = form("aedes-indices-jm.opt");
    let key = unique_field(&definition);
    let overlay = one(&definition, &key);

    let mut revised = definition.clone();
    take(&mut revised, &key);

    assert_eq!(only_outcome(&overlay, &revised), Outcome::Disappeared);
    assert_eq!(
        overlay.entries().len(),
        1,
        "a replay discards nothing, so a later revision can restore the layout"
    );
    let restored = replay(&overlay, &definition);
    assert!(matches!(
        restored.entries().first().unwrap().outcome,
        Outcome::Matched { .. }
    ));
}

#[test]
fn a_node_the_revision_put_somewhere_else_is_a_move_a_person_accepts() {
    let definition = form("aedes-indices-jm.opt");
    let key = unique_node(&definition, 3);
    let overlay = one(&definition, &key);

    let mut revised = definition.clone();
    let item = take(&mut revised, &key);
    let root = revised.root.key.clone();
    put(&mut revised, &root, item);

    let Outcome::Moved { to } = only_outcome(&overlay, &revised) else {
        panic!("the node is elsewhere, so the entry moved");
    };
    assert_ne!(to.steps, key.steps);

    // The replay suggests; only a person re-keys.
    let mut author = Author::resume(&revised, overlay).unwrap();
    let _ = author.accept_move(&key, &to).unwrap();
    let accepted = author.finish();
    assert!(accepted.entry(&key).is_none());
    assert_eq!(
        accepted.entry(&to).unwrap().layout,
        authored("Authored"),
        "accepting a move takes the layout with it"
    );
    assert!(matches!(
        only_outcome(&accepted, &revised),
        Outcome::Matched { .. }
    ));
}

#[test]
fn a_node_whose_pinned_name_changed_while_its_node_id_did_not_is_reported() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.2.3.1 lets a revision
    // restate a node's name without touching its node id, and the pinned name
    // is the discriminator that separates 41.0% of colliding sibling groups,
    // so it is in the key and a change to it has to be reported.
    let definition = form("aedes-indices-jm.opt");
    let key = key_where(&definition, |step| {
        step.pinned_name.is_some() && step.node_id.is_some()
    })
    .expect("the template pins a name on a node with a node id");
    let overlay = one(&definition, &key);

    let mut revised = definition.clone();
    rename(&mut revised, &key, "Container index (revised)");

    let Outcome::Moved { to } = only_outcome(&overlay, &revised) else {
        panic!("the node kept its node id and class, so it is recognized");
    };
    assert_eq!(
        to.terminal().unwrap().pinned_name.as_deref(),
        Some("Container index (revised)")
    );
    assert_eq!(
        to.terminal().unwrap().node_id,
        key.terminal().unwrap().node_id
    );
}

#[test]
fn a_node_that_changed_class_is_reported_as_retyped() {
    let definition = form("aedes-indices-jm.opt");
    let key = unique_field(&definition);
    let was = key.terminal().unwrap().rm_type.clone();
    let overlay = one(&definition, &key);

    let mut revised = definition.clone();
    retype(&mut revised, &key, "DV_CODED_TEXT");

    let Outcome::Retyped {
        at,
        was: reported,
        now,
    } = only_outcome(&overlay, &revised)
    else {
        panic!("the node is in the same place with another class");
    };
    assert_eq!(reported, was);
    assert_eq!(now.as_str(), "DV_CODED_TEXT");
    assert_eq!(at.terminal().unwrap().rm_type.as_str(), "DV_CODED_TEXT");
}

#[test]
fn a_key_that_the_revision_made_ambiguous_is_reported_rather_than_guessed() {
    let definition = form("aedes-indices-jm.opt");
    let key = unique_field(&definition);
    let overlay = one(&definition, &key);

    let mut revised = definition.clone();
    duplicate(&mut revised, &key);

    let Outcome::Ambiguous { candidates } = only_outcome(&overlay, &revised) else {
        panic!("two siblings now carry the key");
    };
    assert_eq!(candidates.len(), 2);
}

#[test]
fn a_positional_entry_whose_container_changed_is_reported_as_reordered() {
    // The pack's own same-id sibling group: two nodes told apart by nothing
    // but position. Change what one of them holds and the position no longer
    // says which node the layout belongs to.
    let definition = form("clinical-context-jm.opt");
    let key = tied_key(&definition).expect("the pack carries a same-id sibling group");
    let overlay = one(&definition, &key);
    assert!(matches!(
        only_outcome(&overlay, &definition),
        Outcome::Matched { .. }
    ));

    let mut revised = definition.clone();
    drop_last_child(&mut revised, &key);

    let Outcome::Reordered { candidates } = only_outcome(&overlay, &revised) else {
        panic!("the siblings the position was measured against changed");
    };
    assert_eq!(candidates.len(), 2);
}

#[test]
fn swapping_two_lookalike_siblings_is_reported_as_reordered() {
    let pack = form("clinical-context-jm.opt");
    let key = tied_key(&pack).expect("the pack carries a same-id sibling group");
    let parent = parent_key(&key);

    // Make the two lookalikes differ in what they hold, so a swap between
    // them is a change a layout can see at all.
    let mut definition = pack.clone();
    drop_last_child(&mut definition, &key);
    let overlay = one(&definition, &key);
    assert!(matches!(
        only_outcome(&overlay, &definition),
        Outcome::Matched { .. }
    ));

    let mut revised = definition.clone();
    let positions = sibling_positions(&revised, &parent, key.terminal().unwrap());
    assert_eq!(positions.len(), 2);
    swap(&mut revised, &parent, positions[0], positions[1]);

    let outcome = only_outcome(&overlay, &revised);
    assert!(
        matches!(outcome, Outcome::Reordered { .. }),
        "swapping the lookalikes at {parent} gave {outcome:?}"
    );
}

#[test]
fn a_node_no_entry_decorates_is_reported_as_new() {
    let definition = form("aedes-indices-jm.opt");
    let key = unique_field(&definition);
    let overlay = one(&definition, &key);
    let report = replay(&overlay, &definition);
    let expected = keys(&definition).len().saturating_sub(1);
    assert_eq!(report.summary().appeared, expected);
    assert!(
        report
            .appeared()
            .iter()
            .all(|node| node.key.steps != key.steps),
        "the decorated node is not new"
    );
}

#[test]
fn a_visibility_rule_reading_a_node_the_revision_removed_is_reported() {
    let definition = form("aedes-indices-jm.opt");
    let subject = unique_field(&definition);
    let decorated = keys(&definition)
        .into_iter()
        .find(|key| key.steps != subject.steps)
        .unwrap();
    let mut layout = authored("Depends");
    layout.visibility = Visibility::When {
        condition: Condition::Answered {
            node: subject.clone(),
        },
    };
    let mut author = Author::new(&definition);
    let _ = author.set(&decorated, layout).unwrap();
    let overlay = author.finish();

    let mut revised = definition.clone();
    take(&mut revised, &subject);

    let report = replay(&overlay, &revised);
    let entry = report.entries().first().unwrap();
    assert_eq!(entry.references.len(), 1);
    assert_eq!(entry.references[0].outcome, ReferenceOutcome::Disappeared);
    assert_eq!(entry.references[0].node.steps, subject.steps);
}

#[test]
fn replaying_the_same_pair_twice_gives_the_same_report() {
    let definition = form("au-covid-19-likelihood-assessment.opt");
    let overlay = whole(&definition);
    let mut revised = definition.clone();
    let key = unique_field(&definition);
    retype(&mut revised, &key, "DV_TEXT");
    let once = replay(&overlay, &revised);
    let twice = replay(&overlay, &revised);
    assert_eq!(once, twice);
    assert_eq!(once.to_string(), twice.to_string());
}

#[test]
fn a_replay_against_another_template_says_so() {
    let definition = form("aedes-indices-jm.opt");
    let overlay = whole(&definition);
    let other = form("birth-detail-jm.opt");
    let report = replay(&overlay, &other);
    assert!(!report.is_same_template());
    assert_eq!(*report.template_was(), definition.template_id);
    assert_eq!(*report.template_now(), other.template_id);
    assert!(report.to_string().contains("a different template"));
}
