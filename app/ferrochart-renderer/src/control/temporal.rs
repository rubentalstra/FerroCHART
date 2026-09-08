// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_DATE`, `DV_TIME` and `DV_DATE_TIME`, at the precision the template
//! states and no finer.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 7.1.2.2 builds a partial
//! value by dropping components from the RIGHT, so the leading component is
//! always collected and the template decides how far to the right a value may
//! run. openEHR AM Release-2.3.0 `AOM1.4.html` sections 6.2.6 to 6.2.9 spell
//! that as a pattern per component: the letter for a mandatory component, `?`
//! for an optional one, `X` for one that is not allowed.
//!
//! A native picker collects one precision, so a control uses one where the
//! template fixes the precision and a typed value where it admits several.

use ferrochart_form::field::{ComponentValidity, DateField, DateTimeField, TimeField};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, within_any};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{HINT, INPUT, LABEL, SELECT};

/// How many components a value must and may carry, the leading one included.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Depth {
    /// The fewest components a value may state.
    pub(crate) least: usize,
    /// The most it may state.
    pub(crate) most: usize,
}

/// The depth an ordered list of component validities gives.
///
/// Components are dropped from the right, so a value runs as far right as the
/// first prohibited component and must run at least as far as the last
/// mandatory one.
pub(crate) fn depth(order: &[ComponentValidity]) -> Depth {
    let mut most = 1;
    for validity in order {
        if *validity == ComponentValidity::Prohibited {
            break;
        }
        most += 1;
    }
    let mut least = 1;
    for validity in order {
        if *validity != ComponentValidity::Mandatory {
            break;
        }
        least += 1;
    }
    Depth { least, most }
}

/// The depth a date field admits.
pub(crate) fn date_depth(field: &DateField) -> Depth {
    depth(&[field.month, field.day])
}

/// The depth a time field admits.
pub(crate) fn time_depth(field: &TimeField) -> Depth {
    depth(&[field.minute, field.second])
}

/// The depth a date-and-time field admits.
pub(crate) fn date_time_depth(field: &DateTimeField) -> Depth {
    depth(&[
        field.month,
        field.day,
        field.hour,
        field.minute,
        field.second,
    ])
}

/// The separator and shape of each component of a date.
pub(crate) static DATE_PARTS: [(&str, &str, &str); 3] = [
    ("", "[0-9]{4}", "YYYY"),
    ("-", "[0-9]{2}", "-MM"),
    ("-", "[0-9]{2}", "-DD"),
];

/// The separator and shape of each component of a time.
pub(crate) static TIME_PARTS: [(&str, &str, &str); 3] = [
    ("", "[0-9]{2}", "hh"),
    (":", "[0-9]{2}", ":mm"),
    (":", "[0-9]{2}(\\.[0-9]+)?", ":ss"),
];

/// The separator and shape of each component of a date and time.
pub(crate) static DATE_TIME_PARTS: [(&str, &str, &str); 6] = [
    ("", "[0-9]{4}", "YYYY"),
    ("-", "[0-9]{2}", "-MM"),
    ("-", "[0-9]{2}", "-DD"),
    ("T", "[0-9]{2}", "Thh"),
    (":", "[0-9]{2}", ":mm"),
    (":", "[0-9]{2}(\\.[0-9]+)?", ":ss"),
];

/// The regular expression a typed value of this depth has to match.
///
/// A component past the last mandatory one is optional, and optional
/// components nest, because dropping one drops every component to its right.
pub(crate) fn pattern_for(parts: &[(&str, &str, &str)], shape: Depth) -> String {
    let mut expression = String::new();
    let mut open = 0_usize;
    for (index, (separator, component, _)) in parts.iter().take(shape.most).enumerate() {
        if index >= shape.least {
            expression.push('(');
            open = open.saturating_add(1);
        }
        expression.push_str(separator);
        expression.push_str(component);
    }
    for _ in 0..open {
        expression.push_str(")?");
    }
    expression
}

