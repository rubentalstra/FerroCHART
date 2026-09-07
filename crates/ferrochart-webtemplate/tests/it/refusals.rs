// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the reader refuses, and the error it refuses with.
//!
//! A document this crate cannot read is refused with a typed error naming the
//! node, never absorbed into a form that would admit more than the document
//! states.

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{
    CountField, FieldKind, FormField, NullFlavour, ProportionField, ProportionKind, RealField,
};
use ferrochart_form::group::{FormGroup, FormItem, GroupShape};
use ferrochart_form::ids::{LanguageTag, RmTypeName, TemplateId};
use ferrochart_form::key::NodeKey;
use ferrochart_form::occurrences::Occurrences;
use ferrochart_form::text::Localized;
use ferrochart_webtemplate::document::WebTemplateDocument;
use ferrochart_webtemplate::error::{ReadError, WriteError};
use serde_json::json;

fn document(tree: &serde_json::Value) -> serde_json::Value {
    json!({
        "templateId": "ferro.test",
        "version": "2.3",
        "defaultLanguage": "en",
        "languages": ["en"],
        "tree": tree,
    })
}

/// What reading a document with this tree came to.
///
/// The outcome rather than the error, so a test asserts on both halves and a
/// document that is wrongly accepted fails loudly.
fn read(tree: &serde_json::Value) -> Result<WebTemplateDocument, ReadError> {
    WebTemplateDocument::from_value(&document(tree))
}

#[test]
fn a_document_that_is_not_json_is_refused_as_such() {
    let outcome = WebTemplateDocument::read("{");
    assert!(matches!(outcome, Err(ReadError::NotJson(_))), "{outcome:?}");
}

#[test]
fn a_document_with_no_tree_is_refused_by_name() {
    let outcome = WebTemplateDocument::from_value(&json!({
        "templateId": "ferro.test",
        "defaultLanguage": "en",
    }));
    assert!(
        matches!(
            outcome,
            Err(ReadError::MissingMember { member: "tree", .. })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_node_with_no_rm_type_is_refused_by_name() {
    let outcome = read(&json!({"id": "root", "aqlPath": "", "min": 1, "max": 1}));
    assert!(
        matches!(
            outcome,
            Err(ReadError::MissingMember {
                member: "rmType",
                ..
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_node_that_states_both_children_and_inputs_is_refused() {
    let outcome = read(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 1,
        "max": 1,
        "inputs": [{"type": "TEXT"}],
        "children": [{"id": "c", "rmType": "DV_TEXT", "aqlPath": "/x", "min": 0, "max": 1}],
    }));
    assert!(
        matches!(outcome, Err(ReadError::ChildrenAndInputs { .. })),
        "{outcome:?}"
    );
}

#[test]
fn an_input_type_this_format_does_not_define_is_refused() {
    let outcome = read(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 1,
        "max": 1,
        "children": [{
            "id": "c",
            "rmType": "DV_TEXT",
            "aqlPath": "/content[at0001]/value",
            "min": 0,
            "max": 1,
            "inputs": [{"type": "SLIDER"}],
        }],
    }));
    assert!(
        matches!(outcome, Err(ReadError::UnknownInputType { .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_proportion_type_the_reference_model_does_not_name_is_refused() {
    // openEHR RM Release-1.1.0 `data_types.html` section 6.2.11 names five
    // PROPORTION_KIND values and no others.
    let outcome = read(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 1,
        "max": 1,
        "children": [{
            "id": "c",
            "rmType": "DV_PROPORTION",
            "aqlPath": "/content[at0001]/value",
            "min": 0,
            "max": 1,
            "proportionTypes": ["per_mille"],
            "inputs": [
                {"suffix": "numerator", "type": "DECIMAL"},
                {"suffix": "denominator", "type": "DECIMAL"},
            ],
        }],
    }));
    assert!(
        matches!(outcome, Err(ReadError::UnknownProportionType { .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_count_that_no_node_can_carry_is_refused_rather_than_repaired() {
    let outcome = read(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 4,
        "max": 2,
    }));
    assert!(
        matches!(
            outcome,
            Err(ReadError::ImpossibleOccurrences { min: 4, max: 2, .. })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_member_stated_as_the_wrong_kind_of_value_is_refused_by_name() {
    let outcome = read(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 1,
        "max": 1,
        "localizedNames": "Blood pressure",
    }));
    assert!(
        matches!(
            outcome,
            Err(ReadError::WrongType {
                member: "localizedNames",
                ..
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_quantity_whose_inputs_are_not_its_shape_still_reads_as_a_quantity() {
    // A leaf the derivation table has a row for reads even when the document
    // states fewer inputs than the published implementations write, because
    // the missing suffix means the constraint stated nothing rather than that
    // the document is malformed.
    let read = WebTemplateDocument::from_value(&document(&json!({
        "id": "root",
        "rmType": "COMPOSITION",
        "aqlPath": "",
        "min": 1,
        "max": 1,
        "children": [{
            "id": "c",
            "rmType": "DV_QUANTITY",
            "aqlPath": "/content[at0001]/value",
            "min": 0,
            "max": 1,
            "inputs": [{"suffix": "magnitude", "type": "DECIMAL"}],
        }],
    })))
    .expect("a quantity with no unit list");
    let field = read.form().fields().next().expect("one field");
    assert_eq!(field.kind.name(), "quantity");
}

#[test]
fn a_proportion_kind_the_reference_model_does_not_name_is_refused_on_the_way_out() {
    // The form definition keeps a PROPORTION_KIND integer outside the five
    // rather than dropping it (openEHR RM Release-1.1.0 `data_types.html`
    // section 6.2.11), and a web template spells a proportion type by name, so
    // there is nothing to write and the write says so.
    let kind = FieldKind::Proportion(ProportionField {
        kinds: vec![ProportionKind::from_code(9)],
        numerator: RealField::default(),
        denominator: RealField::default(),
        decimals: CountField::default(),
        is_integral: None,
    });
    let field = FormField {
        key: NodeKey::root(),
        rm_type: RmTypeName::new("DV_PROPORTION"),
        label: Localized::empty(),
        help: Localized::empty(),
        occurrences: Occurrences::bounded(0, 1),
        kind,
        name_constraint: None,
        reference_ranges: None,
        is_ordered: false,
        is_unique: false,
        null_flavour: NullFlavour::absent(),
        prefill: None,
        is_deprecated: false,
        is_fixed: false,
    };
    let form = FormDefinition {
        format_version: FORMAT_VERSION,
        template_id: TemplateId::new("ferro.test"),
        default_language: LanguageTag::new("en"),
        languages: vec![LanguageTag::new("en")],
        root: FormGroup {
            key: NodeKey::root(),
            rm_type: RmTypeName::new("COMPOSITION"),
            archetype_id: None,
            label: Localized::empty(),
            help: Localized::empty(),
            occurrences: Occurrences::bounded(1, 1),
            shape: GroupShape::Plain,
            name_constraint: None,
            is_ordered: false,
            is_unique: false,
            is_deprecated: false,
            items: vec![FormItem::Field(Box::new(field))],
            undetermined: Vec::new(),
        },
    };
    let outcome = WebTemplateDocument::from_form(form, "2.3").write();
    assert!(
        matches!(
            outcome,
            Err(WriteError::UnnameableProportionKind { ref kind, .. }) if kind == "9"
        ),
        "{outcome:?}"
    );
}
