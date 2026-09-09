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
//! template fixes the precision and collects the components one at a time
//! where it admits several.

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

/// One component of a temporal value.
///
/// The separator and the expression are what openEHR AM Release-2.3.0
/// `AOM1.4.html` sections 6.2.6 to 6.2.9 constrain. The name and the width are
/// what a person reads and types, because a partial value is collected one
/// component at a time rather than typed against the whole pattern
/// (issue #197).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Component {
    /// What separates this component from the one before it.
    pub(crate) separator: &'static str,
    /// The regular expression this component's own text has to match.
    pub(crate) pattern: &'static str,
    /// What a person calls this component.
    pub(crate) name: &'static str,
    /// How many digits the component is written with, which is the width a
    /// shorter number is padded to.
    pub(crate) digits: usize,
}

/// The components of a date.
pub(crate) static DATE_PARTS: [Component; 3] = [
    Component {
        separator: "",
        pattern: "[0-9]{4}",
        name: "Year",
        digits: 4,
    },
    Component {
        separator: "-",
        pattern: "[0-9]{2}",
        name: "Month",
        digits: 2,
    },
    Component {
        separator: "-",
        pattern: "[0-9]{2}",
        name: "Day",
        digits: 2,
    },
];

/// The components of a time.
pub(crate) static TIME_PARTS: [Component; 3] = [
    Component {
        separator: "",
        pattern: "[0-9]{2}",
        name: "Hour",
        digits: 2,
    },
    Component {
        separator: ":",
        pattern: "[0-9]{2}",
        name: "Minute",
        digits: 2,
    },
    Component {
        separator: ":",
        pattern: "[0-9]{2}(\\.[0-9]+)?",
        name: "Second",
        digits: 2,
    },
];

/// The components of a date and time.
pub(crate) static DATE_TIME_PARTS: [Component; 6] = [
    Component {
        separator: "",
        pattern: "[0-9]{4}",
        name: "Year",
        digits: 4,
    },
    Component {
        separator: "-",
        pattern: "[0-9]{2}",
        name: "Month",
        digits: 2,
    },
    Component {
        separator: "-",
        pattern: "[0-9]{2}",
        name: "Day",
        digits: 2,
    },
    Component {
        separator: "T",
        pattern: "[0-9]{2}",
        name: "Hour",
        digits: 2,
    },
    Component {
        separator: ":",
        pattern: "[0-9]{2}",
        name: "Minute",
        digits: 2,
    },
    Component {
        separator: ":",
        pattern: "[0-9]{2}(\\.[0-9]+)?",
        name: "Second",
        digits: 2,
    },
];

/// The value the entered components spell, or why they spell none.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 7.1.2.2 builds a partial
/// value by dropping components from the RIGHT, so a gap in the middle is not
/// a shorter value: it is a value with nothing to say. A component short of
/// its width is padded with leading zeros, which is the conversion a person
/// should not be doing in their head (issue #197).
///
/// `Ok(None)` is nothing entered at all, which is not a refusal: a field
/// nobody has typed into yet is empty rather than wrong.
pub(crate) fn assembled(
    parts: &[Component],
    shape: Depth,
    entered: &[String],
) -> Result<Option<String>, Refusal> {
    let mut spelled = String::new();
    let mut stated = 0_usize;
    let mut gapped = false;
    for (index, part) in parts.iter().take(shape.most).enumerate() {
        let text = entered.get(index).map_or("", |value| value.trim());
        if text.is_empty() {
            gapped = true;
            continue;
        }
        if gapped {
            // A component to the right of an empty one, which the Reference
            // Model has no shape for.
            return Err(Refusal::Malformed);
        }
        spelled.push_str(part.separator);
        spelled.push_str(&padded(text, part.digits));
        stated = index.saturating_add(1);
    }
    if stated == 0 {
        return Ok(None);
    }
    if stated < shape.least {
        return Err(Refusal::Malformed);
    }
    Ok(Some(spelled))
}

/// `text` widened to `digits` with leading zeros, where it is short and
/// numeric.
///
/// A reader who types 9 for September means 09, and a fractional second is
/// left exactly as typed because padding it would change the number.
fn padded(text: &str, digits: usize) -> String {
    if text.len() >= digits || !text.chars().all(|digit| digit.is_ascii_digit()) {
        return text.to_owned();
    }
    let mut padded = String::with_capacity(digits);
    for _ in text.len()..digits {
        padded.push('0');
    }
    padded.push_str(text);
    padded
}