/// What a reader is being asked for, where no native control collects it.
///
/// The placeholder is the ADL notation for the depth, and `YYYY[-MM[-DD]]` is
/// addressed to somebody reading the specification. This says the same thing
/// in the words a person would use (issue #197).
pub(crate) fn asked_for(parts: &[(&str, &str, &str)], shape: Depth) -> &'static str {
    // The component table says which of the three this is: a date and a time
    // both carry three, and only a date leads with a year.
    let leads_with = parts.first().map(|(_, _, token)| *token);
    match (parts.len(), leads_with, shape.least, shape.most) {
        (3, Some("YYYY"), 1, 2) => "Year, and month if known",
        (3, Some("YYYY"), 1, 3) => "Year, then month and day if known",
        (3, Some("YYYY"), 2, 3) => "Year and month, then day if known",
        (3, Some("hh"), 1, 2) => "Hour, and minutes if known",
        (3, Some("hh"), 1, 3) => "Hour, then minutes and seconds if known",
        (3, Some("hh"), 2, 3) => "Hour and minutes, then seconds if known",
        (6, _, _, 3) => "A date",
        (6, _, _, _) => "Date, and time if known",
        _ => "As much of it as you know",
    }
}

/// The native input type the depth calls for, where one collects exactly it.
///
/// `None` where the template admits several precisions: no native control
/// collects a partial date, so the value is typed against a pattern instead.
pub(crate) fn native_date(shape: Depth) -> Option<(&'static str, Option<&'static str>)> {
    match (shape.least, shape.most) {
        (3, 3) => Some(("date", None)),
        (2, 2) => Some(("month", None)),
        _ => None,
    }
}

/// The native input type a time depth calls for.
pub(crate) fn native_time(shape: Depth) -> Option<(&'static str, Option<&'static str>)> {
    match (shape.least, shape.most) {
        (2, 2) => Some(("time", Some("60"))),
        (3, 3) => Some(("time", Some("1"))),
        _ => None,
    }
}

/// The native input type a date-and-time depth calls for.
pub(crate) fn native_date_time(shape: Depth) -> Option<(&'static str, Option<&'static str>)> {
    match (shape.least, shape.most) {
        (5, 5) => Some(("datetime-local", Some("60"))),
        (6, 6) => Some(("datetime-local", Some("1"))),
        _ => None,
    }
}

/// Whether the timezone the value carries is the one the template admits.
pub(crate) fn timezone_admits(validity: ComponentValidity, present: bool) -> bool {
    match validity {
        ComponentValidity::Mandatory => present,
        ComponentValidity::Prohibited => !present,
        ComponentValidity::Optional => true,
        // A validity this renderer does not know is one it cannot honour, so
        // it admits nothing rather than admitting everything.
        _ => false,
    }
}

