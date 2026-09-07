// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A date, time, date-and-time or duration pattern, read component by
//! component.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` sections 6.2.6 to 6.2.8 give the
//! pattern spelling by example: "`YYYY-??-??` (date with optional month and
//! day)" and "`HH:??:xx` (time with optional minutes and seconds not
//! allowed)". So the component letter makes a component mandatory, `?` makes
//! it optional, and `X` or `x` forbids it. Section 6.2.9 gives the duration
//! patterns as `P[Y|y][M|m][D|d][T[H|h][M|m][S|s]]` and `P[W|w]`, where a
//! designator the pattern omits is a slot the value may not fill.

use std::collections::BTreeSet;

use ferrochart_form::field::{ComponentValidity, DurationComponent};

use crate::model::payload::TimezoneValidity;

/// What one component of a pattern says.
///
/// A component the pattern does not reach is prohibited rather than optional.
/// No specification governs the short pattern, so FerroCHART takes the side
/// that cannot admit what the template refuses: a form that collects one
/// component too few loses precision, and a form that collects one too many
/// builds a value the template never allowed.
fn component(part: Option<&str>) -> ComponentValidity {
    match part {
        None | Some("") => ComponentValidity::Prohibited,
        Some(text) if text.chars().all(|c| c == '?') => ComponentValidity::Optional,
        Some(text) if text.chars().all(|c| c == 'X' || c == 'x') => ComponentValidity::Prohibited,
        Some(_) => ComponentValidity::Mandatory,
    }
}

/// The month and day a `C_DATE` pattern admits.
///
/// The year is not returned because openEHR RM Release-1.1.0
/// `data_types.html` section 7.1.2.2 builds a partial date by dropping
/// components from the right, so a date always carries its year.
pub(crate) fn date(pattern: Option<&str>) -> (ComponentValidity, ComponentValidity) {
    let Some(pattern) = pattern else {
        return (ComponentValidity::Optional, ComponentValidity::Optional);
    };
    let mut parts = pattern.split('-').skip(1);
    (component(parts.next()), component(parts.next()))
}

/// The minute and second a `C_TIME` pattern admits.
fn clock(pattern: &str) -> (ComponentValidity, ComponentValidity, ComponentValidity) {
    let mut parts = pattern.split(':');
    (
        component(parts.next()),
        component(parts.next()),
        component(parts.next()),
    )
}

/// The minute and second a `C_TIME` pattern admits, the hour being implied.
pub(crate) fn time(pattern: Option<&str>) -> (ComponentValidity, ComponentValidity) {
    let Some(pattern) = pattern else {
        return (ComponentValidity::Optional, ComponentValidity::Optional);
    };
    let (_hour, minute, second) = clock(pattern);
    (minute, second)
}

/// The five components a `C_DATE_TIME` pattern admits, in order: month, day,
/// hour, minute, second.
pub(crate) fn date_time(
    pattern: Option<&str>,
) -> (
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
) {
    let Some(pattern) = pattern else {
        return (
            ComponentValidity::Optional,
            ComponentValidity::Optional,
            ComponentValidity::Optional,
            ComponentValidity::Optional,
            ComponentValidity::Optional,
        );
    };
    let (date_part, time_part) = match pattern.split_once(['T', 't']) {
        Some((date_part, time_part)) => (date_part, Some(time_part)),
        None => (pattern, None),
    };
    let (month, day) = date(Some(date_part));
    let (hour, minute, second) = match time_part {
        None => (
            ComponentValidity::Prohibited,
            ComponentValidity::Prohibited,
            ComponentValidity::Prohibited,
        ),
        Some(time_part) => clock(time_part),
    };
    (month, day, hour, minute, second)
}

