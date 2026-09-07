// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The column grid: what it admits, what it advises against, and what a
//! recompile says about it.
//!
//! No specification governs any of this, so the tests are the contract. The
//! material is the committed CKM operational template pack, and every layout
//! authored here is synthetic content invented for the test.

use std::error::Error;

use ferrochart_form::text::Localized;
use ferrochart_overlay::advice::Advisory;
use ferrochart_overlay::error::OverlayError;
use ferrochart_overlay::layout::{
    CharacterWidth, ColumnCount, ColumnSpan, Geometry, HintName, Layout, Section, SectionId,
};
use ferrochart_overlay::replay::replay;
use ferrochart_overlay::store::{Author, Overlay};

use crate::support::{form, keys};

fn columns(count: u8) -> ColumnCount {
    ColumnCount::try_from(count).unwrap()
}

fn span(count: u8) -> ColumnSpan {
    ColumnSpan::try_from(count).unwrap()
}

fn wide(count: u8) -> Section {
    Section::new(SectionId::new("wide"), Localized::empty()).on_columns(columns(count))
}

fn in_wide(geometry: Geometry) -> Layout {
    Layout {
        section: Some(SectionId::new("wide")),
        geometry,
        ..Layout::new()
    }
}

#[test]
fn a_section_that_declares_nothing_is_one_column() {
    let section = Section::new(SectionId::new("counts"), Localized::empty());
    assert_eq!(section.columns, ColumnCount::ONE);
    assert_eq!(section.columns.get(), 1);
    assert!(!section.columns.is_crowded());
}

#[test]
fn a_column_count_outside_one_to_twelve_is_not_representable() {
    assert!(ColumnCount::try_from(0).is_err());
    assert!(ColumnCount::try_from(13).is_err());
    assert_eq!(columns(1), ColumnCount::ONE);
    assert_eq!(columns(12).get(), 12);
    let refused = ColumnCount::try_from(13).unwrap_err();
    assert!(refused.to_string().contains("1 to 12 columns"), "{refused}");
    assert!(ColumnSpan::try_from(0).is_err());
    assert!(ColumnSpan::try_from(13).is_err());
    assert!(CharacterWidth::try_from(0).is_err());
}

#[test]
fn a_document_stating_a_column_count_the_grid_does_not_admit_is_refused() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    author.add_section(wide(3)).unwrap();
    let mut document = document_of(&author.finish());
    document["sections"][0]["columns"] = serde_json::json!(13);
    let refused = read(&document).unwrap_err();
    let cause = refused.source().expect("a refusal carries its cause");
    assert!(
        cause.to_string().contains("1 to 12 columns"),
        "{refused}: {cause}"
    );
}

#[test]
fn a_section_wider_than_four_columns_is_stored_and_advised_against() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    assert!(
        author.add_section(wide(4)).unwrap().is_empty(),
        "four columns is inside what the store accepts quietly"
    );
    let advised = author.add_section(wide(5)).unwrap();
    assert_eq!(
        advised,
        vec![Advisory::CrowdedSection {
            section: SectionId::new("wide"),
            columns: columns(5),
        }]
    );
    let overlay = author.finish();
    assert_eq!(
        overlay.section(&SectionId::new("wide")).unwrap().columns,
        columns(5),
        "the width is stored, because it is advice and not a refusal"
    );
    assert!(
        advised[0]
            .to_string()
            .contains("skipped and misinterpreted"),
        "{}",
        advised[0]
    );
}

#[test]
fn an_item_that_asks_for_nothing_takes_its_sections_full_width() {
    let geometry = Geometry::default();
    assert!(geometry.is_empty());
    assert_eq!(geometry.span, None);
    assert_eq!(geometry.span_in(ColumnCount::ONE), ColumnSpan::ONE);
    assert_eq!(geometry.span_in(columns(4)).get(), 4);
}

