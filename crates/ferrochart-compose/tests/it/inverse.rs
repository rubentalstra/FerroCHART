// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The two directions are inverses, over the whole committed template pack.
//!
//! Synthetic content invented for these tests. No patient data.
//!
//! The property asserted is the **form-level inverse**
//! (`docs/architecture.md` section 5.4): a value entered against a field comes
//! back against that same field, unchanged. No openEHR specification
//! guarantees a lossless round trip through a CDR, so byte equality is not
//! asserted and never will be; what a form put in is what a form gets out, and
//! that is the claim the specifications support.

use std::collections::BTreeMap;

use ferrochart_compose::{build, read};
use ferrochart_form::field::{FieldKind, FormField};
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LocalCode;
use ferrochart_form::values::{Datum, Entered, FormValues};

use crate::corpus::{envelope, forms};
use crate::filler;

/// The value entered against every field, and the value that came back.
struct Trip {
    entered: FormValues,
    returned: FormValues,
    uncovered: Vec<String>,
    ambiguous: Vec<String>,
}

/// Builds a composition from `values` and reads it straight back.
fn round_trip(form: &ferrochart_form::definition::FormDefinition, values: FormValues) -> Trip {
    let composition =
        build::composition(form, &values, &envelope()).expect("the form builds a composition");
    let back = read::values(form, &composition).expect("the composition reads back");
    Trip {
        entered: values,
        returned: back.values,
        uncovered: back.uncovered,
        ambiguous: back.ambiguous,
    }
}

#[test]
fn every_value_entered_comes_back_against_its_own_field() {
    // The property the whole issue is about. A value that lands on a
    // different field on the way back is worse than one that is lost: it is a
    // clinical value attributed to the wrong question.
    let mut checked = 0;
    let mut fields = 0;
    let mut ambiguous = 0;
    for (path, form) in forms() {
        let values = filler::fill(&form);
        if values.is_empty() {
            continue;
        }
        if build::composition(&form, &values, &envelope()).is_err() {
            continue;
        }
        let trip = round_trip(&form, values);
        // Where the reader had to place a tied sibling by position, the
        // assignment is a guess it reported as one, and this template's
        // values are not evidence either way.
        if !trip.ambiguous.is_empty() {
            ambiguous += 1;
            continue;
        }
        checked += 1;
        for (slot, entered) in trip.entered.iter() {
            let returned = trip
                .returned
                .get_in(&slot.key, &slot.group_path, slot.occurrence);
            assert_eq!(
                returned,
                Some(entered),
                "{}: the value at {} group path {:?} occurrence {} did not come back",
                path.display(),
                slot.key,
                slot.group_path,
                slot.occurrence
            );
            fields += 1;
        }
    }
    assert!(
        checked >= 60,
        "only {checked} templates round-tripped exactly"
    );
    assert!(fields >= 1000, "only {fields} values round-tripped");
    // A ratchet on the honest number: how many templates carry a tie the data
    // could not fill, where a reader can only guess.
    assert!(
        ambiguous <= 18,
        "{ambiguous} templates resolved a tie by position, which is more than \
         the pack used to need"
    );
}

#[test]
fn the_round_trip_adds_nothing_the_form_did_not_enter() {
    // The other half of an inverse. A reader that invented a value would pass
    // the test above and still be wrong.
    for (path, form) in forms() {
        let values = filler::fill(&form);
        if values.is_empty() || build::composition(&form, &values, &envelope()).is_err() {
            continue;
        }
        let trip = round_trip(&form, values);
        if !trip.ambiguous.is_empty() {
            continue;
        }
        assert_eq!(
            trip.returned.len(),
            trip.entered.len(),
            "{}: {} values went in and {} came back",
            path.display(),
            trip.entered.len(),
            trip.returned.len()
        );
    }
}

#[test]
fn nothing_the_builder_wrote_reads_back_as_uncovered() {
    // A composition FerroCHART built from this very form is covered by it by
    // construction. An uncovered path here means the reader and the builder
    // disagree about the tree, which is the drift the pair exists to catch.
    let mut reported: BTreeMap<String, usize> = BTreeMap::new();
    for (_, form) in forms() {
        let values = filler::fill(&form);
        if values.is_empty() || build::composition(&form, &values, &envelope()).is_err() {
            continue;
        }
        for path in round_trip(&form, values).uncovered {
            *reported.entry(path).or_default() += 1;
        }
    }
    assert!(
        reported.is_empty(),
        "the reader could not place {} paths the builder wrote: {:?}",
        reported.len(),
        reported.keys().take(5).collect::<Vec<_>>()
    );
}

