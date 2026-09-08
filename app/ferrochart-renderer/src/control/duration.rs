// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_DURATION`: a length of time, in the slots the template's pattern
//! leaves open.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.9 gives the permitted
//! duration patterns as `P[Y|y][M|m][D|d][T[H|h][M|m][S|s]]` and `P[W|w]`, so
//! a designator the pattern omits is a slot the value may not fill, and the
//! week form is a pattern of its own rather than one more slot beside the
//! others.
//!
//! A negative duration is legal (openEHR RM Release-1.1.0 `data_types.html`
//! section 7.1.2.1), and ISO 8601 writes the sign in front of the whole
//! duration, so the control carries one sign rather than a sign per slot.

use std::collections::BTreeSet;

use ferrochart_form::field::{DurationComponent, DurationField};
use ferrochart_form::range::Range;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{INPUT, LABEL};

/// Every slot, in the order ISO 8601 writes them.
pub(crate) const ORDER: [DurationComponent; 7] = [
    DurationComponent::Years,
    DurationComponent::Months,
    DurationComponent::Weeks,
    DurationComponent::Days,
    DurationComponent::Hours,
    DurationComponent::Minutes,
    DurationComponent::Seconds,
];

/// The designator the slot writes, where this renderer knows the slot.
pub(crate) const fn designator(component: DurationComponent) -> Option<char> {
    match component {
        DurationComponent::Years => Some('Y'),
        DurationComponent::Months | DurationComponent::Minutes => Some('M'),
        DurationComponent::Weeks => Some('W'),
        DurationComponent::Days => Some('D'),
        DurationComponent::Hours => Some('H'),
        DurationComponent::Seconds => Some('S'),
        _ => None,
    }
}

/// What the slot is called, for a reader.
pub(crate) const fn name(component: DurationComponent) -> &'static str {
    match component {
        DurationComponent::Years => "Years",
        DurationComponent::Months => "Months",
        DurationComponent::Weeks => "Weeks",
        DurationComponent::Days => "Days",
        DurationComponent::Hours => "Hours",
        DurationComponent::Minutes => "Minutes",
        DurationComponent::Seconds => "Seconds",
        _ => "A slot this renderer does not know",
    }
}

/// Whether the slot sits after the `T`.
const fn after_the_t(component: DurationComponent) -> bool {
    matches!(
        component,
        DurationComponent::Hours | DurationComponent::Minutes | DurationComponent::Seconds
    )
}

/// The ISO 8601 duration the filled slots spell.
pub(crate) fn assemble(filled: &[(DurationComponent, i64)], negative: bool) -> String {
    let mut written = String::new();
    if negative {
        written.push('-');
    }
    written.push('P');
    for component in ORDER {
        let Some((_, magnitude)) = filled.iter().find(|(slot, _)| *slot == component) else {
            continue;
        };
        let Some(letter) = designator(component) else {
            continue;
        };
        if after_the_t(component) && !written.contains('T') {
            written.push('T');
        }
        written.push_str(&magnitude.to_string());
        written.push(letter);
    }
    written
}

/// The one slot a duration states, where it states exactly one.
///
/// A duration range is a pair of ISO 8601 strings, and ordering two durations
/// that use different designators needs a calendar, so a range is judged here
/// only where the comparison is a comparison of two numbers.
fn single_slot(text: &str) -> Option<(DurationComponent, i64)> {
    let negative = text.starts_with('-');
    let body = text.strip_prefix('-').unwrap_or(text).strip_prefix('P')?;
    let (after_t, body) = match body.strip_prefix('T') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    let counted = body.chars().take_while(char::is_ascii_digit).count();
    let (digits, rest) = body.split_at_checked(counted)?;
    if digits.is_empty() {
        return None;
    }
    let mut letters = rest.chars();
    let letter = letters.next()?;
    if letters.next().is_some() {
        return None;
    }
    let component = ORDER
        .into_iter()
        .find(|slot| designator(*slot) == Some(letter) && after_the_t(*slot) == after_t)?;
    let magnitude: i64 = digits.parse().ok()?;
    Some((component, if negative { -magnitude } else { magnitude }))
}

