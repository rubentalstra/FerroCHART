// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_INTERVAL<T>`: two ends of the same field.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 6.2.2 carries a lower
//! and an upper value of one type, plus the inclusivity and unbounded flags.
//! Neither ADL generation has a dedicated constrainer, so each end is
//! constrained by the constrainer of the element type and each end is drawn
//! with the control that type calls for.
//!
//! A flag the template fixes is shown and never entered, so a control cannot
//! produce an interval the template refuses.

use ferrochart_form::field::IntervalField;
use ferrochart_form::values::{Datum, Entered};
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::control::{RefusalNote, Slot};
use crate::kit::field::LABEL;

/// Whether the lower end really is at or below the upper one.
///
/// `None` where the two are not comparable without knowing more than the form
/// does: a quantity in another unit, a coded value, an identifier. The
/// server's validation gate judges those, because `Limits_consistent`
/// (section 6.2.2) needs a strict comparison the browser cannot always make.
pub(crate) fn ordered(lower: &Datum, upper: &Datum) -> Option<bool> {
    match (lower, upper) {
        (Datum::Count(low), Datum::Count(high)) => Some(low <= high),
        (
            Datum::Quantity {
                magnitude: low,
                units: low_units,
                ..
            },
            Datum::Quantity {
                magnitude: high,
                units: high_units,
                ..
            },
        ) => (low_units == high_units).then(|| low <= high),
        (Datum::Date(low), Datum::Date(high))
        | (Datum::Time(low), Datum::Time(high))
        | (Datum::DateTime(low), Datum::DateTime(high)) => Some(low <= high),
        (Datum::Ordinal { value: low, .. }, Datum::Ordinal { value: high, .. }) => {
            Some(low <= high)
        }
        (Datum::Scale { value: low, .. }, Datum::Scale { value: high, .. }) => Some(low <= high),
        _ => None,
    }
}

/// The interval the two ends record, where the template admits it.
pub(crate) fn admit(
    field: &IntervalField,
    lower: Option<Datum>,
    upper: Option<Datum>,
    lower_included: bool,
    upper_included: bool,
) -> Result<Datum, Refusal> {
    if lower.is_none() && upper.is_none() {
        return Err(Refusal::Empty);
    }
    if field.lower_unbounded == Some(true) && lower.is_some() {
        return Err(Refusal::NothingAdmitted);
    }
    if field.upper_unbounded == Some(true) && upper.is_some() {
        return Err(Refusal::NothingAdmitted);
    }
    if field.lower_unbounded == Some(false) && lower.is_none() {
        return Err(Refusal::Empty);
    }
    if field.upper_unbounded == Some(false) && upper.is_none() {
        return Err(Refusal::Empty);
    }
    if let (Some(low), Some(high)) = (lower.as_ref(), upper.as_ref())
        && ordered(low, high) == Some(false)
    {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::Interval {
        lower: lower.map(Box::new),
        upper: upper.map(Box::new),
        lower_included: field.lower_included.unwrap_or(lower_included),
        upper_included: field.upper_included.unwrap_or(upper_included),
    })
}

/// A control over two ends of the same field.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn IntervalControl(
    /// What the template admits.
    field: IntervalField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let held = slot.datum();
    let (held_lower, held_upper, held_low_in, held_high_in) = match held {
        Some(Datum::Interval {
            lower,
            upper,
            lower_included,
            upper_included,
        }) => (
            lower.map(|end| Entered::Value(*end)),
            upper.map(|end| Entered::Value(*end)),
            lower_included,
            upper_included,
        ),
        _ => (None, None, true, true),
    };

    let lower = RwSignal::new(held_lower);
    let upper = RwSignal::new(held_upper);
    let lower_included = RwSignal::new(field.lower_included.unwrap_or(held_low_in));
    let upper_included = RwSignal::new(field.upper_included.unwrap_or(held_high_in));

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            let end = |held: RwSignal<Option<Entered>>| match held.get_untracked() {
                Some(Entered::Value(datum)) => Some(datum),
                Some(Entered::Null { .. }) | None => None,
            };
            slot.apply(
                admit(
                    &field,
                    end(lower),
                    end(upper),
                    lower_included.get_untracked(),
                    upper_included.get_untracked(),
                ),
                refused,
            );
        }
    };

    // The ends write into the interval's own signals, and every write
    // reassembles the pair, so the form holds one value rather than two.
    Effect::new({
        let record = record.clone();
        move |_| {
            lower.track();
            upper.track();
            lower_included.track();
            upper_included.track();
            record();
        }
    });

    let lower_slot = slot.scratch("lower", lower, "Lower".to_owned());
    let upper_slot = slot.scratch("upper", upper, "Upper".to_owned());
    let lower_view =
        crate::control::field::control(&field.lower, &field.element_rm_type, &lower_slot);
    let upper_view =
        crate::control::field::control(&field.upper, &field.element_rm_type, &upper_slot);

    view! {
        <div class="grid gap-3 sm:grid-cols-2">
            <div>
                <span class=LABEL>"Lower"</span>
                {lower_view}
                <Inclusivity
                    id=slot.part("lower-included")
                    label="The lower end is part of the interval"
                    fixed=field.lower_included
                    held=lower_included
                />
            </div>
            <div>
                <span class=LABEL>"Upper"</span>
                {upper_view}
                <Inclusivity
                    id=slot.part("upper-included")
                    label="The upper end is part of the interval"
                    fixed=field.upper_included
                    held=upper_included
                />
            </div>
        </div>
        <RefusalNote refused=refused />
    }
}

