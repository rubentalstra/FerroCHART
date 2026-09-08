// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The wire the form definition is published on.
//!
//! `docs/architecture.md` section 11 makes the form definition a contract a
//! third party writes a renderer against, so what a renderer parses is pinned
//! here, in the crate that defines the format and links nothing else of this
//! tree. The document below is hand-built rather than derived, because a
//! contract test that needs the compiler and a template corpus to run is a
//! test of the compiler.
//!
//! Two assertions, and they catch different things. The round trip catches a
//! document that no longer reads back, which is how a serde feature turned on
//! by an unrelated dependency took this format off the wire. The snapshot
//! catches a document that still reads back here and no longer means the same
//! thing to a consumer written against the old bytes. Neither implies the
//! other, and `scripts/checks/serde-json-features.sh` is the third layer: it
//! names the cause where these two report the effect.
//!
//! Read a snapshot change with `cargo insta review` before accepting it, and
//! decide whether it is a [`FORMAT_VERSION`] bump
//! (`.claude/rules/testing.md`).

use std::collections::BTreeMap;

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{
    FieldKind, FormField, NullFlavour, OrdinalField, OrdinalOption, QuantityField,
    QuantityUnitOption, ReferenceRanges, TextField,
};
use ferrochart_form::group::{
    FormGroup, FormItem, GroupShape, SlotAssertion, UndeterminedContent, UndeterminedReason,
};
use ferrochart_form::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId, local_terminology,
};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::occurrences::Occurrences;
use ferrochart_form::range::Range;
use ferrochart_form::text::Localized;
use ferrochart_form::value::{Code, Prefill};

const ARCHETYPE: &str = "openEHR-EHR-OBSERVATION.ferro_wire.v1";

fn english() -> LanguageTag {
    LanguageTag::new("en")
}

fn label(text: &str) -> Localized {
    Localized::in_language(english(), text)
}

fn code(local: &str) -> Code {
    Code::new(local_terminology(), local)
}

fn root_key() -> NodeKey {
    NodeKey::root().child(KeyStep {
        rm_attribute: RmAttributeName::new(""),
        node_id: Some(LocalCode::new("at0000")),
        archetype_id: Some(ArchetypeId::new(ARCHETYPE)),
        rm_type: RmTypeName::new("OBSERVATION"),
        pinned_name: Some("Wire probe".to_owned()),
        sibling_ordinal: 0,
    })
}

fn child_key(parent: &NodeKey, attribute: &str, node_id: &str, rm_type: &str) -> NodeKey {
    parent.child(KeyStep {
        rm_attribute: RmAttributeName::new(attribute),
        node_id: Some(LocalCode::new(node_id)),
        archetype_id: None,
        rm_type: RmTypeName::new(rm_type),
        pinned_name: None,
        sibling_ordinal: 0,
    })
}

/// The scale field, whose score is the `f64` an internally tagged enum has to
/// carry through a buffered deserialization.
fn ordinal(parent: &NodeKey) -> FormField {
    FormField {
        key: child_key(parent, "items", "at0002", "ELEMENT"),
        rm_type: RmTypeName::new("DV_SCALE"),
        label: label("Breathlessness"),
        help: label("How short of breath the person is."),
        occurrences: Occurrences::bounded(1, 1),
        kind: FieldKind::Ordinal(OrdinalField {
            options: vec![
                OrdinalOption {
                    score: 0.0,
                    symbol: code("at0003"),
                    label: label("None"),
                    description: Localized::empty(),
                },
                OrdinalOption {
                    score: 1.5,
                    symbol: code("at0004"),
                    label: label("Some"),
                    description: label("Short of breath on exertion."),
                },
            ],
        }),
        name_constraint: None,
        reference_ranges: None,
        is_ordered: false,
        is_unique: false,
        null_flavour: NullFlavour::offered(),
        prefill: Some(Prefill::Ordinal {
            value: 1.5,
            symbol: code("at0004"),
        }),
        is_deprecated: false,
        is_fixed: false,
    }
}

/// The quantity field, whose magnitude bounds and prefill are the other two
/// `f64` shapes the format publishes.
fn quantity(parent: &NodeKey) -> FormField {
    FormField {
        key: child_key(parent, "items", "at0005", "ELEMENT"),
        rm_type: RmTypeName::new("DV_QUANTITY"),
        label: label("Body temperature"),
        help: Localized::empty(),
        occurrences: Occurrences::bounded(0, 1),
        kind: FieldKind::Quantity(QuantityField {
            property: Some(Code::new(
                ferrochart_form::ids::openehr_terminology(),
                "127",
            )),
            units: vec![QuantityUnitOption {
                units: "Cel".to_owned(),
                magnitude: Some(Range::new(Some(11.0), Some(44.0), true, true)),
                decimals: Some(Range::new(Some(1), Some(1), true, true)),
            }],
        }),
        name_constraint: None,
        reference_ranges: Some(Box::new(ReferenceRanges::default())),
        is_ordered: false,
        is_unique: false,
        null_flavour: NullFlavour::absent(),
        prefill: Some(Prefill::Quantity {
            magnitude: 37.0,
            units: "Cel".to_owned(),
            precision: Some(1),
        }),
        is_deprecated: false,
        is_fixed: false,
    }
}

