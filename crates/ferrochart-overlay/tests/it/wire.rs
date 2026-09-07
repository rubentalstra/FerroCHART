// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The wire the overlay document is stored on.
//!
//! The overlay is written to disk and replayed against a later recompile, so
//! its bytes are stored user work rather than an internal representation. A
//! change to them is a change to what an existing document means, and that is
//! what makes it worth pinning beside the form definition
//! (`docs/architecture.md` section 6.1).
//!
//! Two assertions, and they catch different things. The round trip catches a
//! document that no longer reads back, which is how a serde feature turned on
//! by an unrelated dependency took the form definition off the wire. The
//! snapshot catches a document that still reads back and no longer says the
//! same thing, which a round trip cannot see. Read a change with
//! `cargo insta review` before accepting it (`.claude/rules/testing.md`).
//!
//! The material is the committed CKM operational template pack, read and
//! derived the way every other test in this binary reads it. Every layout
//! authored here is synthetic content invented for the test.

use ferrochart_form::ids::{LanguageTag, TerminologyName};
use ferrochart_form::layout::{
    CharacterWidth, ColumnCount, ColumnSpan, Condition, Geometry, HintName, Layout, Section,
    SectionId, Visibility, WidgetName,
};
use ferrochart_form::text::Localized;
use ferrochart_form::value::{Code, Prefill};
use ferrochart_overlay::store::{Author, Overlay};

use crate::support::{form, keys, tied_key};

const TEMPLATE: &str = "aedes-indices-jm.opt";

fn english() -> LanguageTag {
    LanguageTag::new("en")
}

fn text(value: &str) -> Localized {
    Localized::in_language(english(), value)
}

/// The value a person authored as a default, one shape per position.
///
/// Three of the nine [`Prefill`] variants carry an `f64`, and those are the
/// ones a change to how serde buffers a number reaches, so the cycle spends
/// three of its four positions on them and leaves the fourth undecided.
fn default_at(position: usize) -> Option<Prefill> {
    let code = || Code::new(TerminologyName::new("local"), "at0004");
    match position % 4 {
        0 => Some(Prefill::Real(2.5)),
        1 => Some(Prefill::Quantity {
            magnitude: 37.5,
            units: "Cel".to_owned(),
            precision: Some(1),
        }),
        2 => Some(Prefill::Ordinal {
            value: 1.0,
            symbol: code(),
        }),
        _ => None,
    }
}

/// An overlay in which every member of every stored record is decided.
///
/// A document with the optional half left out pins only the half that is
/// there, so the golden below is built from a form every node of which carries
/// a layout, in two sections one of which sits inside the other.
fn authored() -> Overlay {
    let definition = form(TEMPLATE);
    let mut author = Author::new(&definition);
    author
        .add_section(
            Section::new(SectionId::new("counts"), text("Counts"))
                .on_columns(ColumnCount::try_from(4).unwrap()),
        )
        .unwrap();
    let mut nested = Section::new(SectionId::new("counts-detail"), text("Detail"))
        .on_columns(ColumnCount::try_from(3).unwrap());
    nested.parent = Some(SectionId::new("counts"));
    nested.help = text("The numbers behind the indices.");
    nested.order = Some(2);
    author.add_section(nested).unwrap();

    let all = keys(&definition);
    let watched = all.first().expect("the form has a root").clone();
    for (position, key) in all.into_iter().enumerate() {
        let mut layout = Layout {
            order: Some(u32::try_from(position).unwrap()),
            section: Some(if position % 2 == 0 {
                SectionId::new("counts")
            } else {
                SectionId::new("counts-detail")
            }),
            label: text(&format!("Field {position}")),
            help: text("What the number means."),
            default: default_at(position),
            widget: Some(WidgetName::new("segmented")),
            geometry: Geometry {
                span: Some(ColumnSpan::try_from(2).unwrap()),
                break_before: position % 3 == 0,
                width: Some(CharacterWidth::try_from(12).unwrap()),
            },
            ..Layout::new()
        };
        // The hints go in out of tag order, because the document promises they
        // come back in name order.
        for name in ["zebra", "alpha"] {
            layout
                .hints
                .insert(HintName::new(name), format!("{name}-{position}"));
        }
        layout.visibility = match position % 3 {
            0 => Visibility::Always,
            1 => Visibility::Never,
            _ => Visibility::When {
                condition: Condition::All {
                    conditions: vec![
                        Condition::Answered {
                            node: watched.clone(),
                        },
                        Condition::Equals {
                            node: watched.clone(),
                            value: Prefill::Real(2.5),
                        },
                    ],
                },
            },
        };
        let _ = author.set(&key, layout).unwrap();
    }
    author.finish()
}

#[test]
fn a_fully_authored_overlay_round_trips_through_its_serialisation() {
    // The regression #103 left, on the other published document: every
    // number-bearing shape the overlay can store is in this one, and each is
    // read back rather than only written.
    let overlay = authored();
    let once = overlay.to_json().unwrap();
    let twice = overlay.to_json().unwrap();
    assert_eq!(once, twice, "the same overlay writes the same bytes");

    let parsed = Overlay::from_json(&once).unwrap();
    assert_eq!(parsed, overlay);
    assert_eq!(parsed.to_json().unwrap(), once);
}

#[test]
fn the_overlay_wire_is_unchanged() {
    insta::assert_snapshot!("overlay_wire", authored().to_json().unwrap());
}

#[test]
fn the_overlay_wire_of_a_positionally_keyed_entry_is_unchanged() {
    // The anchor is the one stored record the document above cannot show: no
    // key of that template ties, and the anchor is what a replay reads to tell
    // an unchanged container from a reordered one.
    let definition = form("clinical-context-jm.opt");
    let key = tied_key(&definition).expect("the pack carries a same-id sibling group");
    let mut author = Author::new(&definition);
    let mut layout = Layout::new();
    layout.label = text("Weight");
    let _ = author.set(&key, layout).unwrap();
    let overlay = author.finish();

    let json = overlay.to_json().unwrap();
    assert_eq!(Overlay::from_json(&json).unwrap(), overlay);
    insta::assert_snapshot!("overlay_wire_anchored", json);
}