/// The slots a `C_DURATION` pattern admits.
///
/// The letters carry "or" semantics: each designator the pattern names is a
/// slot the value may fill, and a pattern that names none admits every slot.
pub(crate) fn duration(pattern: Option<&str>) -> BTreeSet<DurationComponent> {
    let Some(pattern) = pattern else {
        return every_duration_component();
    };
    let (head, tail) = match pattern.split_once(['T', 't']) {
        Some((head, tail)) => (head, tail),
        None => (pattern, ""),
    };
    let mut components = BTreeSet::new();
    for letter in head.chars() {
        match letter {
            'Y' | 'y' => components.insert(DurationComponent::Years),
            'M' | 'm' => components.insert(DurationComponent::Months),
            'W' | 'w' => components.insert(DurationComponent::Weeks),
            'D' | 'd' => components.insert(DurationComponent::Days),
            _ => false,
        };
    }
    for letter in tail.chars() {
        match letter {
            'H' | 'h' => components.insert(DurationComponent::Hours),
            'M' | 'm' => components.insert(DurationComponent::Minutes),
            'S' | 's' => components.insert(DurationComponent::Seconds),
            _ => false,
        };
    }
    if components.is_empty() {
        return every_duration_component();
    }
    components
}

fn every_duration_component() -> BTreeSet<DurationComponent> {
    [
        DurationComponent::Years,
        DurationComponent::Months,
        DurationComponent::Weeks,
        DurationComponent::Days,
        DurationComponent::Hours,
        DurationComponent::Minutes,
        DurationComponent::Seconds,
    ]
    .into_iter()
    .collect()
}

/// Whether a timezone must, may or must not be filled.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.7 carries the answer in
/// `C_TIME.timezone_validity`. A template that states none has not forbidden a
/// timezone, so the form collects an optional one.
pub(crate) fn timezone(validity: Option<TimezoneValidity>) -> ComponentValidity {
    match validity {
        None | Some(TimezoneValidity::Optional) => ComponentValidity::Optional,
        Some(TimezoneValidity::Mandatory) => ComponentValidity::Mandatory,
        Some(TimezoneValidity::Disallowed) => ComponentValidity::Prohibited,
    }
}

#[cfg(test)]
mod tests {
    use super::{date, date_time, duration, time};
    use ferrochart_form::field::{ComponentValidity, DurationComponent};

    #[test]
    fn a_date_pattern_is_read_component_by_component() {
        assert_eq!(
            date(Some("YYYY-??-??")),
            (ComponentValidity::Optional, ComponentValidity::Optional)
        );
        assert_eq!(
            date(Some("YYYY-MM-XX")),
            (ComponentValidity::Mandatory, ComponentValidity::Prohibited)
        );
        assert_eq!(
            date(Some("YYYY-MM-DD")),
            (ComponentValidity::Mandatory, ComponentValidity::Mandatory)
        );
    }

    #[test]
    fn the_spec_example_time_pattern_forbids_the_second() {
        assert_eq!(
            time(Some("HH:??:xx")),
            (ComponentValidity::Optional, ComponentValidity::Prohibited)
        );
    }

    #[test]
    fn a_date_time_pattern_splits_at_the_designator() {
        assert_eq!(
            date_time(Some("YYYY-MM-DDT??:??:??")),
            (
                ComponentValidity::Mandatory,
                ComponentValidity::Mandatory,
                ComponentValidity::Optional,
                ComponentValidity::Optional,
                ComponentValidity::Optional
            )
        );
    }

    #[test]
    fn a_date_time_pattern_with_no_time_part_forbids_every_clock_component() {
        assert_eq!(
            date_time(Some("YYYY-MM-DD")),
            (
                ComponentValidity::Mandatory,
                ComponentValidity::Mandatory,
                ComponentValidity::Prohibited,
                ComponentValidity::Prohibited,
                ComponentValidity::Prohibited
            )
        );
    }

    #[test]
    fn a_duration_pattern_names_the_slots_the_value_may_fill() {
        assert_eq!(
            duration(Some("PYMD")),
            [
                DurationComponent::Years,
                DurationComponent::Months,
                DurationComponent::Days
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            duration(Some("PW")),
            [DurationComponent::Weeks].into_iter().collect()
        );
        assert_eq!(
            duration(Some("PTHMS")),
            [
                DurationComponent::Hours,
                DurationComponent::Minutes,
                DurationComponent::Seconds
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            duration(Some("PWDTH")),
            [
                DurationComponent::Weeks,
                DurationComponent::Days,
                DurationComponent::Hours
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn a_duration_with_no_pattern_admits_every_slot() {
        assert_eq!(duration(None).len(), 7);
    }
}
