// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The overlay store: its format, what it refuses, and its round trip.
//!
//! The material is the committed CKM operational template pack. Every layout
//! authored here is synthetic content invented for the test.

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::text::Localized;
use ferrochart_overlay::layout::{Layout, Section, SectionId, Visibility, WidgetName};
use ferrochart_overlay::store::{Author, Overlay, Placement, TemplateIdForm};

use crate::support::{form, key_where, keys, tied_key};

fn english() -> LanguageTag {
    LanguageTag::new("en")
}

fn authored(text: &str) -> Layout {
    Layout {
        order: Some(3),
        label: Localized::in_language(english(), text),
        widget: Some(WidgetName::new("segmented")),
        ..Layout::new()
    }
}

#[test]
fn an_overlay_round_trips_and_writes_the_same_bytes_every_time() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    author
        .add_section(Section::new(
            SectionId::new("counts"),
            Localized::in_language(english(), "Counts"),
        ))
        .unwrap();
    for (position, key) in keys(&definition).into_iter().enumerate() {
        let mut layout = authored(&format!("Field {position}"));
        layout.section = Some(SectionId::new("counts"));
        let _ = author.set(&key, layout).unwrap();
    }
    let overlay = author.finish();

    let once = overlay.to_json().unwrap();
    let twice = overlay.to_json().unwrap();
    assert_eq!(once, twice, "the same overlay writes the same bytes");

    let parsed = Overlay::from_json(&once).unwrap();
    assert_eq!(parsed, overlay);
    assert_eq!(parsed.to_json().unwrap(), once);
}

#[test]
fn the_reader_refuses_a_document_written_in_another_format_version() {
    let definition = form("aedes-indices-jm.opt");
    let overlay = Author::new(&definition).finish();
    let mut document = document_of(&overlay);
    document["format_version"] = serde_json::json!(2);
    let refused = read(&document).unwrap_err();
    assert!(
        refused.to_string().contains("format version 2"),
        "{refused}"
    );
}

#[test]
fn an_entry_that_decorates_no_node_of_the_form_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let other = form("birth-detail-jm.opt");
    let stranger = keys(&other).pop().unwrap();
    let refused = Author::new(&definition)
        .set(&stranger, Layout::new())
        .unwrap_err();
    assert!(
        refused.to_string().contains("no group and no field"),
        "{refused}"
    );
}

#[test]
fn a_key_that_collides_with_a_same_id_sibling_is_reported_rather_than_resolved() {
    // The pack carries the collision the measurement on issue #8 found: two
    // siblings under one attribute with one node id, one archetype id, one
    // Reference Model type and one pinned name.
    let definition = form("clinical-context-jm.opt");
    let key = tied_key(&definition).expect("the pack carries a same-id sibling group");
    let mut author = Author::new(&definition);
    let placement = author.set(&key, authored("Weight")).unwrap();
    assert_eq!(placement, Placement::Positional { tied: 2 });

    let overlay = author.finish();
    let entry = overlay.entry(&key).unwrap();
    assert!(
        entry.key.is_positional,
        "a colliding entry is stored as positionally keyed"
    );
    let anchor = entry
        .anchor
        .as_ref()
        .expect("a positional entry is anchored");
    assert_eq!(anchor.tied_siblings.len(), 2);
}

#[test]
fn an_entry_the_definition_tells_apart_carries_no_anchor() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    assert_eq!(author.set(&key, Layout::new()).unwrap(), Placement::Unique);
    let overlay = author.finish();
    let entry = overlay.entry(&key).unwrap();
    assert!(!entry.key.is_positional);
    assert!(entry.anchor.is_none());
}

#[test]
fn the_overlay_records_which_form_of_the_template_identifier_it_was_keyed_against() {
    // Every template in the pack states a legacy name, so none of them pins a
    // release: openEHR ITS-REST Release-1.1.0 definition.html, "Get a
    // template", leaves the server to resolve one.
    let definition = form("aedes-indices-jm.opt");
    let overlay = Author::new(&definition).finish();
    assert_eq!(overlay.template().id, definition.template_id);
    assert_eq!(overlay.template().id_form, TemplateIdForm::LegacyName);
    assert!(!overlay.template().id_form.pins_one_release());
}

