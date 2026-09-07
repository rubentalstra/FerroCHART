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

use ferrochart_compose::values::{Datum, FormValues};
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::{FieldKind, FormField};
use ferrochart_form::value::ValueSet;

/// Fills every field of `definition` with a value its template permits.
///
/// A field whose kind carries no permitted value at all is left empty, so the
/// builder sees the same shape a clinician would leave behind.
pub(crate) fn fill(definition: &FormDefinition) -> FormValues {
    let mut values = FormValues::new();
    for field in definition.fields() {
        if let Some(datum) = datum_for(field) {
            values.set(
                field.key.clone(),
                ferrochart_compose::values::Entered::Value(datum),
            );
        }
    }
    values
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
        FieldKind::Duration(_) => Some(Datum::Duration("PT1H".to_owned())),
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