/// The components of a value already in the slot, one entry per component the
/// depth admits.
///
/// A value shorter than the depth leaves the components it does not state
/// empty, which is what the reader sees and what they can extend.
fn split_components(parts: &[Component], shape: Depth, value: &str) -> Vec<String> {
    let mut rest = value;
    let mut found = Vec::with_capacity(shape.most);
    for part in parts.iter().take(shape.most) {
        if rest.is_empty() {
            found.push(String::new());
            continue;
        }
        let Some(body) = rest.strip_prefix(part.separator) else {
            found.push(String::new());
            rest = "";
            continue;
        };
        let end = body
            .char_indices()
            .find(|&(_, character)| !character.is_ascii_digit() && character != '.')
            .map_or(body.len(), |(at, _)| at);
        let (component, remainder) = body.split_at(end);
        found.push(component.to_owned());
        rest = remainder;
    }
    found
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
    /// The component table the value is built from.
    parts: &'static [Component],
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

    // A native picker collects one precision exactly, and there is none that
    // collects a partial value, so the components are collected one at a time
    // where the template admits several (issue #197).
    //
    // The value and its timezone sit in one wrapping row rather than in a
    // two-column grid. A grid gave the value half the width whether or not a
    // timezone was beside it, which stacked six component boxes one per line
    // (issues #190 and #197).
    let body = match native {
        // The picker is capped rather than given the row: a date is a short
        // answer and a full-width one reads as a text field.
        Some((input_type, step)) => view! {
            <div class="w-full max-w-xs">
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type=input_type
                    step=step
                    prop:value=move || typed.get()
                    on:input=on_value
                />
            </div>
        }
        .into_any(),
        None => view! {
            <Components
                parts=parts
                shape=shape
                id=slot.id.clone()
                value=typed
                record=Callback::new({
                    let record = record.clone();
                    move |()| record()
                })
                refused=refused
            />
        }
        .into_any(),
    };

    view! {
        <div class="flex flex-wrap items-end gap-2">
            {body} <Show when=move || offers_zone>
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

/// The components of a partial value, one small box each.
///
/// No native control collects a partial date, and the fallback used to be one
/// text box carrying the ADL pattern as its placeholder: `YYYY[-MM[-DD]]`, the
/// specification's notation addressed to a clinician (issue #197). A partial
/// value is a precision choice, so it is drawn as one, on the precedent of the
/// GOV.UK Design System's date input, which collects a date a person knows as
/// separate labelled parts
/// (<https://design-system.service.gov.uk/components/date-input/>).
///
/// A component the template requires is marked required; the rest may be left
/// empty, and the value stops at the first one that is. Everything to the
/// right of a gap is refused rather than silently dropped, because openEHR RM
/// Release-1.1.0 `data_types.html` section 7.1.2.2 drops components from the
/// right and has no shape for a hole in the middle.
#[component]
fn Components(
    /// The component table this value is built from.
    parts: &'static [Component],
    /// How many components the template requires and admits.
    shape: Depth,
    /// The identifier the field's own label points at, which the first box
    /// carries so that label names something.
    id: String,
    /// The assembled value.
    value: RwSignal<String>,
    /// What recording the value looks like to the caller.
    record: Callback<()>,
    /// Where a refusal of the components themselves is reported.
    refused: RwSignal<Option<Refusal>>,
) -> impl IntoView {
    let held = split_components(parts, shape, &value.get_untracked());
    let entered: Vec<RwSignal<String>> = held.into_iter().map(RwSignal::new).collect();
    let boxes = StoredValue::new(entered.clone());

    let assemble = move || {
        let typed: Vec<String> = boxes.with_value(|each| each.iter().map(RwSignal::get).collect());
        match assembled(parts, shape, &typed) {
            Ok(spelled) => {
                refused.set(None);
                value.set(spelled.unwrap_or_default());
                record.run(());
            }
            Err(refusal) => refused.set(Some(refusal)),
        }
    };

    let drawn: Vec<_> = parts
        .iter()
        .take(shape.most)
        .enumerate()
        .map(|(index, part)| {
            let held = entered.get(index).copied().unwrap_or_else(|| {
                // Unreachable: `entered` is built from the same table and the
                // same depth. An empty signal draws an empty box rather than
                // panicking on a request path.
                RwSignal::new(String::new())
            });
            let required = index < shape.least;
            let box_id = if index == 0 {
                id.clone()
            } else {
                format!("{id}-{}", part.name.to_ascii_lowercase())
            };
            // The width is on the box rather than on the input, because the
            // input carries `w-full` and a second width class on the same
            // element is decided by the stylesheet's order rather than by
            // this one.
            let width = if part.digits > 2 { "w-24" } else { "w-20" };
            view! {
                <div class=width>
                    <label class=LABEL for=box_id.clone()>
                        {part.name}
                    </label>
                    <input
                        id=box_id
                        class=INPUT
                        type="text"
                        inputmode="numeric"
                        maxlength=part.digits.saturating_add(4)
                        pattern=part.pattern
                        required=required
                        prop:value=move || held.get()
                        on:input=move |event| {
                            held.set(event_target_value(&event));
                            assemble();
                        }
                    />
                </div>
            }
        })
        .collect();

    view! { <div class="flex flex-wrap items-end gap-2">{drawn}</div> }
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
                view! {
                    <option value=value selected=mine>
                        {text}
                    </option>
                }
            })
            .collect()
    });

    // A zone is not an offset: Europe/Amsterdam is +01:00 in January and
    // +02:00 in July, and `Iso8601_date_time` carries the offset (openEHR RM
    // Release-1.1.0 `data_types.html` section 7.2.4). So the offset is
    // resolved against the instant, and re-resolved whenever either the zone
    // or the instant changes. Resolving only on the pick left a summer offset
    // on a value the reader afterwards moved to January.
    Effect::new(move |_| {
        let picked = chosen.get();
        let at = instant.get();
        if picked.is_empty() {
            return;
        }
        let resolved = crate::zone::offset_at(&picked, &at).unwrap_or_default();
        if zone.get_untracked() == resolved {
            return;
        }
        zone.set(resolved);
        record.run(());
    });

    let pick = move |event: leptos::ev::Event| {
        let picked = event_target_value(&event);
        if picked.is_empty() {
            chosen.set(String::new());
            zone.set(String::new());
            record.run(());
            return;
        }
        // The effect above resolves it, so the pick only says which zone.
        chosen.set(picked);
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
                    <span class=HINT>{move || format!("{} here, at that moment", zone.get())}</span>
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
        assembled, date_depth, date_time_depth, depth, native_date, split_components, time_depth,
        timezone_admits,
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
    fn a_fixed_precision_gets_a_native_picker_and_an_open_one_gets_its_components() {
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

    /// The components a reader typed, as the control holds them.
    fn boxes(typed: &[&str]) -> Vec<String> {
        typed.iter().map(|&text| text.to_owned()).collect()
    }

    #[test]
    fn the_value_stops_at_the_first_component_a_reader_left_empty() {
        // RM Release-1.1.0 `data_types.html` section 7.1.2.2: components drop
        // from the right, so leaving the day empty is a year and a month.
        let shape = Depth { least: 1, most: 3 };
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "09", ""])),
            Ok(Some("2026-09".to_owned()))
        );
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "", ""])),
            Ok(Some("2026".to_owned()))
        );
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "09", "08"])),
            Ok(Some("2026-09-08".to_owned()))
        );
    }

    #[test]
    fn nothing_typed_at_all_is_empty_rather_than_refused() {
        let shape = Depth { least: 1, most: 3 };
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["", "", ""])),
            Ok(None)
        );
    }

    #[test]
    fn a_component_to_the_right_of_a_gap_is_refused() {
        // The Reference Model has no shape for a hole in the middle, so the
        // day is not silently dropped and the month is not silently invented.
        let shape = Depth { least: 1, most: 3 };
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "", "08"])),
            Err(Refusal::Malformed)
        );
    }

    #[test]
    fn a_value_short_of_what_the_template_requires_is_refused() {
        let shape = Depth { least: 3, most: 3 };
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "09", ""])),
            Err(Refusal::Malformed)
        );
        assert_eq!(
            assembled(&DATE_PARTS, shape, &boxes(&["2026", "09", "8"])),
            Ok(Some("2026-09-08".to_owned()))
        );
    }

    #[test]
    fn a_number_a_reader_typed_short_is_padded_rather_than_refused() {
        // Typing 9 for September is what a person does, and turning it into
        // 09 is the conversion they should not be doing themselves (#197).
        let shape = Depth { least: 1, most: 6 };
        assert_eq!(
            assembled(
                &DATE_TIME_PARTS,
                shape,
                &boxes(&["2026", "9", "8", "7", "5", "3"])
            ),
            Ok(Some("2026-09-08T07:05:03".to_owned()))
        );
    }

    #[test]
    fn a_fractional_second_is_left_exactly_as_it_was_typed() {
        // Padding it would change the number rather than its width.
        let shape = Depth { least: 1, most: 3 };
        assert_eq!(
            assembled(&TIME_PARTS, shape, &boxes(&["14", "30", "15.5"])),
            Ok(Some("14:30:15.5".to_owned()))
        );
    }

    #[test]
    fn a_value_already_entered_comes_back_as_the_components_that_spell_it() {
        let shape = Depth { least: 1, most: 3 };
        assert_eq!(
            split_components(&DATE_PARTS, shape, "2026-09"),
            boxes(&["2026", "09", ""])
        );
        assert_eq!(
            split_components(&DATE_PARTS, shape, ""),
            boxes(&["", "", ""])
        );
    }

    #[test]
    fn every_component_a_reader_sees_is_named_in_their_language() {
        // The control used to be one box carrying `YYYY[-MM[-DD]]`, which is
        // the ADL notation addressed to a clinician (issue #197).
        for table in [
            DATE_PARTS.as_slice(),
            TIME_PARTS.as_slice(),
            DATE_TIME_PARTS.as_slice(),
        ] {
            for part in table {
                assert!(!part.name.contains('['), "{}", part.name);
                assert!(
                    part.name.chars().all(|letter| letter.is_ascii_alphabetic()),
                    "{}",
                    part.name
                );
            }
        }
        assert_eq!(DATE_PARTS[0].name, "Year");
        assert_eq!(TIME_PARTS[0].name, "Hour");
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
            assembled(
                &DATE_TIME_PARTS,
                Depth { least: 5, most: 5 },
                &boxes(&["2026", "09", "08", "14", "30"])
            ),
            Ok(Some("2026-09-08T14:30".to_owned()))
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
}