/// A plain text field, so the document carries a kind that holds no number at
/// all beside the two that do.
fn note(parent: &NodeKey) -> FormField {
    FormField {
        key: child_key(parent, "items", "at0006", "ELEMENT"),
        rm_type: RmTypeName::new("DV_TEXT"),
        label: label("Comment"),
        help: Localized::empty(),
        occurrences: Occurrences::unbounded_from(0),
        kind: FieldKind::Text(TextField {
            options: Vec::new(),
            options_closed: false,
            patterns: Vec::new(),
        }),
        name_constraint: None,
        reference_ranges: None,
        is_ordered: true,
        is_unique: false,
        null_flavour: NullFlavour::absent(),
        prefill: None,
        is_deprecated: true,
        is_fixed: false,
    }
}

/// One form definition carrying every published shape that holds a number,
/// plus the nesting and the undetermined content around them.
fn definition() -> FormDefinition {
    let root = root_key();
    let tree = child_key(&root, "data", "at0001", "ITEM_TREE");
    FormDefinition {
        format_version: FORMAT_VERSION,
        template_id: TemplateId::new("Ferro wire probe"),
        default_language: english(),
        languages: vec![english()],
        root: FormGroup {
            key: root.clone(),
            rm_type: RmTypeName::new("OBSERVATION"),
            archetype_id: Some(ArchetypeId::new(ARCHETYPE)),
            label: label("Wire probe"),
            help: Localized::empty(),
            occurrences: Occurrences::bounded(1, 1),
            shape: GroupShape::Plain,
            name_constraint: None,
            is_ordered: true,
            is_unique: false,
            is_deprecated: false,
            items: vec![FormItem::Group(Box::new(FormGroup {
                key: tree.clone(),
                rm_type: RmTypeName::new("ITEM_TREE"),
                archetype_id: None,
                label: label("Tree"),
                help: Localized::empty(),
                occurrences: Occurrences::bounded(1, 1),
                shape: GroupShape::Tree,
                name_constraint: None,
                is_ordered: true,
                is_unique: false,
                is_deprecated: false,
                items: vec![
                    FormItem::Field(Box::new(ordinal(&tree))),
                    FormItem::Field(Box::new(quantity(&tree))),
                    FormItem::Field(Box::new(note(&tree))),
                ],
                undetermined: Vec::new(),
            }))],
            undetermined: vec![UndeterminedContent {
                key: child_key(&root, "protocol", "at0007", "ITEM_TREE"),
                rm_type: RmTypeName::new("ITEM_TREE"),
                label: label("Extension"),
                occurrences: Occurrences::unbounded_from(0),
                reason: UndeterminedReason::OpenSlot {
                    includes: vec![SlotAssertion::ArchetypeIdPattern(
                        "openEHR-EHR-CLUSTER\\..*\\.v1".to_owned(),
                    )],
                    excludes: Vec::new(),
                },
            }],
        },
    }
}

#[test]
fn a_form_definition_round_trips_through_serde_json() {
    // The regression #103 left: a dependency's serde feature stopped an
    // internally tagged enum carrying an f64 from deserializing, and every
    // number-bearing shape the format publishes is in this document.
    let form = definition();
    let json = serde_json::to_string(&form).unwrap();
    let back: FormDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(back, form);
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn a_form_definition_writes_the_same_bytes_every_time() {
    // The format claims byte-determinism, which is what makes the snapshot
    // below a contract rather than one recording of an arbitrary order.
    let form = definition();
    assert_eq!(
        serde_json::to_string(&form).unwrap(),
        serde_json::to_string(&definition()).unwrap()
    );
    let mut labels: BTreeMap<LanguageTag, String> = BTreeMap::new();
    labels.insert(LanguageTag::new("nl"), "Opmerking".to_owned());
    labels.insert(english(), "Comment".to_owned());
    assert_eq!(
        serde_json::to_string(&Localized {
            by_language: labels
        })
        .unwrap(),
        r#"{"en":"Comment","nl":"Opmerking"}"#,
        "the one map of the format writes in tag order"
    );
}

#[test]
fn the_form_definition_wire_is_unchanged() {
    insta::assert_snapshot!(
        "form_definition_wire",
        serde_json::to_string_pretty(&definition()).unwrap()
    );
}