/// How many components an ISO 8601 extended-format date states.
fn date_parts(text: &str) -> Option<usize> {
    let mut parts = text.split('-');
    let year = parts.next()?;
    if year.chars().count() != 4 || !year.chars().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let mut stated = 1;
    if let Some(month) = parts.next() {
        if !two_digits_in(month, 1, 12) {
            return None;
        }
        stated = 2;
    }
    if let Some(day) = parts.next() {
        if stated != 2 || !two_digits_in(day, 1, 31) {
            return None;
        }
        stated = 3;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(stated)
}

/// Whether `text` is exactly two digits naming a number in the range.
fn two_digits_in(text: &str, low: u32, high: u32) -> bool {
    text.chars().count() == 2
        && text.chars().all(|digit| digit.is_ascii_digit())
        && text
            .parse::<u32>()
            .is_ok_and(|value| (low..=high).contains(&value))
}

/// How many components an ISO 8601 time states.
fn time_parts(text: &str) -> Option<usize> {
    let mut parts = text.split(':');
    let hour = parts.next()?;
    if !two_digits_in(hour, 0, 23) {
        return None;
    }
    let mut stated = 1;
    if let Some(minute) = parts.next() {
        if !two_digits_in(minute, 0, 59) {
            return None;
        }
        stated = 2;
    }
    if let Some(second) = parts.next() {
        let whole = second.split_once('.').map_or(second, |(before, _)| before);
        let fraction_ok = second.split_once('.').is_none_or(|(_, after)| {
            !after.is_empty() && after.chars().all(|d| d.is_ascii_digit())
        });
        if stated != 2 || !two_digits_in(whole, 0, 60) || !fraction_ok {
            return None;
        }
        stated = 3;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(stated)
}

/// The time without its timezone, and the timezone where it carries one.
fn split_timezone(time: &str) -> (&str, Option<&str>) {
    if let Some(rest) = time.strip_suffix('Z') {
        return (rest, Some("Z"));
    }
    if let Some(at) = time.rfind(['+', '-'])
        && at > 0
        && let (Some(before), Some(zone)) = (time.get(..at), time.get(at..))
    {
        return (before, Some(zone));
    }
    (time, None)
}

/// Whether a timezone is one ISO 8601 writes.
fn valid_timezone(zone: &str) -> bool {
    if zone == "Z" {
        return true;
    }
    let Some(offset) = zone.strip_prefix(['+', '-']) else {
        return false;
    };
    let (hour, minute) = match offset.split_once(':') {
        Some((hour, minute)) => (hour, Some(minute)),
        None => match offset.chars().count() {
            4 => (offset.get(..2).unwrap_or(""), offset.get(2..)),
            _ => (offset, None),
        },
    };
    two_digits_in(hour, 0, 14) && minute.is_none_or(|minute| two_digits_in(minute, 0, 59))
}

/// Whether the depth admits a value stating `stated` components.
fn depth_admits(shape: Depth, stated: usize) -> Result<(), Refusal> {
    if stated < shape.least {
        return Err(Refusal::TooCoarse);
    }
    if stated > shape.most {
        return Err(Refusal::TooFine);
    }
    Ok(())
}

/// The value `typed` records, where the template admits its precision.
pub(crate) fn admit_date(field: &DateField, typed: &str) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    let stated = date_parts(typed).ok_or(Refusal::Malformed)?;
    depth_admits(date_depth(field), stated)?;
    if !within_any(&field.ranges, &typed.to_owned()) {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::Date(typed.to_owned()))
}

/// The value `typed` plus `zone` records, where the template admits it.
pub(crate) fn admit_time(field: &TimeField, typed: &str, zone: &str) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    let value = format!("{typed}{zone}");
    let (bare, carried) = split_timezone(&value);
    if let Some(carried) = carried
        && !valid_timezone(carried)
    {
        return Err(Refusal::Malformed);
    }
    if !timezone_admits(field.timezone, carried.is_some()) {
        return Err(if carried.is_some() {
            Refusal::TooFine
        } else {
            Refusal::TooCoarse
        });
    }
    let stated = time_parts(bare).ok_or(Refusal::Malformed)?;
    depth_admits(time_depth(field), stated)?;
    if !within_any(&field.ranges, &value) {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::Time(value))
}

/// The value `typed` plus `zone` records, where the template admits it.
pub(crate) fn admit_date_time(
    field: &DateTimeField,
    typed: &str,
    zone: &str,
) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    let value = format!("{typed}{zone}");
    let (stated, carried) = match value.split_once('T') {
        None => (date_parts(&value).ok_or(Refusal::Malformed)?, false),
        Some((date, time)) => {
            if date_parts(date) != Some(3) {
                return Err(Refusal::Malformed);
            }
            let (bare, carried) = split_timezone(time);
            if let Some(carried) = carried
                && !valid_timezone(carried)
            {
                return Err(Refusal::Malformed);
            }
            let counted = time_parts(bare).ok_or(Refusal::Malformed)?;
            (counted.saturating_add(3), carried.is_some())
        }
    };
    if !timezone_admits(field.timezone, carried) {
        return Err(if carried {
            Refusal::TooFine
        } else {
            Refusal::TooCoarse
        });
    }
    depth_admits(date_time_depth(field), stated)?;
    if !within_any(&field.ranges, &value) {
        return Err(Refusal::OutsideRange);
    }
    Ok(Datum::DateTime(value))
}

/// The temporal text in the slot, split into the value and its timezone.
///
/// A date carries no timezone and its own separator is a hyphen, so the split
/// is made only where the control collects one.
fn shown(slot: &Slot, splits_timezone: bool) -> (String, String) {
    let Some(Datum::Date(held) | Datum::Time(held) | Datum::DateTime(held)) = slot.datum() else {
        return (String::new(), String::new());
    };
    if !splits_timezone {
        return (held, String::new());
    }
    let time = held.split_once('T').map_or(held.as_str(), |(_, time)| time);
    match split_timezone(time).1 {
        Some(zone) => (
            held.strip_suffix(zone).unwrap_or(&held).to_owned(),
            zone.to_owned(),
        ),
        None => (held.clone(), String::new()),
    }
}

