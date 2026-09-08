// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_PROPORTION`: a numerator over a denominator, of a stated kind.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 6.2.10 is the richest
//! invariant set in the Reference Model, and the kind decides which of them
//! apply: `Valid_denominator` forbids a zero denominator always,
//! `Unitary_validity` fixes the denominator at 1, `Percent_validity` fixes it
//! at 100, `Fraction_validity` makes both parts whole for `pk_fraction` and
//! `pk_integer_fraction`, `Precision_validity` makes both parts whole when the
//! precision is 0, and `Type_validity` refuses a kind outside the five section
//! 6.2.11 names.

use ferrochart_form::field::{ProportionField, ProportionKind, RealField};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, decimals, within_any};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{INPUT, LABEL, SELECT};

/// The five kinds section 6.2.11 names, in the order it numbers them.
const NAMED: [ProportionKind; 5] = [
    ProportionKind::Ratio,
    ProportionKind::Unitary,
    ProportionKind::Percent,
    ProportionKind::Fraction,
    ProportionKind::IntegerFraction,
];

/// The kinds a control offers.
///
/// A template that constrains `type` not at all admits every kind, so the
/// control offers the five the specification names.
pub(crate) fn offered(field: &ProportionField) -> Vec<ProportionKind> {
    if field.kinds.is_empty() {
        return NAMED.to_vec();
    }
    field.kinds.clone()
}

/// What the kind is called, for a reader.
pub(crate) const fn name(kind: ProportionKind) -> &'static str {
    match kind {
        ProportionKind::Ratio => "Ratio",
        ProportionKind::Unitary => "Unitary",
        ProportionKind::Percent => "Percent",
        ProportionKind::Fraction => "Fraction",
        ProportionKind::IntegerFraction => "Integer fraction",
        _ => "A kind this renderer does not know",
    }
}

/// The denominator the kind fixes, where it fixes one.
///
/// `Unitary_validity` and `Percent_validity`. A fixed denominator is shown
/// rather than entered, so a control cannot produce one the invariant
/// refuses.
pub(crate) const fn fixed_denominator(kind: ProportionKind) -> Option<f64> {
    match kind {
        ProportionKind::Unitary => Some(1.0),
        ProportionKind::Percent => Some(100.0),
        _ => None,
    }
}

/// Whether both parts have to be whole numbers.
///
/// `Fraction_validity` for the two fraction kinds, and
/// `ProportionField::is_integral` where the template states it.
pub(crate) fn requires_whole(field: &ProportionField, kind: ProportionKind) -> bool {
    matches!(
        kind,
        ProportionKind::Fraction | ProportionKind::IntegerFraction
    ) || field.is_integral == Some(true)
}

/// Whether a real constraint admits `value`.
fn real_admits(constraint: &RealField, value: f64) -> bool {
    if !constraint.options.is_empty() && !constraint.options.contains(&value) {
        return false;
    }
    within_any(&constraint.ranges, &value)
}

/// Whether `value` is a whole number.
#[expect(
    clippy::float_cmp,
    reason = "openEHR RM Release-1.1.0 data_types.html section 6.2.10 states \
              Fraction_validity as an exactness on the value, and a margin of \
              error would admit 2.0000001 as a whole numerator"
)]
fn is_whole(value: f64) -> bool {
    value.floor() == value
}

/// Whether the denominator is the zero `Valid_denominator` forbids.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.10 states
/// `Valid_denominator` as `denominator /= 0.0`, so the comparison is exact: a
/// margin of error would refuse a legitimate 0.0001.
fn is_zero(value: f64) -> bool {
    value == 0.0
}