#[test]
fn a_stated_span_is_clamped_by_a_narrower_grid_and_the_overlay_keeps_it() {
    let geometry = Geometry {
        span: Some(span(6)),
        ..Geometry::default()
    };
    assert_eq!(geometry.span_in(columns(12)).get(), 6);
    assert_eq!(geometry.span_in(columns(3)).get(), 3);
    assert_eq!(
        geometry.span,
        Some(span(6)),
        "clamping is the renderer's, so nothing stored is discarded"
    );
}

#[test]
fn a_span_wider_than_its_section_is_refused_naming_both_numbers() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    author.add_section(wide(3)).unwrap();
    let layout = in_wide(Geometry {
        span: Some(span(6)),
        ..Geometry::default()
    });
    let refused = author.set(&key, layout).unwrap_err();
    assert!(
        matches!(
            refused,
            OverlayError::SpanExceedsColumns {
                span: 6,
                columns: 3,
                ..
            }
        ),
        "{refused}"
    );
    assert!(
        refused.to_string().contains("spans 6 columns of section")
            && refused.to_string().contains("which is 3 wide"),
        "{refused}"
    );
}

#[test]
fn an_item_in_no_authored_section_has_one_column_to_span() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let layout = Layout {
        geometry: Geometry {
            span: Some(span(2)),
            ..Geometry::default()
        },
        ..Layout::new()
    };
    let refused = author.set(&key, layout).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("the form itself, which is 1 wide"),
        "{refused}"
    );
    let fits = Layout {
        geometry: Geometry {
            span: Some(ColumnSpan::ONE),
            break_before: true,
            ..Geometry::default()
        },
        ..Layout::new()
    };
    let _ = author.set(&key, fits).unwrap();
}

#[test]
fn a_span_a_break_and_a_width_hint_round_trip() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    author.add_section(wide(12)).unwrap();
    let geometry = Geometry {
        span: Some(span(6)),
        break_before: true,
        width: Some(CharacterWidth::try_from(3).unwrap()),
    };
    let _ = author.set(&key, in_wide(geometry)).unwrap();
    let overlay = author.finish();

    let once = overlay.to_json().unwrap();
    assert_eq!(once, overlay.to_json().unwrap());
    let parsed = Overlay::from_json(&once).unwrap();
    assert_eq!(parsed, overlay);
    assert_eq!(parsed.entry(&key).unwrap().layout.geometry, geometry);

    let document = document_of(&overlay);
    let stored = &document["entries"][0]["layout"]["geometry"];
    let members: Vec<&String> = stored.as_object().unwrap().keys().collect();
    assert_eq!(
        members,
        ["span", "break_before", "width"],
        "the grid stores no row and no column index"
    );
    assert_eq!(stored["width"], serde_json::json!(3), "in character units");
}

#[test]
fn the_hint_map_round_trips_in_name_order_and_nothing_reads_it() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    let mut layout = Layout::new();
    layout
        .hints
        .insert(HintName::new("wound-canvas"), "x=12,y=40".to_owned());
    layout
        .hints
        .insert(HintName::new("accent"), "rose".to_owned());
    let _ = author.set(&key, layout.clone()).unwrap();
    let overlay = author.finish();
    let parsed = Overlay::from_json(&overlay.to_json().unwrap()).unwrap();
    assert_eq!(parsed.entry(&key).unwrap().layout.hints, layout.hints);

    let document = document_of(&overlay);
    let names: Vec<&String> = document["entries"][0]["layout"]["hints"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(names, ["accent", "wound-canvas"]);

    let report = replay(&overlay, &definition);
    assert_eq!(
        report.summary().geometry,
        0,
        "a hint is outside the clean-replay guarantee, so the replay says nothing about it"
    );
}

