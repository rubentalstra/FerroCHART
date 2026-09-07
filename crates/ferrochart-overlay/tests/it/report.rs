// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a form author reads after a recompile.
//!
//! The report is the product: no implementation surveyed for this project says
//! what a recompile did to hand-authored layout. These snapshots are the text
//! a person sees, so a change to it is read before it is accepted
//! (`.claude/rules/testing.md`).

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::layout::Layout;
use ferrochart_form::text::Localized;
use ferrochart_overlay::replay::replay;
use ferrochart_overlay::store::Author;

use crate::support::{
    drop_last_child, duplicate, form, key_with_node_id, keys, parent_key, put, retype,
    sibling_positions, swap, take, tied_key,
};

fn authored(text: &str) -> Layout {
    Layout {
        order: Some(1),
        label: Localized::in_language(LanguageTag::new("en"), text),
        ..Layout::new()
    }
}

#[test]
fn the_report_of_a_revision_that_did_everything_at_once() {
    let definition = form("aedes-indices-jm.opt");
    let untouched = key_with_node_id(&definition, "at0004");
    let removed = key_with_node_id(&definition, "at0007");
    let relocated = key_with_node_id(&definition, "at0008");
    let reclassed = key_with_node_id(&definition, "at0006");
    let doubled = key_with_node_id(&definition, "at0005");

    // A person laid out every node but one, which is the one the report has
    // to name as carrying no layout yet.
    let mut author = Author::new(&definition);
    for key in keys(&definition) {
        if key.steps == untouched.steps {
            continue;
        }
        // A person who set an order and a control but left the archetype's
        // own rubric as the label, which is the ordinary case.
        let _ = author
            .set(
                &key,
                Layout {
                    order: Some(1),
                    ..Layout::new()
                },
            )
            .unwrap();
    }
    let overlay = author.finish();

    let mut revised = definition.clone();
    take(&mut revised, &removed);
    let item = take(&mut revised, &relocated);
    let root = revised.root.key.clone();
    put(&mut revised, &root, item);
    retype(&mut revised, &reclassed, "DV_CODED_TEXT");
    duplicate(&mut revised, &doubled);

    insta::assert_snapshot!(replay(&overlay, &revised).to_string());
}

#[test]
fn the_report_of_a_reordering_among_nodes_the_template_tells_apart_by_nothing() {
    let pack = form("clinical-context-jm.opt");
    let key = tied_key(&pack).expect("the pack carries a same-id sibling group");
    let parent = parent_key(&key);

    let mut definition = pack.clone();
    drop_last_child(&mut definition, &key);
    let mut author = Author::new(&definition);
    let _ = author.set(&key, authored("Weight, as recorded")).unwrap();
    let overlay = author.finish();

    let mut revised = definition.clone();
    let positions = sibling_positions(&revised, &parent, key.terminal().unwrap());
    swap(&mut revised, &parent, positions[0], positions[1]);

    insta::assert_snapshot!(replay(&overlay, &revised).to_string());
}