/// A control over a date, a time, or a date and time.
#[component]
pub(crate) fn TemporalControl(
    /// What the template admits, as the shape a value takes.
    shape: Depth,
    /// The native input type the shape calls for, where one collects exactly
    /// it.
    native: Option<(&'static str, Option<&'static str>)>,
    /// The component table the pattern and the placeholder are built from.
    parts: &'static [(&'static str, &'static str, &'static str)],
    /// Whether a timezone is collected beside the value.
    timezone: Option<ComponentValidity>,
    /// Where the value goes.
    at: Slot,
    /// What turns the entered text into a value.
    admit: Callback<(String, String), Result<Datum, Refusal>>,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let offers_zone = timezone.is_some_and(|validity| validity != ComponentValidity::Prohibited);
    let held = shown(&slot, offers_zone);
    let typed = RwSignal::new(held.0);
    let zone = RwSignal::new(held.1);
    let zone_id = slot.part("timezone");
    let zone_required = timezone == Some(ComponentValidity::Mandatory);

    let record = {
        let slot = slot.clone();
        move || {
            slot.apply(
                admit.run((typed.get_untracked(), zone.get_untracked())),
                refused,
            );
        }
    };
    let on_value = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            typed.set(event_target_value(&event));
            record();
        }
    };

    let (input_type, step) = native.unwrap_or(("text", None));
    let pattern = native.is_none().then(|| pattern_for(parts, shape));
    // The pattern still gates what the browser accepts; only what a person
    // reads changes.
    let placeholder = native.is_none().then(|| asked_for(parts, shape));

    view! {
        <div class="grid gap-2 sm:grid-cols-2">
            <div>
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type=input_type
                    step=step
                    pattern=pattern
                    placeholder=placeholder
                    prop:value=move || typed.get()
                    on:input=on_value
                />
            </div>
            <Show when=move || offers_zone>
                <Timezone
                    id=zone_id.clone()
                    required=zone_required
                    zone=zone
                    instant=typed
                    record=Callback::new({
                        let record = record.clone();
                        move |()| record()
                    })
                />
            </Show>
        </div>
        <RefusalNote refused=refused />
    }
}