/// Whether the ranges the template states admit the filled slots.
///
/// A range whose ends do not share one slot with the value is left to the
/// server's validation gate.
fn ranges_admit(ranges: &[Range<String>], filled: &[(DurationComponent, i64)]) -> bool {
    let [(component, magnitude)] = filled else {
        return true;
    };
    for range in ranges {
        let low = range.minimum.as_deref().map(single_slot);
        let high = range.maximum.as_deref().map(single_slot);
        let comparable = |end: &Option<Option<(DurationComponent, i64)>>| match *end {
            None => true,
            Some(Some((slot, _))) => slot == *component,
            Some(None) => false,
        };
        if !comparable(&low) || !comparable(&high) {
            continue;
        }
        let numeric = Range::new(
            low.flatten().map(|(_, value)| value),
            high.flatten().map(|(_, value)| value),
            range.minimum_included,
            range.maximum_included,
        );
        if !crate::control::admit::within(&numeric, magnitude) {
            return false;
        }
    }
    true
}

/// The value the filled slots record, where the template admits them.
pub(crate) fn admit(
    field: &DurationField,
    filled: &[(DurationComponent, i64)],
    negative: bool,
) -> Result<Datum, Refusal> {
    if filled.is_empty() {
        return Err(Refusal::Empty);
    }
    if filled
        .iter()
        .any(|(slot, _)| !field.components.contains(slot) || designator(*slot).is_none())
    {
        return Err(Refusal::NotEnumerated);
    }
    if filled.iter().any(|(_, magnitude)| *magnitude < 0) {
        return Err(Refusal::Malformed);
    }
    let weeks = filled
        .iter()
        .any(|(slot, _)| *slot == DurationComponent::Weeks);
    if weeks && filled.len() > 1 {
        return Err(Refusal::Malformed);
    }
    // TODO(#150): judge a range whose ends do not share one slot with the
    // value.
    if !ranges_admit(&field.ranges, filled) {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::Duration(assemble(filled, negative)))
}

/// The slots a control draws, in ISO 8601 order.
pub(crate) fn slots(components: &BTreeSet<DurationComponent>) -> Vec<DurationComponent> {
    ORDER
        .into_iter()
        .filter(|component| components.contains(component))
        .collect()
}