#[test]
fn a_null_flavour_survives_both_directions() {
    // An acceptance criterion of its own. A null flavour is a clinical
    // statement, not an absence: "not applicable" and "unknown" mean
    // different things, and losing either loses the statement.
    let Some((_, form)) = forms().into_iter().find(|(_, form)| {
        !filler::fill(form).is_empty()
            && build::composition(form, &filler::fill(form), &envelope()).is_ok()
            && form.fields().count() > 1
    }) else {
        panic!("no template in the pack builds a composition");
    };

    let mut values = filler::fill(&form);
    let (key, group_path, occurrence) = {
        let (slot, _) = values.iter().next().expect("the filler entered a value");
        (slot.key.clone(), slot.group_path.clone(), slot.occurrence)
    };
    values.set_in(
        key.clone(),
        group_path.clone(),
        occurrence,
        Entered::Null {
            code: LocalCode::new("273"),
            reason: Some("a synthetic reason".to_owned()),
        },
    );

    let trip = round_trip(&form, values);
    let back = trip
        .returned
        .get_in(&key, &group_path, occurrence)
        .expect("the null-flavoured field came back");
    match *back {
        Entered::Null {
            ref code,
            ref reason,
        } => {
            assert_eq!(code.as_str(), "273");
            assert_eq!(reason.as_deref(), Some("a synthetic reason"));
        }
        Entered::Value(_) => panic!("a null flavour came back as a value"),
    }
}

#[test]
fn content_the_form_does_not_cover_is_reported_rather_than_dropped() {
    // A composition can carry content this form has no field for, because the
    // template was revised or another system wrote it. An editing round trip
    // that dropped it silently would delete a colleague's data, so the reader
    // reports every node it cannot place.
    let Some((_, form)) = forms().into_iter().find(|(_, form)| {
        !filler::fill(form).is_empty()
            && build::composition(form, &filler::fill(form), &envelope()).is_ok()
    }) else {
        panic!("no template in the pack builds a composition");
    };

    let mut composition = build::composition(&form, &filler::fill(&form), &envelope())
        .expect("the form builds a composition");

    // A node id no template of this pack uses, so the form cannot cover it.
    let content = composition
        .content
        .as_mut()
        .expect("the composition carries content");
    let mut stranger = content.head().clone();
    rename(&mut stranger, "at9999");
    let mut items: Vec<_> = content.iter().cloned().collect();
    items.push(stranger);
    composition.content = Some(items.try_into().expect("the content is not empty"));

    let back = read::values(&form, &composition).expect("the composition reads back");
    assert_eq!(
        back.uncovered.len(),
        1,
        "the node the form does not cover was not reported: {:?}",
        back.uncovered
    );
}

/// Gives a content item a node id nothing in the form matches.
fn rename(item: &mut openehr_rm::v1_2::composition::content::content_item::ContentItem, id: &str) {
    use openehr_rm::v1_2::composition::content::content_item::ContentItem;
    let target = match *item {
        ContentItem::Section(ref mut it) => &mut it.archetype_node_id,
        ContentItem::Observation(ref mut it) => &mut it.archetype_node_id,
        ContentItem::Evaluation(ref mut it) => &mut it.archetype_node_id,
        ContentItem::AdminEntry(ref mut it) => &mut it.archetype_node_id,
        ContentItem::Instruction(ref mut it) => &mut it.archetype_node_id,
        ContentItem::Action(ref mut it) => &mut it.archetype_node_id,
        ContentItem::GenericEntry(_) => return,
    };
    id.clone_into(target);
}