/// The timezone beside a time, chosen from the list the platform carries.
///
/// The reader picks where they are and this resolves it to the offset in
/// force at the instant they entered, which is what `Iso8601_date_time`
/// carries (`crate::zone`). A platform that does not publish the list falls
/// back to the offset field, so nothing becomes unreachable.
#[component]
fn Timezone(
    /// The identifier the label points at.
    id: String,
    /// Whether the template makes the timezone mandatory.
    required: bool,
    /// The offset the value carries, as ISO 8601 writes it.
    zone: RwSignal<String>,
    /// The value beside it, which decides which offset a zone is on.
    instant: RwSignal<String>,
    /// What recording the value looks like to the caller.
    record: Callback<()>,
) -> impl IntoView {
    let zones = StoredValue::new(crate::zone::every());
    let listed = zones.with_value(|every| !every.is_empty());
    // The reader's own zone is the answer nine times in ten, so it is what the
    // picker opens on. It is not written into the value until they choose:
    // a timezone nobody asked for is not one the template stated.
    let chosen = RwSignal::new(String::new());
    let named = if required {
        "Timezone"
    } else {
        "Timezone (optional)"
    };

    let options: Vec<_> = zones.with_value(|every| {
        let here = crate::zone::here();
        every
            .iter()
            .map(|zone| {
                let mine = here.as_deref() == Some(zone.as_str());
                let value = zone.clone();
                let text = zone.clone();
                view! { <option value=value selected=mine>{text}</option> }
            })
            .collect()
    });

    let pick = move |event: leptos::ev::Event| {
        let picked = event_target_value(&event);
        if picked.is_empty() {
            chosen.set(String::new());
            zone.set(String::new());
        } else {
            let at = instant.get_untracked();
            let resolved = crate::zone::offset_at(&picked, &at);
            chosen.set(picked);
            zone.set(resolved.unwrap_or_default());
        }
        record.run(());
    };
    let typed = move |event: leptos::ev::Event| {
        zone.set(event_target_value(&event));
        record.run(());
    };

    view! {
        <div>
            <label class=LABEL for=id.clone()>
                {named}
            </label>
            <Show
                when=move || listed
                fallback={
                    let id = id.clone();
                    move || {
                        view! {
                            <input
                                id=id.clone()
                                class=INPUT
                                type="text"
                                placeholder="+02:00"
                                pattern="Z|[+-][0-9]{2}(:?[0-9]{2})?"
                                prop:value=move || zone.get()
                                on:input=typed
                            />
                        }
                    }
                }
            >
                <select id=id.clone() class=SELECT on:change=pick>
                    <option value="">"Not stated"</option>
                    {options.clone()}
                </select>
                <Show when=move || !zone.get().is_empty()>
                    <span class=HINT>
                        {move || format!("{} here, at that moment", zone.get())}
                    </span>
                </Show>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{ComponentValidity, DateField, DateTimeField, TimeField};
    use ferrochart_form::range::Range;
    use ferrochart_form::values::Datum;

    use super::{
        DATE_PARTS, DATE_TIME_PARTS, Depth, TIME_PARTS, admit_date, admit_date_time, admit_time,
        date_depth, date_time_depth, depth, native_date, pattern_for, time_depth, timezone_admits,
    };
    use crate::control::admit::Refusal;

    fn date(month: ComponentValidity, day: ComponentValidity) -> DateField {
        DateField {
            month,
            day,
            ranges: Vec::new(),
        }
    }

    fn time(
        minute: ComponentValidity,
        second: ComponentValidity,
        timezone: ComponentValidity,
    ) -> TimeField {
        TimeField {
            minute,
            second,
            timezone,
            ranges: Vec::new(),
        }
    }

    #[test]
    fn a_prohibited_component_stops_the_value_and_every_one_to_its_right() {
        let field = date(ComponentValidity::Optional, ComponentValidity::Prohibited);
        assert_eq!(date_depth(&field), Depth { least: 1, most: 2 });
        assert_eq!(
            admit_date(&field, "2026"),
            Ok(Datum::Date("2026".to_owned()))
        );
        assert_eq!(
            admit_date(&field, "2026-09"),
            Ok(Datum::Date("2026-09".to_owned()))
        );
        assert_eq!(admit_date(&field, "2026-09-08"), Err(Refusal::TooFine));
    }

    #[test]
    fn a_mandatory_component_makes_a_coarser_value_a_refusal() {
        let field = date(ComponentValidity::Mandatory, ComponentValidity::Mandatory);
        assert_eq!(date_depth(&field), Depth { least: 3, most: 3 });
        assert_eq!(admit_date(&field, "2026"), Err(Refusal::TooCoarse));
        assert_eq!(admit_date(&field, "2026-09"), Err(Refusal::TooCoarse));
        assert_eq!(
            admit_date(&field, "2026-09-08"),
            Ok(Datum::Date("2026-09-08".to_owned()))
        );
    }

    #[test]
    fn a_fixed_precision_gets_a_native_picker_and_an_open_one_gets_a_pattern() {
        assert_eq!(
            native_date(Depth { least: 3, most: 3 }),
            Some(("date", None))
        );
        assert_eq!(
            native_date(Depth { least: 2, most: 2 }),
            Some(("month", None))
        );
        assert_eq!(native_date(Depth { least: 1, most: 3 }), None);
    }

    #[test]
    fn the_pattern_nests_every_component_past_the_last_mandatory_one() {
        assert_eq!(
            pattern_for(&DATE_PARTS, Depth { least: 1, most: 3 }),
            "[0-9]{4}(-[0-9]{2}(-[0-9]{2})?)?"
        );
        assert_eq!(
            pattern_for(&DATE_PARTS, Depth { least: 3, most: 3 }),
            "[0-9]{4}-[0-9]{2}-[0-9]{2}"
        );
    }

    #[test]
    fn a_time_carries_its_precision_and_its_timezone_separately() {
        let field = time(
            ComponentValidity::Mandatory,
            ComponentValidity::Prohibited,
            ComponentValidity::Prohibited,
        );
        assert_eq!(time_depth(&field), Depth { least: 2, most: 2 });
        assert_eq!(
            admit_time(&field, "08:30", ""),
            Ok(Datum::Time("08:30".to_owned()))
        );
        assert_eq!(admit_time(&field, "08:30:15", ""), Err(Refusal::TooFine));
        assert_eq!(admit_time(&field, "08:30", "Z"), Err(Refusal::TooFine));
    }

    #[test]
    fn a_mandatory_timezone_refuses_a_value_without_one() {
        let field = time(
            ComponentValidity::Mandatory,
            ComponentValidity::Optional,
            ComponentValidity::Mandatory,
        );
        assert_eq!(admit_time(&field, "08:30", ""), Err(Refusal::TooCoarse));
        assert_eq!(
            admit_time(&field, "08:30", "+02:00"),
            Ok(Datum::Time("08:30+02:00".to_owned()))
        );
        assert!(timezone_admits(ComponentValidity::Optional, false));
        assert!(timezone_admits(ComponentValidity::Optional, true));
    }

    #[test]
    fn a_date_and_time_counts_both_halves_toward_one_depth() {
        let field = DateTimeField {
            month: ComponentValidity::Mandatory,
            day: ComponentValidity::Mandatory,
            hour: ComponentValidity::Mandatory,
            minute: ComponentValidity::Mandatory,
            second: ComponentValidity::Prohibited,
            timezone: ComponentValidity::Optional,
            ranges: Vec::new(),
        };
        assert_eq!(date_time_depth(&field), Depth { least: 5, most: 5 });
        assert_eq!(
            admit_date_time(&field, "2026-09-08T14:30", ""),
            Ok(Datum::DateTime("2026-09-08T14:30".to_owned()))
        );
        assert_eq!(
            admit_date_time(&field, "2026-09-08T14:30:15", ""),
            Err(Refusal::TooFine)
        );
        assert_eq!(
            admit_date_time(&field, "2026-09-08", ""),
            Err(Refusal::TooCoarse)
        );
        assert_eq!(
            pattern_for(&DATE_TIME_PARTS, Depth { least: 5, most: 5 }),
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}"
        );
    }

    #[test]
    fn a_value_outside_the_range_the_template_states_is_refused() {
        let field = DateField {
            month: ComponentValidity::Mandatory,
            day: ComponentValidity::Mandatory,
            ranges: vec![Range::new(
                Some("2020-01-01".to_owned()),
                Some("2030-12-31".to_owned()),
                true,
                true,
            )],
        };
        assert_eq!(
            admit_date(&field, "2026-09-08"),
            Ok(Datum::Date("2026-09-08".to_owned()))
        );
        assert_eq!(admit_date(&field, "2019-12-31"), Err(Refusal::OutsideRange));
    }

    #[test]
    fn text_that_is_not_a_date_at_all_is_refused() {
        let field = date(ComponentValidity::Optional, ComponentValidity::Optional);
        assert_eq!(admit_date(&field, "the eighth"), Err(Refusal::Malformed));
        assert_eq!(admit_date(&field, "2026-13"), Err(Refusal::Malformed));
        assert_eq!(admit_date(&field, "26-09-08"), Err(Refusal::Malformed));
        assert_eq!(admit_date(&field, ""), Err(Refusal::Empty));
    }

    #[test]
    fn the_depth_of_an_all_optional_list_runs_from_one_to_the_end() {
        assert_eq!(
            depth(&[ComponentValidity::Optional, ComponentValidity::Optional]),
            Depth { least: 1, most: 3 }
        );
    }
    #[test]
    fn a_partial_date_is_asked_for_in_words_and_never_in_adl() {
        // The placeholder was `YYYY[-MM[-DD]]`, which is the notation and not
        // the question (issue #197).
        for shape in [
            Depth { least: 1, most: 2 },
            Depth { least: 1, most: 3 },
            Depth { least: 2, most: 3 },
        ] {
            let said = super::asked_for(&DATE_PARTS, shape);
            assert!(!said.contains("YYYY"), "{said}");
            assert!(!said.contains('['), "{said}");
            assert!(said.to_lowercase().contains("year"), "{said}");
        }
    }

    #[test]
    fn a_partial_time_and_a_partial_date_are_told_apart() {
        let shape = Depth { least: 1, most: 3 };
        let lowered = |said: &str| said.to_lowercase();
        assert!(lowered(super::asked_for(&TIME_PARTS, shape)).contains("hour"));
        assert!(lowered(super::asked_for(&DATE_PARTS, shape)).contains("year"));
        assert!(
            lowered(super::asked_for(
                &DATE_TIME_PARTS,
                Depth { least: 1, most: 6 }
            ))
            .contains("date")
        );
    }
}