/// The checkbox for one end's inclusivity, or the fact where the template
/// fixed it.
#[component]
fn Inclusivity(
    /// The id the label points at.
    id: String,
    /// What the flag says.
    label: &'static str,
    /// The value the template fixed, where it fixed one.
    fixed: Option<bool>,
    /// Where the entered flag lives.
    held: RwSignal<bool>,
) -> impl IntoView {
    match fixed {
        Some(value) => view! {
            <p class="mt-1 text-xs text-ink-muted">
                {if value {
                    "The template includes this end."
                } else {
                    "The template excludes this end."
                }}
            </p>
        }
        .into_any(),
        None => view! {
            <label class="mt-1 inline-flex items-center gap-2 text-xs text-ink" for=id.clone()>
                <input
                    id=id.clone()
                    type="checkbox"
                    class="h-4 w-4 accent-accent"
                    prop:checked=move || held.get()
                    on:change=move |event| held.set(event_target_checked(&event))
                />
                {label}
            </label>
        }
        .into_any(),
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{CountField, FieldKind, IntervalField};
    use ferrochart_form::ids::RmTypeName;
    use ferrochart_form::values::Datum;

    use super::{admit, ordered};
    use crate::control::admit::Refusal;

    fn counts() -> IntervalField {
        IntervalField {
            element_rm_type: RmTypeName::new("DV_COUNT"),
            lower: FieldKind::Count(CountField::default()),
            upper: FieldKind::Count(CountField::default()),
            lower_included: None,
            upper_included: None,
            lower_unbounded: None,
            upper_unbounded: None,
        }
    }

    #[test]
    fn an_interval_with_neither_end_is_nothing_entered() {
        assert_eq!(
            admit(&counts(), None, None, true, true),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn a_template_that_fixes_an_end_s_inclusivity_wins_over_the_checkbox() {
        let field = IntervalField {
            lower_included: Some(false),
            ..counts()
        };
        assert_eq!(
            admit(
                &field,
                Some(Datum::Count(1)),
                Some(Datum::Count(9)),
                true,
                true
            ),
            Ok(Datum::Interval {
                lower: Some(Box::new(Datum::Count(1))),
                upper: Some(Box::new(Datum::Count(9))),
                lower_included: false,
                upper_included: true,
            })
        );
    }

    #[test]
    fn an_end_the_template_says_is_unbounded_admits_no_value() {
        let field = IntervalField {
            lower_unbounded: Some(true),
            ..counts()
        };
        assert_eq!(
            admit(
                &field,
                Some(Datum::Count(1)),
                Some(Datum::Count(9)),
                true,
                true
            ),
            Err(Refusal::NothingAdmitted)
        );
        assert_eq!(
            admit(&field, None, Some(Datum::Count(9)), true, true),
            Ok(Datum::Interval {
                lower: None,
                upper: Some(Box::new(Datum::Count(9))),
                lower_included: true,
                upper_included: true,
            })
        );
    }

    #[test]
    fn an_end_the_template_says_is_bounded_needs_a_value() {
        let field = IntervalField {
            upper_unbounded: Some(false),
            ..counts()
        };
        assert_eq!(
            admit(&field, Some(Datum::Count(1)), None, true, true),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn an_upper_end_below_the_lower_one_is_refused_where_the_two_compare() {
        assert_eq!(
            admit(
                &counts(),
                Some(Datum::Count(9)),
                Some(Datum::Count(1)),
                true,
                true
            ),
            Err(Refusal::OutsideRange)
        );
        assert_eq!(ordered(&Datum::Count(1), &Datum::Count(9)), Some(true));
    }

    #[test]
    fn two_ends_in_different_units_are_left_to_the_server() {
        let low = Datum::Quantity {
            magnitude: 1.0,
            units: "kPa".to_owned(),
            precision: None,
        };
        let high = Datum::Quantity {
            magnitude: 1.0,
            units: "mm[Hg]".to_owned(),
            precision: None,
        };
        assert_eq!(ordered(&low, &high), None);
        assert!(
            admit(&counts(), Some(low), Some(high), true, true).is_ok(),
            "an incomparable pair is judged by the validation gate"
        );
    }
}
