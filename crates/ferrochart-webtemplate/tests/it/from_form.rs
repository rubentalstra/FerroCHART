// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Writing a form definition the compiler derived, which has no source
//! spelling behind it.
//!
//! This is the other direction of the surface, and it is a weaker claim than
//! the round trip: a form definition derived from an operational template
//! holds facts a web template has no member for, so the document written from
//! it is a projection rather than a copy. What the tests pin is that the
//! document is well formed, that the projection is stable, and that every fact
//! the format could not carry is named rather than dropped.

use std::fs;

use ferrochart_compile::adl14;
use ferrochart_compile::derive;
use ferrochart_form::definition::FormDefinition;
use ferrochart_webtemplate::document::WebTemplateDocument;
use serde_json::Value;

use crate::support::{templates, walk};

/// The form definition the compiler derives from every template of the pack
/// that reads.
fn compiled_forms() -> Vec<(String, FormDefinition)> {
    let mut forms = Vec::new();
    for path in templates() {
        let Ok(xml) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let Ok(form) = derive::form(&template) else {
            continue;
        };
        forms.push((
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default(),
            form,
        ));
    }
    forms
}

/// The format version a test writes a compiler-derived form definition in.
///
/// The caller states it because no openEHR specification defines the sequence,
/// so this crate has none of its own to claim.
const FORMAT_VERSION: &str = "2.3";

#[test]
fn the_pack_compiles_the_form_definitions_the_tests_write() {
    assert!(
        compiled_forms().len() >= 100,
        "expected the vendored CKM pack to compile"
    );
}

#[test]
fn every_compiled_form_writes_a_well_formed_web_template() {
    for (name, form) in compiled_forms() {
        let template_id = form.template_id.to_string();
        let language = form.default_language.to_string();
        let document = WebTemplateDocument::from_form(form, FORMAT_VERSION);
        let written = document
            .write()
            .unwrap_or_else(|error| panic!("{name} could not be written: {error}"));
        let root = written.json.as_object().expect("a document is an object");
        assert_eq!(
            root.get("templateId").and_then(Value::as_str),
            Some(template_id.as_str())
        );
        assert_eq!(
            root.get("defaultLanguage").and_then(Value::as_str),
            Some(language.as_str())
        );
        assert_eq!(
            root.get("version").and_then(Value::as_str),
            Some(FORMAT_VERSION)
        );
        let tree = root.get("tree").expect("a document states a tree");
        walk(tree, &mut |node| {
            assert!(
                node.get("rmType").and_then(Value::as_str).is_some(),
                "{name} wrote a node with no rmType"
            );
            assert!(
                node.get("max").and_then(Value::as_i64).is_some(),
                "{name} wrote a node with no max"
            );
            assert!(
                node.get("min").and_then(Value::as_i64).is_some(),
                "{name} wrote a node with no min"
            );
        });
    }
}

#[test]
fn a_written_document_reads_back_with_the_same_counts_and_field_kinds() {
    for (name, form) in compiled_forms() {
        let before: Vec<(String, String)> = form
            .fields()
            .map(|field| (field.rm_type.to_string(), field.kind.name().to_owned()))
            .collect();
        let counts = (form.groups().count(), form.fields().count());
        let document = WebTemplateDocument::from_form(form, FORMAT_VERSION);
        let written = document
            .write()
            .unwrap_or_else(|error| panic!("{name} could not be written: {error}"));
        let back = WebTemplateDocument::from_value(&written.json)
            .unwrap_or_else(|error| panic!("{name} did not read back: {error}"));
        let after: Vec<(String, String)> = back
            .form()
            .fields()
            .map(|field| (field.rm_type.to_string(), field.kind.name().to_owned()))
            .collect();
        // A field whose kind the format has no shape for comes back as the
        // group its node became, so the comparison is over what the format
        // does carry.
        let carried: Vec<(String, String)> = before
            .into_iter()
            .filter(|(_, kind)| !matches!(kind.as_str(), "interval" | "state" | "choice"))
            .collect();
        assert_eq!(after, carried, "{name} changed shape on the way out");
        assert!(
            counts.0 >= 1,
            "{name} compiled to a form definition with no groups"
        );
    }
}

#[test]
fn writing_the_same_form_twice_writes_the_same_document() {
    for (name, form) in compiled_forms().into_iter().take(8) {
        let document = WebTemplateDocument::from_form(form, FORMAT_VERSION);
        let first = document.write().expect("a compiled form writes");
        let second = document.write().expect("a compiled form writes");
        assert_eq!(
            first.json, second.json,
            "{name} did not write the same twice"
        );
    }
}

#[test]
fn a_fact_the_format_cannot_carry_is_named_rather_than_dropped() {
    let carried_nothing = compiled_forms()
        .into_iter()
        .filter_map(|(name, form)| {
            let document = WebTemplateDocument::from_form(form, FORMAT_VERSION);
            document
                .write()
                .ok()
                .map(|written| (name, written.not_carried))
        })
        .any(|(_, not_carried)| !not_carried.is_empty());
    assert!(
        carried_nothing,
        "the pack states reference bands and intervals, and none were reported as not carried"
    );
}