/// A control over a length of time.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn DurationControl(
    /// What the template admits.
    field: DurationField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let drawn = slots(&field.components);
    let entries: Vec<(DurationComponent, RwSignal<String>)> = drawn
        .iter()
        .map(|component| (*component, RwSignal::new(String::new())))
        .collect();
    let negative = RwSignal::new(false);
    let held = StoredValue::new(entries.clone());

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            let mut filled = Vec::new();
            for (component, typed) in held.get_value() {
                let text = typed.get_untracked();
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                let Ok(magnitude) = text.parse::<i64>() else {
                    refused.set(Some(Refusal::NotANumber));
                    slot.clear();
                    return;
                };
                filled.push((component, magnitude));
            }
            slot.apply(admit(&field, &filled, negative.get_untracked()), refused);
        }
    };

    let boxes: Vec<_> = entries
        .iter()
        .map(|(component, typed)| {
            let id = slot.part(&name(*component).to_ascii_lowercase());
            let record = record.clone();
            let typed = *typed;
            view! {
                <div>
                    <label class=LABEL for=id.clone()>
                        {name(*component)}
                    </label>
                    <input
                        id=id
                        class=INPUT
                        type="number"
                        min="0"
                        step="1"
                        prop:value=move || typed.get()
                        on:input=move |event| {
                            typed.set(event_target_value(&event));
                            record();
                        }
                    />
                </div>
            }
        })
        .collect();

    let sign_id = slot.part("sign");
    let on_sign = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            negative.set(event_target_checked(&event));
            record();
        }
    };

    view! {
        <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">{boxes}</div>
        <label class="mt-2 inline-flex items-center gap-2 text-sm text-ink" for=sign_id.clone()>
            <input
                id=sign_id.clone()
                type="checkbox"
                class="h-4 w-4 accent-accent"
                prop:checked=move || negative.get()
                on:change=on_sign
            />
            "A duration into the past"
        </label>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ferrochart_form::field::{DurationComponent, DurationField};
    use ferrochart_form::range::Range;
    use ferrochart_form::values::Datum;

    use super::{admit, assemble, slots};
    use crate::control::admit::Refusal;

    fn permitting(components: &[DurationComponent]) -> DurationField {
        DurationField {
            components: components.iter().copied().collect::<BTreeSet<_>>(),
            ranges: Vec::new(),
        }
    }

    #[test]
    fn the_slots_are_written_in_the_order_iso_8601_states_them() {
        assert_eq!(
            assemble(
                &[
                    (DurationComponent::Seconds, 30),
                    (DurationComponent::Years, 1),
                    (DurationComponent::Hours, 2),
                ],
                false
            ),
            "P1YT2H30S"
        );
        assert_eq!(
            assemble(&[(DurationComponent::Days, 7)], true),
            "-P7D",
            "the sign belongs to the whole duration"
        );
    }

    #[test]
    fn a_slot_the_pattern_omits_is_refused() {
        let field = permitting(&[DurationComponent::Days]);
        assert_eq!(
            admit(&field, &[(DurationComponent::Days, 3)], false),
            Ok(Datum::Duration("P3D".to_owned()))
        );
        assert_eq!(
            admit(&field, &[(DurationComponent::Hours, 3)], false),
            Err(Refusal::NotEnumerated)
        );
        assert_eq!(slots(&field.components), [DurationComponent::Days]);
    }

    #[test]
    fn the_week_form_does_not_combine_with_another_slot() {
        let field = permitting(&[DurationComponent::Weeks, DurationComponent::Days]);
        assert_eq!(
            admit(&field, &[(DurationComponent::Weeks, 2)], false),
            Ok(Datum::Duration("P2W".to_owned()))
        );
        assert_eq!(
            admit(
                &field,
                &[(DurationComponent::Weeks, 2), (DurationComponent::Days, 1)],
                false
            ),
            Err(Refusal::Malformed)
        );
    }

    #[test]
    fn a_slot_with_no_magnitude_at_all_is_nothing_entered() {
        assert_eq!(
            admit(&permitting(&[DurationComponent::Days]), &[], false),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn a_range_over_one_slot_refuses_a_magnitude_outside_it() {
        let field = DurationField {
            components: [DurationComponent::Hours].into_iter().collect(),
            ranges: vec![Range::new(
                Some("PT0H".to_owned()),
                Some("PT24H".to_owned()),
                true,
                true,
            )],
        };
        assert_eq!(
            admit(&field, &[(DurationComponent::Hours, 12)], false),
            Ok(Datum::Duration("PT12H".to_owned()))
        );
        assert_eq!(
            admit(&field, &[(DurationComponent::Hours, 25)], false),
            Err(Refusal::OutsideRange)
        );
    }

    #[test]
    fn a_range_whose_ends_use_another_slot_is_left_to_the_server() {
        let field = DurationField {
            components: [DurationComponent::Days].into_iter().collect(),
            ranges: vec![Range::new(
                Some("P0M".to_owned()),
                Some("P1M".to_owned()),
                true,
                true,
            )],
        };
        assert_eq!(
            admit(&field, &[(DurationComponent::Days, 90)], false),
            Ok(Datum::Duration("P90D".to_owned())),
            "months against days has no answer without a calendar (#150)"
        );
    }

    #[test]
    fn a_slot_typed_with_a_minus_is_refused_because_the_sign_is_not_per_slot() {
        assert_eq!(
            admit(
                &permitting(&[DurationComponent::Days]),
                &[(DurationComponent::Days, -1)],
                false
            ),
            Err(Refusal::Malformed)
        );
    }
}
