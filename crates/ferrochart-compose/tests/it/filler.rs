// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A value for every field of a form, chosen from what the template permits.
//!
//! Synthetic content invented for these tests. No patient data: every value
//! here is either a code the template itself enumerates, or a number inside
//! the range the template states, or an invented string that names itself.
//!
//! The filler is deliberately strict about the template's own constraints,
//! because a value the template refuses would test the wrong thing: the point
//! of the inverse test is that a legitimate entry survives, not that an
//! illegitimate one is caught.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::{DurationComponent, FieldKind, FormField};
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::value::ValueSet;
use ferrochart_form::values::{Datum, FormValues};

/// How many occurrences of a repeating group the filler enters.
///
/// More than one, so a builder that ignored the occurrence path would drop a
/// value the round trip then reports as lost.
const REPEATS: usize = 2;

/// Fills every field of `definition` with a value its template permits.
///
/// A field whose kind carries no permitted value at all is left empty, so the
/// builder sees the same shape a clinician would leave behind.
pub(crate) fn fill(definition: &FormDefinition) -> FormValues {
    let mut values = FormValues::new();
    fill_group(&definition.root, &[], &mut values);
    values
}

/// Fills one group, once per occurrence path its repeating ancestors name.
fn fill_group(group: &FormGroup, path: &[usize], values: &mut FormValues) {
    for item in &group.items {
        match *item {
            FormItem::Field(ref field) => {
                if let Some(datum) = datum_for(field) {
                    values.set_in(
                        field.key.clone(),
                        path.to_vec(),
                        0,
                        ferrochart_form::values::Entered::Value(datum),
                    );
                }
            }
            FormItem::Group(ref child) => {
                for inner in instances(child, path) {
                    fill_group(child, &inner, values);
                }
            }
            _ => {}
        }
    }
}

/// The occurrence paths one child group is filled under.
fn instances(group: &FormGroup, path: &[usize]) -> Vec<Vec<usize>> {
    if !group.occurrences.is_repeatable() {
        return vec![path.to_vec()];
    }
    (0..REPEATS)
        .map(|index| {
            let mut grown = path.to_vec();
            grown.push(index);
            grown
        })
        .collect()
}

/// A value the field's own constraint admits, where one can be chosen.
fn datum_for(field: &FormField) -> Option<Datum> {
    match field.kind {
        FieldKind::Boolean(_) => Some(Datum::Boolean(true)),
        FieldKind::Text(ref it) => Some(Datum::Text(
            it.options
                .first()
                .cloned()
                // A pattern-constrained text needs a matching string, and
                // generating one is a research problem of its own, so a
                // pattern-only field is skipped rather than guessed at.
                .or_else(|| {
                    it.patterns
                        .is_empty()
                        .then(|| "a synthetic entry".to_owned())
                })?,
        )),
        FieldKind::Uri(_) => Some(Datum::Uri("https://ferro.example/synthetic".to_owned())),
        FieldKind::Coded(ref it) => coded(&it.value_set),
        FieldKind::Ordinal(ref it) => {
            let option = it.options.first()?;
            Some(Datum::Ordinal {
                terminology: option.symbol.terminology.as_str().to_owned(),
                code: option.symbol.code.clone(),
                rubric: "a synthetic symbol".to_owned(),
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "an ordinal score is a whole number the template states"
                )]
                value: option.score as i64,
            })
        }
        FieldKind::Count(ref it) => {
            let value = it.options.first().copied().or_else(|| {
                it.ranges
                    .first()
                    .and_then(|range| range.minimum)
                    .or(Some(1))
            })?;
            Some(Datum::Count(value))
        }
        FieldKind::Quantity(ref it) => {
            let unit = it.units.first()?;
            let magnitude = unit
                .magnitude
                .as_ref()
                .and_then(|range| range.minimum)
                .unwrap_or(1.0);
            Some(Datum::Quantity {
                magnitude,
                units: unit.units.clone(),
                precision: None,
            })
        }
        FieldKind::Date(_) => Some(Datum::Date("2026-09-07".to_owned())),
        FieldKind::Time(_) => Some(Datum::Time("12:00:00".to_owned())),
        FieldKind::DateTime(_) => Some(Datum::DateTime("2026-09-07T12:00:00Z".to_owned())),
        FieldKind::Duration(ref it) => {
            // A duration the template refuses is not a value a form would
            // admit, so the components decide the designator and a
            // range-constrained field is skipped rather than guessed at.
            // openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.9 gives the
            // pattern its slots.
            // A range's own lower bound is a value the template author wrote
            // against the same pattern, so it satisfies both constraints
            // where the range states one and is included.
            if let Some(stated) = it.ranges.iter().find_map(|range| {
                range
                    .minimum_included
                    .then(|| range.minimum.clone())
                    .flatten()
            }) {
                return Some(Datum::Duration(stated));
            }
            if !it.ranges.is_empty() {
                return None;
            }
            // `DurationComponent` is #[non_exhaustive], so a slot added
            // later falls back to the hour rather than failing the build.
            let designator =
                it.components
                    .iter()
                    .next()
                    .map_or("PT1H", |component| match *component {
                        DurationComponent::Years => "P1Y",
                        DurationComponent::Months => "P1M",
                        DurationComponent::Weeks => "P1W",
                        DurationComponent::Days => "P1D",
                        DurationComponent::Minutes => "PT1M",
                        DurationComponent::Seconds => "PT1S",
                        _ => "PT1H",
                    });
            Some(Datum::Duration(designator.to_owned()))
        }
        FieldKind::Identifier(_) => Some(Datum::Identifier {
            id: "SYN-0001".to_owned(),
            issuer: None,
            assigner: None,
            identifier_type: None,
        }),
        FieldKind::Parsable(_) => Some(Datum::Parsable {
            value: "a synthetic expression".to_owned(),
            formalism: "text/plain".to_owned(),
        }),
        // A proportion's permitted kinds constrain its denominator, and
        // picking a numerator that satisfies every invariant for every kind is
        // its own problem, so the datum tests cover that class directly. The
        // rest need a shape the value model does not carry yet, or a choice
        // between alternatives that belongs to the renderer.
        _ => None,
    }
}

/// A code the value set enumerates, where it enumerates any.
fn coded(value_set: &ValueSet) -> Option<Datum> {
    let ValueSet::Enumerated(ref set) = *value_set else {
        return None;
    };
    let option = set.options.first()?;
    Some(Datum::Coded {
        terminology: option.code.terminology.as_str().to_owned(),
        code: option.code.code.clone(),
        rubric: "a synthetic rubric".to_owned(),
    })
}
