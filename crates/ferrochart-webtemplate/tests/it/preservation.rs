// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A member FerroCHART does not model survives a round trip verbatim.
//!
//! The pack the other tests read carries no annotations, so the documents here
//! are the pack's own with third-party members put into them: the case the
//! obligation exists for is a consumer who annotated a web template with
//! another tool and passed it through this one.

use ferrochart_form::ids::LanguageTag;
use ferrochart_webtemplate::document::WebTemplateDocument;
use serde_json::{Value, json};

use crate::support::{one_web_template, walk, walk_mut, web_templates};

/// The pack's own document with a member on every object this crate does not
/// model.
fn annotated(mut json: Value) -> Value {
    if let Some(root) = json.as_object_mut() {
        root.insert("semVer".to_owned(), json!("1.2.3"));
        root.insert(
            "otherDetails".to_owned(),
            json!({"MetaDataSet:Sample Set": "LOINC version 2.54"}),
        );
        root.insert("aThirdPartyMemberNobodyHereKnows".to_owned(), json!([1, 2]));
    }
    if let Some(tree) = json.get_mut("tree") {
        walk_mut(tree, &mut |node| {
            let Some(node) = node.as_object_mut() else {
                return;
            };
            node.insert(
                "annotations".to_owned(),
                json!({"ui:widget": "slider", "comment": "authored by hand"}),
            );
            node.insert("dependsOn".to_owned(), json!(["some_sibling"]));
            let Some(inputs) = node.get_mut("inputs").and_then(Value::as_array_mut) else {
                return;
            };
            for input in inputs {
                let Some(input) = input.as_object_mut() else {
                    continue;
                };
                input.insert("defaultValue".to_owned(), json!("a default"));
                let Some(list) = input.get_mut("list").and_then(Value::as_array_mut) else {
                    continue;
                };
                for option in list {
                    if let Some(option) = option.as_object_mut() {
                        option.insert(
                            "termBindings".to_owned(),
                            json!({"SNOMED-CT": {"value": "163020007", "terminologyId": "SNOMED-CT"}}),
                        );
                    }
                }
            }
        });
    }
    json
}

#[test]
fn a_member_this_crate_does_not_model_survives_the_whole_pack_verbatim() {
    for (name, json) in web_templates() {
        let source = annotated(json);
        let document = WebTemplateDocument::from_value(&source)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        let written = document
            .write()
            .unwrap_or_else(|error| panic!("{name} could not be written: {error}"));
        assert_eq!(
            written.json, source,
            "{name} lost a member the form definition does not model"
        );
    }
}

#[test]
fn the_annotations_reach_the_written_document_and_not_the_form_definition() {
    let source = annotated(one_web_template().expect("the vendored CKM pack"));
    let document = WebTemplateDocument::from_value(&source).expect("an annotated document");
    let written = document
        .write()
        .expect("a document that came from the pack");

    let mut annotated_nodes = 0_usize;
    walk(written.json.get("tree").expect("a tree"), &mut |node| {
        assert_eq!(
            node.get("annotations"),
            Some(&json!({"ui:widget": "slider", "comment": "authored by hand"})),
            "a node lost its annotations"
        );
        annotated_nodes = annotated_nodes.saturating_add(1);
    });
    assert!(annotated_nodes > 1, "the tree has one node");
    assert_eq!(
        written.json.get("otherDetails"),
        source.get("otherDetails"),
        "the document lost its other details"
    );
}