#[test]
fn an_overlay_keyed_against_another_template_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let other = form("birth-detail-jm.opt");
    let overlay = Author::new(&other).finish();
    let refused = Author::resume(&definition, overlay).unwrap_err();
    assert!(
        refused.to_string().contains("keyed against template"),
        "{refused}"
    );
}

#[test]
fn a_section_naming_a_parent_the_overlay_does_not_define_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    let mut section = Section::new(SectionId::new("child"), Localized::empty());
    section.parent = Some(SectionId::new("nowhere"));
    let refused = author.add_section(section).unwrap_err();
    assert!(refused.to_string().contains("nowhere"), "{refused}");
}

#[test]
fn a_chain_of_section_parents_that_closes_on_itself_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    author
        .add_section(Section::new(SectionId::new("one"), Localized::empty()))
        .unwrap();
    let mut two = Section::new(SectionId::new("two"), Localized::empty());
    two.parent = Some(SectionId::new("one"));
    author.add_section(two).unwrap();
    let mut one = Section::new(SectionId::new("one"), Localized::empty());
    one.parent = Some(SectionId::new("two"));
    let refused = author.add_section(one).unwrap_err();
    assert!(
        refused.to_string().contains("its own ancestor"),
        "{refused}"
    );
}

#[test]
fn an_entry_naming_a_section_the_overlay_does_not_define_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut layout = Layout::new();
    layout.section = Some(SectionId::new("nowhere"));
    let refused = Author::new(&definition).set(&key, layout).unwrap_err();
    assert!(refused.to_string().contains("nowhere"), "{refused}");
}

#[test]
fn a_document_that_decorates_one_node_twice_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let _ = author.set(&key, authored("Once")).unwrap();
    let mut document = document_of(&author.finish());
    let entry = document["entries"][0].clone();
    document["entries"].as_array_mut().unwrap().push(entry);
    let refused = read(&document).unwrap_err();
    assert!(
        refused.to_string().contains("two entries decorate"),
        "{refused}"
    );
}

#[test]
fn an_entry_that_claims_a_position_without_an_anchor_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let _ = author.set(&key, authored("Once")).unwrap();
    let mut document = document_of(&author.finish());
    document["entries"][0]["key"]["is_positional"] = serde_json::json!(true);
    let refused = read(&document).unwrap_err();
    assert!(
        refused.to_string().contains("positionally keyed = true"),
        "{refused}"
    );
}

#[test]
fn an_entry_carries_the_key_the_anchor_and_the_layout_and_nothing_else() {
    // Nothing in the overlay repeats what the compiled definition already
    // carries: an entry is a key plus what a person authored.
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let _ = author.set(&key, authored("Once")).unwrap();
    let document = document_of(&author.finish());
    let entry = document["entries"][0].as_object().unwrap();
    let members: Vec<&String> = entry.keys().collect();
    assert_eq!(members, ["key", "rm_type", "anchor", "layout"]);
    let layout: Vec<&String> = document["entries"][0]["layout"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(
        layout,
        [
            "order",
            "section",
            "label",
            "help",
            "default",
            "visibility",
            "widget"
        ]
    );
}

#[test]
fn a_hidden_item_is_a_layout_decision_and_not_a_change_to_the_definition() {
    let definition = form("aedes-indices-jm.opt");
    let key = key_where(&definition, |step| step.node_id.is_some()).unwrap();
    let mut author = Author::new(&definition);
    let mut layout = Layout::new();
    layout.visibility = Visibility::Never;
    let _ = author.set(&key, layout).unwrap();
    let overlay = author.finish();
    assert_eq!(
        overlay.entry(&key).unwrap().layout.visibility,
        Visibility::Never
    );
    assert_eq!(
        definition,
        form("aedes-indices-jm.opt"),
        "authoring a layout changes no definition"
    );
}

#[test]
fn a_person_can_take_an_entry_out_again() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let _ = author.set(&key, authored("Once")).unwrap();
    let mut overlay = author.finish();
    assert!(overlay.remove(&key).is_some());
    assert!(overlay.entries().is_empty());
    assert!(overlay.remove(&key).is_none());
}

/// One overlay, as the JSON it writes.
fn document_of(overlay: &Overlay) -> serde_json::Value {
    serde_json::from_str(&overlay.to_json().unwrap()).unwrap()
}

/// One JSON document, read back as an overlay.
fn read(document: &serde_json::Value) -> Result<Overlay, ferrochart_overlay::error::OverlayError> {
    Overlay::from_json(&document.to_string())
}