#[test]
fn the_replay_reports_a_span_that_no_longer_fits_the_section_it_lands_in() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    author.add_section(wide(12)).unwrap();
    let _ = author
        .set(
            &key,
            in_wide(Geometry {
                span: Some(span(6)),
                ..Geometry::default()
            }),
        )
        .unwrap();
    // The person changes their mind about the section, which the store takes
    // and reports rather than refusing or rewriting the span.
    let advised = author.add_section(wide(3)).unwrap();
    assert_eq!(
        advised,
        vec![Advisory::SpanOverflows {
            key: key.clone(),
            span: span(6),
            section: SectionId::new("wide"),
            columns: columns(3),
        }]
    );
    let overlay = author.finish();

    let report = replay(&overlay, &definition);
    assert_eq!(report.summary().geometry, 1);
    assert_eq!(report.entries()[0].advisories, advised);
    let text = report.to_string();
    assert!(
        text.contains("Geometry to put right (1)")
            && text.contains("spans 6 columns")
            && text.contains("is 3 wide"),
        "{text}"
    );
}

#[test]
fn the_replay_reports_an_entry_whose_section_disappeared() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    author.add_section(wide(12)).unwrap();
    let _ = author
        .set(
            &key,
            in_wide(Geometry {
                span: Some(span(6)),
                ..Geometry::default()
            }),
        )
        .unwrap();
    let mut overlay = author.finish();
    let dropped = overlay
        .remove_section(&SectionId::new("wide"))
        .unwrap()
        .expect("the section was there");
    assert_eq!(dropped.columns, columns(12));

    let report = replay(&overlay, &definition);
    assert_eq!(
        report.entries()[0].advisories,
        vec![Advisory::SectionDisappeared {
            key: key.clone(),
            section: SectionId::new("wide"),
        }],
        "the grouping is the whole report: a section that is gone has no width to overflow"
    );
    assert!(report.to_string().contains("no longer defines"), "{report}");
    assert_eq!(
        overlay.entry(&key).unwrap().layout.section,
        Some(SectionId::new("wide")),
        "the entry keeps its grouping, so restoring the section restores it"
    );
}

#[test]
fn an_overlay_whose_section_was_dropped_still_reads_back() {
    let definition = form("aedes-indices-jm.opt");
    let key = keys(&definition).pop().unwrap();
    let mut author = Author::new(&definition);
    author.add_section(wide(3)).unwrap();
    let _ = author.set(&key, in_wide(Geometry::default())).unwrap();
    let mut overlay = author.finish();
    overlay.remove_section(&SectionId::new("wide")).unwrap();
    let text = overlay.to_json().unwrap();
    assert_eq!(Overlay::from_json(&text).unwrap(), overlay);
}

#[test]
fn a_section_another_section_sits_in_is_not_dropped_on_its_own() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    author.add_section(wide(3)).unwrap();
    let mut inner = Section::new(SectionId::new("inner"), Localized::empty());
    inner.parent = Some(SectionId::new("wide"));
    author.add_section(inner).unwrap();
    let mut overlay = author.finish();
    let refused = overlay.remove_section(&SectionId::new("wide")).unwrap_err();
    assert!(
        refused.to_string().contains("holds section inner"),
        "{refused}"
    );
}

#[test]
fn a_section_carries_its_column_count_and_nothing_else_about_geometry() {
    let definition = form("aedes-indices-jm.opt");
    let mut author = Author::new(&definition);
    author.add_section(wide(2)).unwrap();
    let document = document_of(&author.finish());
    let members: Vec<&String> = document["sections"][0]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(
        members,
        ["id", "parent", "label", "help", "order", "columns"]
    );
    assert_eq!(document["sections"][0]["columns"], serde_json::json!(2));
}

/// One overlay, as the JSON it writes.
fn document_of(overlay: &Overlay) -> serde_json::Value {
    serde_json::from_str(&overlay.to_json().unwrap()).unwrap()
}

/// One JSON document, read back as an overlay.
fn read(document: &serde_json::Value) -> Result<Overlay, OverlayError> {
    Overlay::from_json(&document.to_string())
}