#[test]
fn changing_a_label_rewrites_that_node_and_leaves_every_other_member_alone() {
    let source = annotated(one_web_template().expect("the vendored CKM pack"));
    let mut document = WebTemplateDocument::from_value(&source).expect("an annotated document");
    let language = document.form().default_language.clone();

    let key = {
        let field = document
            .form()
            .fields()
            .next()
            .expect("the pack states fields")
            .clone();
        field.key
    };
    for field in field_iter(&mut document) {
        if field.key == key {
            field.label.insert(language.clone(), "A new label");
        }
    }

    let written = document
        .write()
        .expect("a document that came from the pack");
    let mut changed = 0_usize;
    let mut checked = 0_usize;
    walk(written.json.get("tree").expect("a tree"), &mut |node| {
        checked = checked.saturating_add(1);
        // Whatever else the rewrite touched, the members this crate does not
        // model are the source's own on every node.
        assert_eq!(
            node.get("annotations"),
            Some(&json!({"ui:widget": "slider", "comment": "authored by hand"}))
        );
        assert!(node.get("aqlPath").is_some(), "a node lost its aqlPath");
        assert!(node.get("id").is_some(), "a node lost its id");
        // The rule holds one and two levels down as well, so a rewrite does
        // not cost a per-input default or a per-option terminology binding.
        for input in node
            .get("inputs")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            if input.get("suffix").and_then(Value::as_str) == Some("other") {
                continue;
            }
            assert_eq!(
                input.get("defaultValue"),
                Some(&json!("a default")),
                "an input lost its default value"
            );
            for option in input
                .get("list")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                assert!(
                    option.get("termBindings").is_some(),
                    "a coded option lost its terminology binding"
                );
            }
        }
        if node.get("localizedName") == Some(&json!("A new label")) {
            changed = changed.saturating_add(1);
            assert_eq!(
                node.get("localizedNames")
                    .and_then(|names| names.get(language.as_str())),
                Some(&json!("A new label")),
                "the per-language label did not follow the change"
            );
        }
    });
    assert_eq!(changed, 1, "exactly one node was relabelled");
    assert!(checked > 1, "the tree has one node");
    assert_ne!(
        written.json, source,
        "the change did not reach the document"
    );
}

/// Every field of the document's form definition, to change in place.
fn field_iter(document: &mut WebTemplateDocument) -> Vec<&mut ferrochart_form::field::FormField> {
    fn collect<'a>(
        group: &'a mut ferrochart_form::group::FormGroup,
        out: &mut Vec<&'a mut ferrochart_form::field::FormField>,
    ) {
        for item in &mut group.items {
            match *item {
                ferrochart_form::group::FormItem::Group(ref mut nested) => collect(nested, out),
                ferrochart_form::group::FormItem::Field(ref mut field) => out.push(field),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(&mut document.form_mut().root, &mut out);
    out
}

#[test]
fn a_source_spelling_can_be_stored_and_read_back_beside_its_form_definition() {
    // The spelling is what a consumer's annotations live in between a read and
    // a write, so it has to survive being put away.
    let source = annotated(one_web_template().expect("the vendored CKM pack"));
    let document = WebTemplateDocument::from_value(&source).expect("an annotated document");
    let stored = serde_json::to_string(&document).expect("a document serializes");
    let back: WebTemplateDocument = serde_json::from_str(&stored).expect("a document deserializes");
    assert_eq!(
        back.write().expect("a stored document writes").json,
        source,
        "a stored document lost what it was holding"
    );
    let _unused: LanguageTag = back.form().default_language.clone();
}

#[test]
fn a_node_with_no_field_keeps_its_inputs_when_the_form_definition_changes() {
    // A PARTY_PROXY node states inputs and becomes a group, because the
    // derivation table of docs/architecture.md section 5 has no row for the
    // class. Relabelling the group must not cost it the inputs the form
    // definition never held.
    let source = annotated(one_web_template().expect("the vendored CKM pack"));
    let mut document = WebTemplateDocument::from_value(&source).expect("an annotated document");
    let key = document
        .unmodelled_nodes()
        .first()
        .cloned()
        .expect("the pack states party nodes");

    let language = document.form().default_language.clone();
    let mut relabelled = false;
    for_each_group(&mut document.form_mut().root, &mut |group| {
        if group.key == key {
            group.label.insert(language.clone(), "A new label");
            relabelled = true;
        }
    });
    assert!(relabelled, "the node with no field was not found");

    let written = document
        .write()
        .expect("a document that came from the pack");
    let mut found = false;
    walk(written.json.get("tree").expect("a tree"), &mut |node| {
        if node.get("localizedName") != Some(&json!("A new label")) {
            return;
        }
        found = true;
        let inputs = node
            .get("inputs")
            .and_then(Value::as_array)
            .expect("the node kept the inputs it arrived with");
        assert!(!inputs.is_empty(), "the node kept an empty inputs array");
    });
    assert!(found, "the relabelled node did not reach the document");
}

/// Every group of the document's form definition, root first, to change in
/// place.
fn for_each_group(
    group: &mut ferrochart_form::group::FormGroup,
    visit: &mut dyn FnMut(&mut ferrochart_form::group::FormGroup),
) {
    visit(group);
    for item in &mut group.items {
        if let ferrochart_form::group::FormItem::Group(ref mut child) = *item {
            for_each_group(child, visit);
        }
    }
}
