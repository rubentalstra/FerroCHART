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
use crate::kit::field::{INPUT, SELECT};

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
    // One part per unit. Two parts naming the same one would assemble into a
    // duration carrying whichever `assemble` found first, silently dropping
    // the other.
    if filled.iter().enumerate().any(|(at, (slot, _))| {
        filled
            .iter()
            .skip(at.saturating_add(1))
            .any(|(other, _)| other == slot)
    }) {
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

/// Whether a negative duration would survive the ranges the template states.
///
/// A range with a lower bound of zero or above refuses one, so the sign is not
/// offered where it could only ever be a refusal. Hiding it changes no
/// judgement: [`admit`] is what decides, and a range still refuses whatever it
/// refused before.
fn admits_negative(field: &DurationField) -> bool {
    !field.ranges.iter().any(|range| {
        matches!(
            range.minimum.as_deref().and_then(single_slot),
            Some((_, low)) if low >= 0
        )
    })
}

/// One part of a duration a reader is filling: how many, and of what.
#[derive(Clone, Copy)]
struct Part {
    /// How many, as typed.
    magnitude: RwSignal<String>,
    /// Which unit it counts.
    unit: RwSignal<DurationComponent>,
}

/// A control over a length of time.
///
/// A duration is one number and one unit, because that is how a person says
/// one: an age is "82 years", not eighty-two in a row of seven boxes. Where
/// the template admits more than one unit a second part can be added, so a
/// compound duration stays reachable and costs nothing on a field that does
/// not use one (issue #198).
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
    let units = slots(&field.components);
    let Some(&first) = units.first() else {
        // A duration whose pattern leaves no slot open admits no value, which
        // is a template that fixed it. Nothing is drawn rather than an empty
        // row of controls.
        return view! { <p class="text-sm text-ink-muted">"This field takes no value."</p> }
            .into_any();
    };
    // One part per admitted unit is the most a duration can carry, so every
    // part's signals exist from the start and `shown` decides how many are
    // drawn. A growing list of signals would be the same thing with a keyed
    // loop around it.
    let parts: Vec<Part> = units
        .iter()
        .map(|unit| Part {
            magnitude: RwSignal::new(String::new()),
            unit: RwSignal::new(*unit),
        })
        .collect();
    let held = StoredValue::new(parts.clone());
    let shown = RwSignal::new(1_usize);
    let negative = RwSignal::new(false);
    let most = units.len();
    let one_unit = most == 1;
    let signed = admits_negative(&field);

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            let mut filled = Vec::new();
            for part in held.get_value().into_iter().take(shown.get_untracked()) {
                let typed = part.magnitude.get_untracked();
                let typed = typed.trim();
                if typed.is_empty() {
                    continue;
                }
                let Ok(magnitude) = typed.parse::<i64>() else {
                    refused.set(Some(Refusal::NotANumber));
                    slot.clear();
                    return;
                };
                filled.push((part.unit.get_untracked(), magnitude));
            }
            slot.apply(admit(&field, &filled, negative.get_untracked()), refused);
        }
    };

    let recorder = Callback::new({
        let record = record.clone();
        move |()| record()
    });
    let rows: Vec<_> = parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            view! {
                <DurationPart
                    part=*part
                    index=index
                    units=units.clone()
                    only=one_unit.then_some(first)
                    shown=shown
                    id=slot.part(&format!("part-{index}"))
                    unit_id=slot.part(&format!("unit-{index}"))
                    record=recorder
                />
            }
        })
        .collect();

    view! {
        <div class="flex flex-col gap-2">
            {rows} <Show when=move || !one_unit && shown.get() < most>
                <button
                    type="button"
                    class="self-start text-xs text-ink-muted underline underline-offset-2 hover:text-ink"
                    on:click=move |_| shown.update(|count| *count = count.saturating_add(1))
                >
                    "and…"
                </button>
            </Show> <Show when=move || signed>
                <Backwards id=slot.part("sign") negative=negative record=recorder />
            </Show>
        </div>
        <RefusalNote refused=refused />
    }
    .into_any()
}

/// The sign of a duration, where the template admits a negative one.
#[component]
fn Backwards(
    /// The identifier the label points at.
    id: String,
    /// Whether the duration counts backwards.
    negative: RwSignal<bool>,
    /// What recording the value looks like to the caller.
    record: Callback<()>,
) -> impl IntoView {
    view! {
        <label class="inline-flex items-center gap-2 text-sm text-ink" for=id.clone()>
            <input
                id=id.clone()
                type="checkbox"
                class="h-4 w-4 accent-accent"
                prop:checked=move || negative.get()
                on:change=move |event| {
                    negative.set(event_target_checked(&event));
                    record.run(());
                }
            />
            "Counts backwards"
        </label>
    }
}

