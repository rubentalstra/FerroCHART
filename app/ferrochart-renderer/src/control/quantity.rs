// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_QUANTITY`: a magnitude in one of the units the template permits.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 6.2.8. Each permitted
//! unit carries its own magnitude interval and its own decimal precision, so
//! changing the unit changes both, and a control that kept one unit's range
//! after a switch would admit a magnitude the template refuses.
//!
//! A template that names no unit at all admits any unit of the property, and
//! constrains neither the magnitude nor the precision.

use ferrochart_form::field::{QuantityField, QuantityUnitOption};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, decimals, within};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{HINT, INPUT, LABEL, SELECT};

/// The unit option the template states for `units`.
pub(crate) fn unit<'a>(field: &'a QuantityField, units: &str) -> Option<&'a QuantityUnitOption> {
    field.units.iter().find(|option| option.units == units)
}

/// The magnitude and decimal bounds the chosen unit carries, as the number
/// input's own attributes.
///
/// Both come from the unit rather than from the field, which is what makes a
/// unit change carry its range and its precision with it.
pub(crate) fn bounds(option: Option<&QuantityUnitOption>) -> (Option<f64>, Option<f64>, String) {
    let Some(option) = option else {
        return (None, None, "any".to_owned());
    };
    let magnitude = option.magnitude.as_ref();
    let low = magnitude.and_then(|range| range.minimum);
    let high = magnitude.and_then(|range| range.maximum);
    let step = option
        .decimals
        .as_ref()
        .and_then(|range| range.maximum)
        .map_or_else(
            || "any".to_owned(),
            |most| {
                if most <= 0 {
                    return "1".to_owned();
                }
                let places = usize::try_from(most).unwrap_or(0);
                format!("0.{}1", "0".repeat(places.saturating_sub(1)))
            },
        );
    (low, high, step)
}

/// The value the magnitude and the unit record, where the template admits
/// them.
pub(crate) fn admit(field: &QuantityField, units: &str, typed: &str) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    if units.is_empty() {
        return Err(Refusal::Empty);
    }
    let option = unit(field, units);
    if !field.units.is_empty() && option.is_none() {
        return Err(Refusal::NotEnumerated);
    }
    let magnitude: f64 = typed.trim().parse().map_err(|_| Refusal::NotANumber)?;
    if !magnitude.is_finite() {
        return Err(Refusal::NotANumber);
    }
    let places = decimals(typed.trim()).ok_or(Refusal::NotPlainDecimal)?;
    if let Some(option) = option {
        if let Some(range) = option.magnitude.as_ref()
            && !within(range, &magnitude)
        {
            return Err(Refusal::OutsideRange);
        }
        if let Some(range) = option.decimals.as_ref()
            && !within(range, &places)
        {
            return Err(Refusal::TooPrecise);
        }
    }
    Ok(Datum::Quantity {
        magnitude,
        units: units.to_owned(),
        precision: i32::try_from(places).ok(),
    })
}

/// The magnitude and the unit in the slot, as the inputs show them.
fn shown(slot: &Slot) -> (String, String) {
    match slot.datum() {
        Some(Datum::Quantity {
            magnitude, units, ..
        }) => (magnitude.to_string(), units),
        _ => (String::new(), String::new()),
    }
}

/// A control over a magnitude with a unit.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn QuantityControl(
    /// What the template admits.
    field: QuantityField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let units_id = slot.part("units");
    let held = shown(&slot);

    // The unit the form is on decides the magnitude range and the decimal
    // places, so it is signal state rather than a read of the datum: a
    // clinician picks the unit before there is a magnitude to read it from.
    let chosen_unit = RwSignal::new(if held.1.is_empty() {
        field
            .units
            .first()
            .map_or_else(String::new, |option| option.units.clone())
    } else {
        held.1
    });
    let magnitude = RwSignal::new(held.0);

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            slot.apply(
                admit(
                    &field,
                    &chosen_unit.get_untracked(),
                    &magnitude.get_untracked(),
                ),
                refused,
            );
        }
    };

    let attributes = {
        let field = field.clone();
        move || {
            let units = chosen_unit.get();
            bounds(unit(&field, &units))
        }
    };
    let low = {
        let attributes = attributes.clone();
        move || attributes().0.map(|value| value.to_string())
    };
    let high = {
        let attributes = attributes.clone();
        move || attributes().1.map(|value| value.to_string())
    };
    let step = move || attributes().2;

    let names_units = !field.units.is_empty();
    let on_magnitude = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            magnitude.set(event_target_value(&event));
            record();
        }
    };
    let on_unit = Callback::new(move |units: String| {
        chosen_unit.set(units);
        record();
    });

    view! {
        <div class="grid gap-2 sm:grid-cols-2">
            <div>
                <label class=LABEL for=slot.id.clone()>
                    "Magnitude"
                </label>
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type="number"
                    min=low
                    max=high
                    step=step
                    prop:value=move || magnitude.get()
                    on:input=on_magnitude
                />
            </div>
            <UnitInput
                id=units_id.clone()
                units=field.units.clone()
                chosen=chosen_unit
                on_change=on_unit
            />
        </div>
        <Show when=move || !names_units>
            <span class=HINT>
                "The template names no unit, so any unit of the property is admitted."
            </span>
        </Show>
        <RefusalNote refused=refused />
    }
}

