// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A temporal pattern, read component by component and written back.
//!
//! A web template carries the archetype's own pattern in an input's
//! `validation.pattern`, so the spelling is the one openEHR AM Release-2.3.0
//! `AOM1.4.html` sections 6.2.6 to 6.2.8 give by example: "`YYYY-??-??` (date
//! with optional month and day)" and "`HH:??:xx` (time with optional minutes
//! and seconds not allowed)". The component letter makes a component
//! mandatory, `?` makes it optional, and `X` or `x` forbids it.

use ferrochart_form::field::ComponentValidity;

/// What one component of a pattern says.
///
/// A component the pattern does not reach is prohibited rather than optional,
/// which is the side that cannot admit what the template refuses.
fn component(part: Option<&str>) -> ComponentValidity {
    match part {
        None | Some("") => ComponentValidity::Prohibited,
        Some(text) if text.chars().all(|c| c == '?') => ComponentValidity::Optional,
        Some(text) if text.chars().all(|c| c == 'X' || c == 'x') => ComponentValidity::Prohibited,
        Some(_) => ComponentValidity::Mandatory,
    }
}

/// The month and day a date pattern admits.
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

fn clock(pattern: &str) -> (ComponentValidity, ComponentValidity, ComponentValidity) {
    let mut parts = pattern.split(':');
    (
        component(parts.next()),
        component(parts.next()),
        component(parts.next()),
    )
}

/// The minute and second a time pattern admits, the hour being implied.
pub(crate) fn time(pattern: Option<&str>) -> (ComponentValidity, ComponentValidity) {
    let Some(pattern) = pattern else {
        return (ComponentValidity::Optional, ComponentValidity::Optional);
    };
    let (_hour, minute, second) = clock(pattern);
    (minute, second)
}

/// The five components a date-and-time pattern admits: month, day, hour,
/// minute, second.
pub(crate) type DateTimeComponents = (
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
    ComponentValidity,
);

/// The five components a date-and-time pattern admits.
pub(crate) fn date_time(pattern: Option<&str>) -> DateTimeComponents {
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

/// How one component is spelled, given the letter it is mandatory as.
fn spell(validity: ComponentValidity, letter: char) -> String {
    let width = 2_usize;
    match validity {
        ComponentValidity::Mandatory => letter.to_string().repeat(width),
        ComponentValidity::Optional => "?".repeat(width),
        // A validity outside the three is spelled as the one that cannot
        // admit what the template refuses.
        _ => "X".repeat(width),
    }
}

/// The date pattern that states these components.
pub(crate) fn date_pattern(month: ComponentValidity, day: ComponentValidity) -> String {
    format!("YYYY-{}-{}", spell(month, 'M'), spell(day, 'D'))
}

/// The time pattern that states these components, the hour being mandatory.
pub(crate) fn time_pattern(minute: ComponentValidity, second: ComponentValidity) -> String {
    format!("HH:{}:{}", spell(minute, 'M'), spell(second, 'S'))
}

/// The date-and-time pattern that states these components.
pub(crate) fn date_time_pattern(components: DateTimeComponents) -> String {
    let (month, day, hour, minute, second) = components;
    format!(
        "{}T{}:{}:{}",
        date_pattern(month, day),
        spell(hour, 'H'),
        spell(minute, 'M'),
        spell(second, 'S')
    )
}

#[cfg(test)]
mod tests {
    use super::{date, date_pattern, date_time, date_time_pattern, time, time_pattern};
    use ferrochart_form::field::ComponentValidity;

    #[test]
    fn the_spec_example_patterns_read_as_the_spec_describes_them() {
        assert_eq!(
            date(Some("YYYY-??-??")),
            (ComponentValidity::Optional, ComponentValidity::Optional)
        );
        assert_eq!(
            time(Some("HH:??:xx")),
            (ComponentValidity::Optional, ComponentValidity::Prohibited)
        );
    }

    #[test]
    fn a_pattern_written_from_components_reads_back_as_those_components() {
        let pairs = [
            ComponentValidity::Mandatory,
            ComponentValidity::Optional,
            ComponentValidity::Prohibited,
        ];
        for month in pairs {
            for day in pairs {
                assert_eq!(date(Some(&date_pattern(month, day))), (month, day));
            }
            for second in pairs {
                assert_eq!(time(Some(&time_pattern(month, second))), (month, second));
            }
        }
    }

    #[test]
    fn a_date_time_pattern_written_from_components_reads_back_unchanged() {
        let components = (
            ComponentValidity::Mandatory,
            ComponentValidity::Mandatory,
            ComponentValidity::Optional,
            ComponentValidity::Optional,
            ComponentValidity::Prohibited,
        );
        assert_eq!(date_time(Some(&date_time_pattern(components))), components);
    }
}
