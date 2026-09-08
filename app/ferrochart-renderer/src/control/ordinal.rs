// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_ORDINAL` and `DV_SCALE`: an ordered list of scored symbols.
//!
//! openEHR RM Release-1.1.0 `data_types.html` sections 6.2.4 and 6.2.5. The
//! composition stores the SYMBOL, and the score rides along with it, so a
//! control shows the rubric and records the code. List order is display
//! order, because the specification gives an ordered list and nothing else to
//! order by.
//!
//! `DV_ORDINAL.value` is an integer and `DV_SCALE.value` is real, so the
//! field's Reference Model class decides which datum a chosen symbol becomes.

use ferrochart_form::field::OrdinalField;
use ferrochart_form::ids::{LanguageTag, RmTypeName};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::control::{RefusalNote, Slot, localized};
use crate::kit::field::SELECT;

/// The Reference Model class whose score is real rather than integral.
const SCALE: &str = "DV_SCALE";

/// Whether the field collects a `DV_SCALE` rather than a `DV_ORDINAL`.
pub(crate) fn is_scale(rm_type: &RmTypeName) -> bool {
    rm_type.as_str() == SCALE
}

/// The score as the integer a `DV_ORDINAL` stores, where it is one.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is checked whole and inside the exactly-representable range first"
)]
pub(crate) fn whole(score: f64) -> Option<i64> {
    /// The largest magnitude an `f64` represents every integer up to.
    const EXACT: f64 = 9_007_199_254_740_992.0;
    let rounded = score.round();
    if (score - rounded).abs() > f64::EPSILON || rounded.abs() > EXACT {
        return None;
    }
    Some(rounded as i64)
}

/// The value the chosen symbol records, where the template lists it.
pub(crate) fn admit(
    field: &OrdinalField,
    rm_type: &RmTypeName,
    language: &LanguageTag,
    code: &str,
) -> Result<Datum, Refusal> {
    if code.is_empty() {
        return Err(Refusal::Empty);
    }
    let chosen = field
        .options
        .iter()
        .find(|option| option.symbol.code == code)
        .ok_or(Refusal::NotEnumerated)?;
    let terminology = chosen.symbol.terminology.as_str().to_owned();
    let rubric = localized(&chosen.label, language);
    if is_scale(rm_type) {
        return Ok(Datum::Scale {
            terminology,
            code: code.to_owned(),
            rubric,
            value: chosen.score,
        });
    }
    Ok(Datum::Ordinal {
        terminology,
        code: code.to_owned(),
        rubric,
        value: whole(chosen.score).ok_or(Refusal::NotWhole)?,
    })
}

/// The symbol in the slot, as the control shows it.
fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Ordinal { code, .. } | Datum::Scale { code, .. }) => code,
        _ => String::new(),
    }
}

/// A control over an ordinal or a scale.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn OrdinalControl(
    /// What the template admits.
    field: OrdinalField,
    /// The Reference Model class the field collects.
    rm_type: RmTypeName,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let language = slot.language.clone();
    let choices: Vec<_> = field
        .options
        .iter()
        .map(|option| {
            let rubric = localized(&option.label, &language);
            let shown = if rubric.is_empty() {
                option.symbol.code.clone()
            } else {
                format!("{rubric} ({})", option.score)
            };
            view! { <option value=option.symbol.code.clone()>{shown}</option> }
        })
        .collect();

    let held = {
        let slot = slot.clone();
        move || shown(&slot)
    };
    let on_change = {
        let slot = slot.clone();
        let field = field.clone();
        let rm_type = rm_type.clone();
        move |event: leptos::ev::Event| {
            let code = event_target_value(&event);
            let language = slot.language.clone();
            slot.apply(admit(&field, &rm_type, &language, &code), refused);
        }
    };

    view! {
        <select id=slot.id.clone() class=SELECT prop:value=held on:change=on_change>
            <option value="">"Not entered"</option>
            {choices}
        </select>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{OrdinalField, OrdinalOption};
    use ferrochart_form::ids::{LanguageTag, RmTypeName, local_terminology};
    use ferrochart_form::text::Localized;
    use ferrochart_form::value::Code;
    use ferrochart_form::values::Datum;

    use super::admit;
    use crate::control::admit::Refusal;

    fn english() -> LanguageTag {
        LanguageTag::new("en")
    }

    fn scored(score: f64) -> OrdinalField {
        OrdinalField {
            options: vec![OrdinalOption {
                score,
                symbol: Code::new(local_terminology(), "at0005"),
                label: Localized::in_language(english(), "Mild"),
                description: Localized::empty(),
            }],
        }
    }

    #[test]
    fn an_ordinal_stores_the_symbol_and_carries_the_score() {
        assert_eq!(
            admit(
                &scored(1.0),
                &RmTypeName::new("DV_ORDINAL"),
                &english(),
                "at0005"
            ),
            Ok(Datum::Ordinal {
                terminology: "local".to_owned(),
                code: "at0005".to_owned(),
                rubric: "Mild".to_owned(),
                value: 1,
            })
        );
    }

    #[test]
    fn a_scale_keeps_the_real_score_the_template_states() {
        assert_eq!(
            admit(
                &scored(1.5),
                &RmTypeName::new("DV_SCALE"),
                &english(),
                "at0005"
            ),
            Ok(Datum::Scale {
                terminology: "local".to_owned(),
                code: "at0005".to_owned(),
                rubric: "Mild".to_owned(),
                value: 1.5,
            })
        );
    }

    #[test]
    fn a_symbol_the_template_does_not_list_is_refused() {
        assert_eq!(
            admit(
                &scored(1.0),
                &RmTypeName::new("DV_ORDINAL"),
                &english(),
                "at9999"
            ),
            Err(Refusal::NotEnumerated)
        );
        assert_eq!(
            admit(&scored(1.0), &RmTypeName::new("DV_ORDINAL"), &english(), ""),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn an_ordinal_whose_score_is_not_whole_is_refused_rather_than_rounded() {
        assert_eq!(
            admit(
                &scored(1.5),
                &RmTypeName::new("DV_ORDINAL"),
                &english(),
                "at0005"
            ),
            Err(Refusal::NotWhole)
        );
    }
}
