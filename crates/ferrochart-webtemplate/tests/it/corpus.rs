// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The whole vendored pack, read into a form definition and written back.

use ferrochart_webtemplate::document::WebTemplateDocument;
use serde_json::Value;

use crate::support::{one_web_template, walk, web_templates};

#[test]
fn the_pack_builds_the_web_templates_the_tests_read() {
    let built = web_templates();
    assert!(
        built.len() >= 100,
        "expected the vendored CKM pack, built {} web templates",
        built.len()
    );
}

#[test]
fn every_web_template_of_the_pack_reads_into_a_form_definition() {
    for (name, json) in web_templates() {
        let document = WebTemplateDocument::from_value(&json)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        assert!(
            document.form().is_current_format(),
            "{name} produced a form definition in another format version"
        );
        assert_eq!(
            document.form().template_id.as_str(),
            json.get("templateId").and_then(Value::as_str).unwrap_or(""),
            "{name} lost its template id"
        );
    }
}

#[test]
fn every_web_template_of_the_pack_writes_back_as_the_document_it_came_from() {
    for (name, json) in web_templates() {
        let document = WebTemplateDocument::from_value(&json)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        let written = document
            .write()
            .unwrap_or_else(|error| panic!("{name} could not be written: {error}"));
        assert_eq!(written.json, json, "{name} did not survive the round trip");
        assert!(
            written.not_carried.is_empty(),
            "{name} lost a fact it never carried: {:?}",
            written.not_carried
        );
    }
}

#[test]
fn every_node_of_the_pack_reaches_the_form_definition() {
    for (name, json) in web_templates() {
        let mut nodes = 0_usize;
        walk(json.get("tree").unwrap_or(&Value::Null), &mut |_| {
            nodes = nodes.saturating_add(1);
        });
        let document = WebTemplateDocument::from_value(&json)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        let read = document.source().nodes().count();
        assert_eq!(
            read, nodes,
            "{name} states {nodes} nodes and {read} reached the form definition"
        );
    }
}

#[test]
fn min_and_max_are_read_as_the_integers_they_are() {
    // A web template states the flattened counts rather than an occurrences
    // interval to re-derive, and `max` of -1 is the unbounded node.
    let json = one_web_template().expect("the vendored CKM pack");
    let document = WebTemplateDocument::from_value(&json).expect("a document the pack built");
    let mut checked = 0_usize;
    walk(json.get("tree").expect("a tree"), &mut |node| {
        let Some(max) = node.get("max").and_then(Value::as_i64) else {
            return;
        };
        let min = node.get("min").and_then(Value::as_i64).unwrap_or(0);
        let Some(found) = document
            .form()
            .groups()
            .map(|group| group.occurrences)
            .chain(document.form().fields().map(|field| field.occurrences))
            .find(|occurrences| {
                i64::from(occurrences.minimum) == min
                    && occurrences.maximum.map_or(-1_i64, i64::from) == max
            })
        else {
            panic!("no item carries min {min} and max {max}");
        };
        if max == -1 {
            assert!(found.maximum.is_none(), "an unbounded node gained a bound");
        }
        checked = checked.saturating_add(1);
    });
    assert!(checked > 0, "the document states no counts at all");
}

#[test]
fn a_node_the_form_definition_has_no_field_for_is_named_rather_than_lost() {
    // A web template states inputs for the PARTY_PROXY family, which the
    // derivation table of docs/architecture.md section 5 has no row for.
    let json = one_web_template().expect("the vendored CKM pack");
    let document = WebTemplateDocument::from_value(&json).expect("a document the pack built");
    assert!(
        !document.unmodelled_nodes().is_empty(),
        "the pack states party nodes and none were reported"
    );
    let written = document
        .write()
        .expect("a document that came from the pack");
    assert_eq!(written.json, json, "a node with no field lost its members");
}