/// The value the parts record, where every invariant the kind brings admits
/// them.
pub(crate) fn admit(
    field: &ProportionField,
    kind: ProportionKind,
    numerator_text: &str,
    denominator_text: &str,
) -> Result<Datum, Refusal> {
    if matches!(kind, ProportionKind::Other(_)) || !offered(field).contains(&kind) {
        return Err(Refusal::NotEnumerated);
    }
    if numerator_text.is_empty() {
        return Err(Refusal::Empty);
    }
    let numerator: f64 = numerator_text
        .trim()
        .parse()
        .map_err(|_| Refusal::NotANumber)?;
    let denominator = if let Some(fixed) = fixed_denominator(kind) {
        fixed
    } else {
        if denominator_text.is_empty() {
            return Err(Refusal::Empty);
        }
        denominator_text
            .trim()
            .parse()
            .map_err(|_| Refusal::NotANumber)?
    };
    if !numerator.is_finite() || !denominator.is_finite() {
        return Err(Refusal::NotANumber);
    }
    if is_zero(denominator) {
        return Err(Refusal::ZeroDenominator);
    }
    if requires_whole(field, kind) && !(is_whole(numerator) && is_whole(denominator)) {
        return Err(Refusal::NotWhole);
    }
    if !real_admits(&field.numerator, numerator) {
        return Err(Refusal::OutsideRange);
    }
    if !real_admits(&field.denominator, denominator) {
        return Err(Refusal::OutsideRange);
    }

    let numerator_places = decimals(numerator_text.trim()).ok_or(Refusal::NotPlainDecimal)?;
    let denominator_places = match fixed_denominator(kind) {
        Some(_) => 0,
        None => decimals(denominator_text.trim()).ok_or(Refusal::NotPlainDecimal)?,
    };
    let places = numerator_places.max(denominator_places);
    if !field.decimals.options.is_empty() && !field.decimals.options.contains(&places) {
        return Err(Refusal::TooPrecise);
    }
    if !within_any(&field.decimals.ranges, &places) {
        return Err(Refusal::TooPrecise);
    }

    Ok(Datum::Proportion {
        numerator,
        denominator,
        kind: kind.code(),
        precision: i32::try_from(places).ok(),
    })
}

/// The three parts in the slot, as the inputs show them.
fn shown(slot: &Slot) -> (String, String, Option<ProportionKind>) {
    match slot.datum() {
        Some(Datum::Proportion {
            numerator,
            denominator,
            kind,
            ..
        }) => (
            numerator.to_string(),
            denominator.to_string(),
            Some(ProportionKind::from_code(kind)),
        ),
        _ => (String::new(), String::new(), None),
    }
}

/// A control over a proportion.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn ProportionControl(
    /// What the template admits.
    field: ProportionField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let kinds = offered(&field);
    let held = shown(&slot);

    let kind = RwSignal::new(
        held.2
            .or_else(|| kinds.first().copied())
            .unwrap_or(ProportionKind::Ratio),
    );
    let numerator = RwSignal::new(held.0);
    let denominator = RwSignal::new(held.1);

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            slot.apply(
                admit(
                    &field,
                    kind.get_untracked(),
                    &numerator.get_untracked(),
                    &denominator.get_untracked(),
                ),
                refused,
            );
        }
    };

    let denominator_id = slot.part("denominator");
    let fixed = move || fixed_denominator(kind.get());
    let denominator_shown =
        move || fixed().map_or_else(|| denominator.get(), |value| value.to_string());

    let on_kind = {
        let record = record.clone();
        Callback::new(move |chosen: ProportionKind| {
            kind.set(chosen);
            record();
        })
    };
    let on_numerator = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            numerator.set(event_target_value(&event));
            record();
        }
    };
    let on_denominator = move |event: leptos::ev::Event| {
        denominator.set(event_target_value(&event));
        record();
    };

    view! {
        <div class="grid gap-2 sm:grid-cols-3">
            <KindPicker id=slot.part("kind") kinds=kinds chosen=kind on_change=on_kind />
            <div>
                <label class=LABEL for=slot.id.clone()>
                    "Numerator"
                </label>
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type="number"
                    step="any"
                    prop:value=move || numerator.get()
                    on:input=on_numerator
                />
            </div>
            <div>
                <label class=LABEL for=denominator_id.clone()>
                    "Denominator"
                </label>
                <input
                    id=denominator_id
                    class=INPUT
                    type="number"
                    step="any"
                    readonly=move || fixed().is_some()
                    prop:value=denominator_shown
                    on:input=on_denominator
                />
            </div>
        </div>
        <RefusalNote refused=refused />
    }
}