/// One part of a duration: how many, and of what.
#[component]
fn DurationPart(
    /// The signals this part writes into.
    part: Part,
    /// Which part of the duration this is, counting from the first.
    index: usize,
    /// Every unit the template admits.
    units: Vec<DurationComponent>,
    /// The one unit, where the template admits exactly one and no picker is
    /// drawn.
    only: Option<DurationComponent>,
    /// How many parts the reader has asked for.
    shown: RwSignal<usize>,
    /// The identifier the amount's label points at.
    id: String,
    /// The identifier the unit's label points at.
    unit_id: String,
    /// What recording the value looks like to the caller.
    record: Callback<()>,
) -> impl IntoView {
    let choices: Vec<_> = units
        .iter()
        .map(|unit| {
            let unit = *unit;
            view! {
                <option value=name(unit) selected=move || part.unit.get() == unit>
                    {name(unit).to_lowercase()}
                </option>
            }
        })
        .collect();
    // Hoisted out of the `view!` macro: a bare `>` inside an attribute ends
    // the tag as far as the macro is concerned.
    let removable = move || index != 0 && index == shown.get().saturating_sub(1);
    let visible = move || index < shown.get();
    let pick = move |event: leptos::ev::Event| {
        let chosen = event_target_value(&event);
        if let Some(unit) = units_named(&chosen) {
            part.unit.set(unit);
        }
        record.run(());
    };

    view! {
        <div class="flex items-end gap-2" hidden=move || !visible()>
            <div class="w-28">
                <label class="sr-only" for=id.clone()>
                    "How many"
                </label>
                <input
                    id=id.clone()
                    class=INPUT
                    type="number"
                    min="0"
                    step="1"
                    prop:value=move || part.magnitude.get()
                    on:input=move |event| {
                        part.magnitude.set(event_target_value(&event));
                        record.run(());
                    }
                />
            </div>
            <Show
                when=move || only.is_none()
                fallback=move || {
                    view! {
                        <span class="pb-1.5 text-sm text-ink-muted">
                            {only.map(|unit| name(unit).to_lowercase())}
                        </span>
                    }
                }
            >
                <div class="w-40">
                    <label class="sr-only" for=unit_id.clone()>
                        "Of what"
                    </label>
                    <select id=unit_id.clone() class=SELECT on:change=pick>
                        {choices.clone()}
                    </select>
                </div>
            </Show>
            <button
                type="button"
                hidden=move || !removable()
                class="pb-1.5 text-xs text-ink-muted underline underline-offset-2 hover:text-ink"
                on:click=move |_| {
                    part.magnitude.set(String::new());
                    shown.update(|count| *count = count.saturating_sub(1));
                    record.run(());
                }
            >
                "Remove"
            </button>
        </div>
    }
}

/// The unit a rendered option names.
fn units_named(name: &str) -> Option<DurationComponent> {
    ORDER
        .into_iter()
        .find(|component| super::duration::name(*component) == name)
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
    #[test]
    fn two_parts_naming_one_unit_are_refused() {
        // `assemble` writes the first match per unit, so a second part naming
        // the same one would be dropped without a word.
        let field = permitting(&[DurationComponent::Years, DurationComponent::Months]);
        let twice = [
            (DurationComponent::Years, 1_i64),
            (DurationComponent::Years, 2_i64),
        ];
        assert_eq!(admit(&field, &twice, false), Err(Refusal::Malformed));
    }

    #[test]
    fn two_parts_naming_different_units_are_admitted() {
        let field = permitting(&[DurationComponent::Years, DurationComponent::Months]);
        let both = [
            (DurationComponent::Years, 1_i64),
            (DurationComponent::Months, 3_i64),
        ];
        assert_eq!(
            admit(&field, &both, false),
            Ok(Datum::Duration("P1Y3M".to_owned()))
        );
    }

    #[test]
    fn a_range_that_starts_at_zero_rules_the_sign_out() {
        // The control does not offer a sign a range could only refuse.
        let mut field = permitting(&[DurationComponent::Years]);
        field.ranges = vec![Range::new(
            Some("P0Y".to_owned()),
            Some("P200Y".to_owned()),
            true,
            true,
        )];
        assert!(!super::admits_negative(&field));
    }

    #[test]
    fn a_field_with_no_range_still_offers_the_sign() {
        let field = permitting(&[DurationComponent::Years]);
        assert!(super::admits_negative(&field));
    }
}