/// The unit beside the magnitude: a selection where the template names units,
/// and a free entry where it names none.
#[component]
fn UnitInput(
    /// The id the label points at.
    id: String,
    /// The units the template names, which may be none at all.
    units: Vec<QuantityUnitOption>,
    /// Which unit the form is on.
    chosen: RwSignal<String>,
    /// What to do when a reader picks another.
    on_change: Callback<String>,
) -> impl IntoView {
    let names_units = !units.is_empty();
    let choices: Vec<_> = units
        .iter()
        .map(|option| view! { <option value=option.units.clone()>{option.units.clone()}</option> })
        .collect();
    let picked = move |event: leptos::ev::Event| on_change.run(event_target_value(&event));

    view! {
        <div>
            <label class=LABEL for=id.clone()>
                "Unit"
            </label>
            <Show
                when=move || names_units
                fallback={
                    let id = id.clone();
                    move || {
                        view! {
                            <input
                                id=id.clone()
                                class=INPUT
                                type="text"
                                prop:value=move || chosen.get()
                                on:input=picked
                            />
                        }
                    }
                }
            >
                <select id=id.clone() class=SELECT prop:value=move || chosen.get() on:change=picked>
                    {choices.clone()}
                </select>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{QuantityField, QuantityUnitOption};
    use ferrochart_form::range::Range;
    use ferrochart_form::values::Datum;

    use super::{admit, bounds, unit};
    use crate::control::admit::Refusal;

    /// Two units whose magnitude ranges and decimal places differ, which is
    /// the case a control gets wrong by keeping one after a switch.
    fn pressure() -> QuantityField {
        QuantityField {
            property: None,
            units: vec![
                QuantityUnitOption {
                    units: "mm[Hg]".to_owned(),
                    magnitude: Some(Range::new(Some(0.0), Some(300.0), true, true)),
                    decimals: Some(Range::new(Some(0), Some(0), true, true)),
                },
                QuantityUnitOption {
                    units: "kPa".to_owned(),
                    magnitude: Some(Range::new(Some(0.0), Some(40.0), true, true)),
                    decimals: Some(Range::new(Some(0), Some(1), true, true)),
                },
            ],
        }
    }

    #[test]
    fn each_unit_carries_its_own_magnitude_range() {
        assert_eq!(
            admit(&pressure(), "mm[Hg]", "120"),
            Ok(Datum::Quantity {
                magnitude: 120.0,
                units: "mm[Hg]".to_owned(),
                precision: Some(0),
            })
        );
        // 120 is inside the millimetres-of-mercury range and outside the
        // kilopascal one, so a control that kept the old range would admit it.
        assert_eq!(admit(&pressure(), "kPa", "120"), Err(Refusal::OutsideRange));
    }

    #[test]
    fn each_unit_carries_its_own_decimal_precision() {
        assert_eq!(
            admit(&pressure(), "mm[Hg]", "120.5"),
            Err(Refusal::TooPrecise)
        );
        assert_eq!(
            admit(&pressure(), "kPa", "16.5"),
            Ok(Datum::Quantity {
                magnitude: 16.5,
                units: "kPa".to_owned(),
                precision: Some(1),
            })
        );
        assert_eq!(admit(&pressure(), "kPa", "16.55"), Err(Refusal::TooPrecise));
    }

    #[test]
    fn the_input_s_own_bounds_follow_the_chosen_unit() {
        let field = pressure();
        assert_eq!(
            bounds(unit(&field, "mm[Hg]")),
            (Some(0.0), Some(300.0), "1".to_owned())
        );
        assert_eq!(
            bounds(unit(&field, "kPa")),
            (Some(0.0), Some(40.0), "0.1".to_owned())
        );
    }

    #[test]
    fn a_unit_the_template_does_not_name_is_refused() {
        assert_eq!(admit(&pressure(), "psi", "1"), Err(Refusal::NotEnumerated));
    }

    #[test]
    fn a_template_that_names_no_unit_admits_any_unit_and_any_magnitude() {
        let field = QuantityField {
            property: None,
            units: Vec::new(),
        };
        assert_eq!(
            admit(&field, "furlong", "1.25"),
            Ok(Datum::Quantity {
                magnitude: 1.25,
                units: "furlong".to_owned(),
                precision: Some(2),
            })
        );
        assert_eq!(bounds(None), (None, None, "any".to_owned()));
    }

    #[test]
    fn a_magnitude_with_no_unit_and_a_unit_with_no_magnitude_are_both_unfilled() {
        assert_eq!(admit(&pressure(), "", "120"), Err(Refusal::Empty));
        assert_eq!(admit(&pressure(), "mm[Hg]", ""), Err(Refusal::Empty));
    }

    #[test]
    fn a_magnitude_written_with_an_exponent_states_no_decimal_places() {
        assert_eq!(
            admit(&pressure(), "mm[Hg]", "1.2e2"),
            Err(Refusal::NotPlainDecimal)
        );
        assert_eq!(
            admit(&pressure(), "mm[Hg]", "high"),
            Err(Refusal::NotANumber)
        );
    }
}