#[test]
fn a_composition_from_another_template_is_refused() {
    // Reading a document built from a different template into this form would
    // land values wherever the node ids happened to collide.
    let mut forms = forms().into_iter();
    let Some((_, first)) = forms.next() else {
        return;
    };
    let Some((_, second)) = forms.find(|(_, form)| {
        form.template_id != first.template_id
            && !filler::fill(form).is_empty()
            && build::composition(form, &filler::fill(form), &envelope()).is_ok()
    }) else {
        return;
    };

    let composition = build::composition(&second, &filler::fill(&second), &envelope())
        .expect("the second form builds a composition");
    match read::values(&first, &composition) {
        Err(ferrochart_compose::error::ReadError::WrongTemplate { expected, found }) => {
            assert_eq!(expected, first.template_id.as_str());
            assert_eq!(found, second.template_id.as_str());
        }
        Ok(_) => panic!("a composition from another template was read into this form"),
        Err(other) => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn a_repeated_field_reads_back_into_the_right_occurrence() {
    // A repeatable node produces several data nodes sharing one
    // `archetype_node_id` (openEHR BASE Release-1.2.0
    // `architecture_overview.html` section 10.4). Reading them back into the
    // wrong occurrence would reorder a clinician's entries.
    let Some((_, form)) = forms().into_iter().find(|(_, form)| {
        !filler::fill(form).is_empty()
            && build::composition(form, &filler::fill(form), &envelope()).is_ok()
            && form
                .fields()
                .any(|field| field.occurrences.maximum != Some(1))
    }) else {
        return;
    };

    let repeatable = form
        .fields()
        .find(|field| field.occurrences.maximum != Some(1))
        .expect("the form has a repeatable field");
    let mut values = filler::fill(&form);
    // The occurrence path the filler reached that field by, so the repeats
    // land in one instance of whatever repeating group holds it.
    let group_path = values
        .iter()
        .find(|(slot, _)| slot.key == repeatable.key)
        .map_or_else(Vec::new, |(slot, _)| slot.group_path.clone());
    // Three distinct values in a known order, so a reordering is visible.
    for (index, text) in ["first", "second", "third"].iter().enumerate() {
        values.set_in(
            repeatable.key.clone(),
            group_path.clone(),
            index,
            Entered::Value(Datum::Text((*text).to_owned())),
        );
    }

    let Ok(composition) = build::composition(&form, &values, &envelope()) else {
        return;
    };
    let back = read::values(&form, &composition).expect("the composition reads back");
    for (index, text) in ["first", "second", "third"].iter().enumerate() {
        match back.values.get_in(&repeatable.key, &group_path, index) {
            Some(&Entered::Value(Datum::Text(ref returned))) => assert_eq!(returned, text),
            other => panic!("occurrence {index} came back as {other:?}"),
        }
    }
}

/// The committed template the nested-repeat case is measured on.
///
/// It carries exactly one repeating `CLUSTER` inside another repeating
/// `CLUSTER`, so the pair the test needs is the only one in the tree and
/// nothing else in the document can account for the nodes it counts.
const NESTED_REPEAT: &str = "family-history-summary-item-r2.opt";

#[test]
fn a_repeat_inside_a_repeat_keeps_all_four_instances_apart() {
    // The case a flat slot cannot address. Two occurrences of an outer group,
    // each carrying two of an inner group, are four places one field holds
    // four different clinical statements, and openEHR BASE Release-1.2.0
    // `architecture_overview.html` section 10.4 says all four share one
    // `archetype_node_id`: "a single archetype node may be replicated in the
    // data".
    let Some((_, form)) = forms()
        .into_iter()
        .find(|(path, _)| path.file_name().is_some_and(|name| name == NESTED_REPEAT))
    else {
        panic!("the committed pack no longer carries {NESTED_REPEAT}");
    };
    let Some(field) = nested_repeat_field(&form.root) else {
        panic!("{NESTED_REPEAT} no longer carries a repeat inside a repeat");
    };

    let mut values = filler::fill(&form);
    let paths: Vec<Vec<usize>> = values
        .iter()
        .filter(|(slot, _)| slot.key == field.key)
        .map(|(slot, _)| slot.group_path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![vec![0, 0], vec![0, 1], vec![1, 0], vec![1, 1]],
        "the field sits in one instance of each pair of repeats"
    );

    // A different value in each of the four, so a slot that lost its place
    // comes back as the wrong text rather than as a missing one.
    for (index, path) in paths.iter().enumerate() {
        values.set_in(
            field.key.clone(),
            path.clone(),
            0,
            Entered::Value(Datum::Text(format!("statement {index}"))),
        );
    }

    let composition =
        build::composition(&form, &values, &envelope()).expect("the form builds a composition");
    let json = serde_json::to_string(&composition).expect("a composition serialises");
    for index in 0..paths.len() {
        assert_eq!(
            json.matches(&format!("\"statement {index}\"")).count(),
            1,
            "the value entered in instance {index} is not in exactly one data node"
        );
    }

    let back = read::values(&form, &composition).expect("the composition reads back");
    assert!(back.uncovered.is_empty(), "{:?}", back.uncovered);
    assert!(back.ambiguous.is_empty(), "{:?}", back.ambiguous);
    for (index, path) in paths.iter().enumerate() {
        match back.values.get_in(&field.key, path, 0) {
            Some(&Entered::Value(Datum::Text(ref returned))) => {
                assert_eq!(*returned, format!("statement {index}"));
            }
            other => panic!("the value at {path:?} came back as {other:?}"),
        }
    }
}

/// A text field inside a repeating group that is itself inside a repeating
/// group.
fn nested_repeat_field(group: &FormGroup) -> Option<&FormField> {
    for item in &group.items {
        let FormItem::Group(ref child) = *item else {
            continue;
        };
        if child.occurrences.is_repeatable() {
            let found = child.items.iter().find_map(|item| match *item {
                FormItem::Group(ref inner) if inner.occurrences.is_repeatable() => {
                    inner.items.iter().find_map(|item| match *item {
                        FormItem::Field(ref field) if matches!(field.kind, FieldKind::Text(_)) => {
                            Some(&**field)
                        }
                        _ => None,
                    })
                }
                _ => None,
            });
            if found.is_some() {
                return found;
            }
        }
        if let Some(found) = nested_repeat_field(child) {
            return Some(found);
        }
    }
    None
}