/// The kind selector, disabled where the template admits exactly one kind.
#[component]
fn KindPicker(
    /// The id the label points at.
    id: String,
    /// The kinds the template admits.
    kinds: Vec<ProportionKind>,
    /// Which kind the form is on.
    chosen: RwSignal<ProportionKind>,
    /// What to do when a reader picks another.
    on_change: Callback<ProportionKind>,
) -> impl IntoView {
    let single_kind = kinds.len() == 1;
    let choices: Vec<_> = kinds
        .iter()
        .map(|offered| {
            view! { <option value=offered.code().to_string()>{name(*offered)}</option> }
        })
        .collect();
    let picked = move |event: leptos::ev::Event| {
        let code = event_target_value(&event).parse().unwrap_or(0);
        on_change.run(ProportionKind::from_code(code));
    };

    view! {
        <div>
            <label class=LABEL for=id.clone()>
                "Kind"
            </label>
            <select
                id=id
                class=SELECT
                disabled=single_kind
                prop:value=move || chosen.get().code().to_string()
                on:change=picked
            >
                {choices}
            </select>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{CountField, ProportionField, ProportionKind, RealField};
    use ferrochart_form::range::Range;
    use ferrochart_form::values::Datum;

    use super::{admit, fixed_denominator, offered, requires_whole};
    use crate::control::admit::Refusal;

    fn anything() -> ProportionField {
        ProportionField {
            kinds: Vec::new(),
            numerator: RealField::default(),
            denominator: RealField::default(),
            decimals: CountField::default(),
            is_integral: None,
        }
    }

    #[test]
    fn a_template_that_states_no_kind_offers_the_five_the_spec_names() {
        assert_eq!(offered(&anything()).len(), 5);
    }

    #[test]
    fn a_zero_denominator_is_refused_whatever_the_kind() {
        assert_eq!(
            admit(&anything(), ProportionKind::Ratio, "1", "0"),
            Err(Refusal::ZeroDenominator)
        );
    }

    #[test]
    fn a_unitary_proportion_fixes_its_denominator_at_one() {
        assert_eq!(fixed_denominator(ProportionKind::Unitary), Some(1.0));
        assert_eq!(
            admit(&anything(), ProportionKind::Unitary, "3", "99"),
            Ok(Datum::Proportion {
                numerator: 3.0,
                denominator: 1.0,
                kind: 1,
                precision: Some(0),
            }),
            "the entered denominator cannot break Unitary_validity"
        );
    }

    #[test]
    fn a_percentage_fixes_its_denominator_at_a_hundred() {
        assert_eq!(fixed_denominator(ProportionKind::Percent), Some(100.0));
        assert_eq!(
            admit(&anything(), ProportionKind::Percent, "12.5", ""),
            Ok(Datum::Proportion {
                numerator: 12.5,
                denominator: 100.0,
                kind: 2,
                precision: Some(1),
            })
        );
    }

    #[test]
    fn a_fraction_refuses_a_part_that_is_not_whole() {
        assert!(requires_whole(&anything(), ProportionKind::Fraction));
        assert_eq!(
            admit(&anything(), ProportionKind::Fraction, "1", "2"),
            Ok(Datum::Proportion {
                numerator: 1.0,
                denominator: 2.0,
                kind: 3,
                precision: Some(0),
            })
        );
        assert_eq!(
            admit(&anything(), ProportionKind::Fraction, "1.5", "2"),
            Err(Refusal::NotWhole)
        );
        assert_eq!(
            admit(&anything(), ProportionKind::IntegerFraction, "3", "2.5"),
            Err(Refusal::NotWhole)
        );
    }

    #[test]
    fn a_template_that_says_integral_refuses_a_part_that_is_not() {
        let field = ProportionField {
            is_integral: Some(true),
            ..anything()
        };
        assert!(requires_whole(&field, ProportionKind::Ratio));
        assert_eq!(
            admit(&field, ProportionKind::Ratio, "1.5", "2"),
            Err(Refusal::NotWhole)
        );
    }

    #[test]
    fn a_kind_the_template_does_not_admit_is_refused() {
        let field = ProportionField {
            kinds: vec![ProportionKind::Percent],
            ..anything()
        };
        assert_eq!(
            admit(&field, ProportionKind::Ratio, "1", "2"),
            Err(Refusal::NotEnumerated)
        );
        assert_eq!(
            admit(&anything(), ProportionKind::Other(9), "1", "2"),
            Err(Refusal::NotEnumerated),
            "Type_validity refuses a kind outside the five the spec numbers"
        );
    }

    #[test]
    fn a_numerator_outside_the_range_the_template_states_is_refused() {
        let field = ProportionField {
            numerator: RealField {
                options: Vec::new(),
                ranges: vec![Range::new(Some(0.0), Some(10.0), true, true)],
            },
            ..anything()
        };
        assert_eq!(
            admit(&field, ProportionKind::Ratio, "11", "2"),
            Err(Refusal::OutsideRange)
        );
    }

    #[test]
    fn more_decimal_places_than_the_template_admits_are_refused() {
        let field = ProportionField {
            decimals: CountField {
                options: Vec::new(),
                ranges: vec![Range::new(Some(0), Some(1), true, true)],
            },
            ..anything()
        };
        assert_eq!(
            admit(&field, ProportionKind::Ratio, "1.25", "2"),
            Err(Refusal::TooPrecise)
        );
    }

    #[test]
    fn nothing_entered_is_nothing_entered() {
        assert_eq!(
            admit(&anything(), ProportionKind::Ratio, "", "2"),
            Err(Refusal::Empty)
        );
        assert_eq!(
            admit(&anything(), ProportionKind::Ratio, "1", ""),
            Err(Refusal::Empty)
        );
    }
}
