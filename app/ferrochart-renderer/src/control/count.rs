// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_COUNT`: a whole number, from a list or inside a range.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 6.2.9. The template
//! states the permitted values as an enumeration, as ranges, or as neither,
//! and a control offers exactly what it states.

use ferrochart_form::field::CountField;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, within_any};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{INPUT, SELECT};

/// The value `typed` records, where the constraint admits it.
pub(crate) fn admit(field: &CountField, typed: &str) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    let value: i64 = typed.trim().parse().map_err(|_| Refusal::NotANumber)?;
    if !field.options.is_empty() && !field.options.contains(&value) {
        return Err(Refusal::NotEnumerated);
    }
    if !within_any(&field.ranges, &value) {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::Count(value))
}

/// The lowest and highest value the ranges admit, for the input's own bounds.
///
/// Only where the template states exactly one range: two ranges are
/// alternatives with a hole between them, and `min` and `max` would close the
/// hole. The admission function is what decides either way.
pub(crate) fn bounds(field: &CountField) -> (Option<i64>, Option<i64>) {
    match field.ranges.as_slice() {
        [only] => {
            let low = only.minimum.map(|value| {
                if only.minimum_included {
                    value
                } else {
                    value.saturating_add(1)
                }
            });
            let high = only.maximum.map(|value| {
                if only.maximum_included {
                    value
                } else {
                    value.saturating_sub(1)
                }
            });
            (low, high)
        }
        _ => (None, None),
    }
}

/// What is in the slot, as the input shows it.
fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Count(value)) => value.to_string(),
        _ => String::new(),
    }
}

/// A control over a whole number.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn CountControl(
    /// What the template admits.
    field: CountField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let held = {
        let slot = slot.clone();
        move || shown(&slot)
    };
    let on_change = {
        let slot = slot.clone();
        let field = field.clone();
        move |event: leptos::ev::Event| {
            slot.apply(admit(&field, &event_target_value(&event)), refused);
        }
    };

    let control = if field.options.is_empty() {
        let (low, high) = bounds(&field);
        view! {
            <input
                id=slot.id.clone()
                class=INPUT
                type="number"
                step="1"
                min=low.map(|value| value.to_string())
                max=high.map(|value| value.to_string())
                prop:value=held
                on:input=on_change
            />
        }
        .into_any()
    } else {
        let choices: Vec<_> = field
            .options
            .iter()
            .filter(|value| within_any(&field.ranges, value))
            .map(|value| view! { <option value=value.to_string()>{value.to_string()}</option> })
            .collect();
        view! {
            <select id=slot.id.clone() class=SELECT prop:value=held on:change=on_change>
                <option value="">"Not entered"</option>
                {choices}
            </select>
        }
        .into_any()
    };

    view! {
        {control}
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::CountField;
    use ferrochart_form::range::Range;
    use ferrochart_form::values::Datum;

    use super::{admit, bounds};
    use crate::control::admit::Refusal;

    #[test]
    fn a_range_refuses_a_value_outside_it() {
        let field = CountField {
            options: Vec::new(),
            ranges: vec![Range::new(Some(0), Some(10), true, true)],
        };
        assert_eq!(admit(&field, "10"), Ok(Datum::Count(10)));
        assert_eq!(admit(&field, "11"), Err(Refusal::OutsideRange));
        assert_eq!(bounds(&field), (Some(0), Some(10)));
    }

    #[test]
    fn an_excluded_end_moves_the_input_s_own_bound_by_one() {
        let field = CountField {
            options: Vec::new(),
            ranges: vec![Range::new(Some(0), Some(10), false, false)],
        };
        assert_eq!(bounds(&field), (Some(1), Some(9)));
        assert_eq!(admit(&field, "0"), Err(Refusal::OutsideRange));
        assert_eq!(admit(&field, "1"), Ok(Datum::Count(1)));
    }

    #[test]
    fn two_ranges_leave_the_input_unbounded_and_the_hole_still_refuses() {
        let field = CountField {
            options: Vec::new(),
            ranges: vec![
                Range::new(Some(0), Some(2), true, true),
                Range::new(Some(8), Some(9), true, true),
            ],
        };
        assert_eq!(bounds(&field), (None, None));
        assert_eq!(admit(&field, "5"), Err(Refusal::OutsideRange));
        assert_eq!(admit(&field, "8"), Ok(Datum::Count(8)));
    }

    #[test]
    fn an_enumeration_refuses_a_value_it_does_not_list() {
        let field = CountField {
            options: vec![1, 3, 5],
            ranges: Vec::new(),
        };
        assert_eq!(admit(&field, "3"), Ok(Datum::Count(3)));
        assert_eq!(admit(&field, "4"), Err(Refusal::NotEnumerated));
    }

    #[test]
    fn text_that_is_not_a_whole_number_is_refused() {
        let field = CountField::default();
        assert_eq!(admit(&field, "two"), Err(Refusal::NotANumber));
        assert_eq!(admit(&field, "1.5"), Err(Refusal::NotANumber));
        assert_eq!(admit(&field, ""), Err(Refusal::Empty));
    }
}
