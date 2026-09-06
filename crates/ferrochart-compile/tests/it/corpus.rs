// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The vendored CKM template pack, read by the ADL 1.4 reader.

use std::fs;

use ferrochart_compile::adl14;
use ferrochart_compile::error::ReadError;
use ferrochart_compile::model::payload::ConstraintPayload;

use crate::support::{corpus_dir, templates};

#[test]
fn the_pack_is_present() {
    let found = templates();
    assert!(
        found.len() >= 100,
        "expected the vendored CKM pack, found {} templates in {}. \
         Run scripts/vendor/ckm-templates.sh",
        found.len(),
        corpus_dir().display()
    );
}

#[test]
fn every_committed_template_states_a_licence() {
    for path in templates() {
        let text = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        assert!(
            text.contains(r#"<other_details id="licence">"#),
            "{} states no licence, so it must not be committed \
             (corpus/templates/ckm/PROVENANCE.md)",
            path.display()
        );
    }
}

#[test]
fn every_template_reads_or_names_what_it_refuses() {
    let mut read = 0_usize;
    let mut refused = Vec::new();
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        match adl14::from_xml(&xml) {
            Ok(_) => read += 1,
            Err(error) => refused.push((path, error)),
        }
    }
    for (path, error) in &refused {
        // Every refusal is a typed refusal that names its node, never a parse
        // failure or a silent empty model.
        assert!(
            matches!(error, ReadError::RequiredSlotUnfilled { .. }),
            "{} was refused by {error}",
            path.display()
        );
    }
    // A ratchet: the pack reads what it read when the reader landed, and a
    // change that reads fewer is a regression rather than a new baseline.
    assert!(
        read >= 121,
        "only {read} of {} templates read",
        templates().len()
    );
    assert_eq!(read + refused.len(), templates().len());
}

#[test]
fn every_node_of_every_template_carries_the_model_it_owes() {
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        assert!(!template.id().as_str().is_empty(), "{}", path.display());
        assert!(
            template.root().identity().archetype_id().is_some(),
            "{} has a root that is not an archetype root",
            path.display()
        );
        for node in template.walk() {
            assert!(
                !node.identity().rm_type().as_str().is_empty(),
                "{} carries a node with no RM type",
                path.display()
            );
            // A node the template removed with `occurrences matches {0}` is
            // absent, so nothing in the model is prohibited.
            assert!(
                !node.occurrences().is_prohibited(),
                "{} kept a node the template removed",
                path.display()
            );
            assert!(
                template.terminology(node.terminology_scope()).is_some(),
                "{} has a node whose terminology scope {} the template does not carry",
                path.display(),
                node.terminology_scope()
            );
        }
    }
}

#[test]
fn an_open_slot_is_recorded_and_never_rendered_as_a_guess() {
    let mut slots = 0_usize;
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        for node in template.walk() {
            if let ConstraintPayload::OpenSlot(_) = *node.payload() {
                slots += 1;
                // An open slot admits nothing determinate, so it holds no
                // fields of its own.
                assert!(node.children().is_empty(), "{}", path.display());
                // Only a slot the template does not require survives; a
                // required one is refused.
                assert!(!node.occurrences().is_mandatory(), "{}", path.display());
            }
        }
    }
    assert!(slots > 0, "the pack carries open slots and none were read");
}
