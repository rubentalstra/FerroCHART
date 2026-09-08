// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The value a template prefills a field with.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.2 keeps a default
//! value distinct from an assumed one: "default values do appear in data,
//! while assumed values don't". Only the former reaches a form, so a prefill
//! is written into the document rather than only shown, and a clinician who
//! types over it replaces it.
//!
//! The field's kind decides how a prefill is read, because the same prefill
//! spells different values under different kinds: `Temporal` is a date under
//! a date field and a duration under a duration field.

use ferrochart_form::field::FieldKind;
use ferrochart_form::ids::RmTypeName;
use ferrochart_form::value::Prefill;
use ferrochart_form::values::Datum;

use crate::control::ordinal::is_scale;

/// The value the prefill records under this kind.
///
/// `None` where the prefill and the kind do not name the same shape, which
/// includes every [`Prefill::Opaque`]: the template stated a value in a form
/// this format does not model, and guessing at it would put something in the
/// document that the template did not say.
pub(crate) fn datum(prefill: &Prefill, kind: &FieldKind, rm_type: &RmTypeName) -> Option<Datum> {
    match (prefill, kind) {
        (Prefill::Boolean(value), FieldKind::Boolean(_)) => Some(Datum::Boolean(*value)),
        (Prefill::Integer(value), FieldKind::Count(_)) => Some(Datum::Count(*value)),
        (Prefill::Text(value), FieldKind::Text(_)) => Some(Datum::Text(value.clone())),
        (Prefill::Text(value), FieldKind::Uri(_)) => Some(Datum::Uri(value.clone())),
        (Prefill::Coded { code, rubric }, FieldKind::Coded(_)) => Some(Datum::Coded {
            terminology: code.terminology.as_str().to_owned(),
            code: code.code.clone(),
            rubric: rubric.clone().unwrap_or_default(),
        }),
        (
            Prefill::Quantity {
                magnitude,
                units,
                precision,
            },
            FieldKind::Quantity(_),
        ) => Some(Datum::Quantity {
            magnitude: *magnitude,
            units: units.clone(),
            precision: *precision,
        }),
        (Prefill::Ordinal { value, symbol }, FieldKind::Ordinal(field)) => {
            let rubric = field
                .options
                .iter()
                .find(|option| option.symbol == *symbol)
                .map(|option| option.label.clone())
                .unwrap_or_default();
            let rubric = rubric
                .languages()
                .next()
                .and_then(|first| rubric.get(first))
                .unwrap_or_default()
                .to_owned();
            let terminology = symbol.terminology.as_str().to_owned();
            let code = symbol.code.clone();
            if is_scale(rm_type) {
                return Some(Datum::Scale {
                    terminology,
                    code,
                    rubric,
                    value: *value,
                });
            }
            Some(Datum::Ordinal {
                terminology,
                code,
                rubric,
                value: crate::control::ordinal::whole(*value)?,
            })
        }
        (Prefill::Temporal(value), FieldKind::Date(_)) => Some(Datum::Date(value.clone())),
        (Prefill::Temporal(value), FieldKind::Time(_)) => Some(Datum::Time(value.clone())),
        (Prefill::Temporal(value), FieldKind::DateTime(_)) => Some(Datum::DateTime(value.clone())),
        (Prefill::Temporal(value), FieldKind::Duration(_)) => Some(Datum::Duration(value.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{
        BooleanField, CountField, DateField, DurationField, FieldKind, OrdinalField, OrdinalOption,
        TextField,
    };
    use ferrochart_form::ids::{LanguageTag, RmTypeName, local_terminology};
    use ferrochart_form::text::Localized;
    use ferrochart_form::value::{Code, Prefill};
    use ferrochart_form::values::Datum;

    use super::datum;

    fn element() -> RmTypeName {
        RmTypeName::new("DV_TEXT")
    }

    #[test]
    fn a_prefill_of_the_field_s_own_shape_becomes_a_datum() {
        assert_eq!(
            datum(
                &Prefill::Boolean(true),
                &FieldKind::Boolean(BooleanField {
                    true_allowed: true,
                    false_allowed: true,
                }),
                &element()
            ),
            Some(Datum::Boolean(true))
        );
        assert_eq!(
            datum(
                &Prefill::Integer(4),
                &FieldKind::Count(CountField::default()),
                &element()
            ),
            Some(Datum::Count(4))
        );
    }

    #[test]
    fn the_kind_decides_what_a_temporal_prefill_spells() {
        let value = Prefill::Temporal("P1D".to_owned());
        assert_eq!(
            datum(
                &value,
                &FieldKind::Duration(DurationField {
                    components: std::collections::BTreeSet::new(),
                    ranges: Vec::new(),
                }),
                &element()
            ),
            Some(Datum::Duration("P1D".to_owned()))
        );
        assert_eq!(
            datum(
                &value,
                &FieldKind::Date(DateField {
                    month: ferrochart_form::field::ComponentValidity::Optional,
                    day: ferrochart_form::field::ComponentValidity::Optional,
                    ranges: Vec::new(),
                }),
                &element()
            ),
            Some(Datum::Date("P1D".to_owned())),
            "the prefill is the template's own text, and the date control judges it"
        );
    }

    #[test]
    fn a_prefill_that_does_not_match_the_kind_seeds_nothing() {
        assert_eq!(
            datum(
                &Prefill::Boolean(true),
                &FieldKind::Text(TextField::default()),
                &element()
            ),
            None
        );
        assert_eq!(
            datum(
                &Prefill::Opaque("something".to_owned()),
                &FieldKind::Text(TextField::default()),
                &element()
            ),
            None,
            "a shape this format does not model is never guessed at"
        );
    }

    #[test]
    fn an_ordinal_prefill_takes_its_rubric_from_the_template_s_own_list() {
        let symbol = Code::new(local_terminology(), "at0005");
        let kind = FieldKind::Ordinal(OrdinalField {
            options: vec![OrdinalOption {
                score: 2.0,
                symbol: symbol.clone(),
                label: Localized::in_language(LanguageTag::new("en"), "Moderate"),
                description: Localized::empty(),
            }],
        });
        assert_eq!(
            datum(
                &Prefill::Ordinal { value: 2.0, symbol },
                &kind,
                &RmTypeName::new("DV_ORDINAL")
            ),
            Some(Datum::Ordinal {
                terminology: "local".to_owned(),
                code: "at0005".to_owned(),
                rubric: "Moderate".to_owned(),
                value: 2,
            })
        );
    }
}
