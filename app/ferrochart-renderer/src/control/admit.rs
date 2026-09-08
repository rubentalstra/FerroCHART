// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a field may hold, decided before anything is drawn.
//!
//! Every control renders from a function in this module or in one of its
//! siblings, and never decides admission inside an event handler. A form
//! never admits what the template refuses, so the decision is a pure function
//! a test can drive; a decision buried in a closure is one only a browser can
//! check.

use ferrochart_form::field::TextField;
use ferrochart_form::range::Range;

/// Why a control refuses what a clinician entered.
///
/// The variants are the refusals a renderer can prove on its own. A refusal
/// that needs the whole document, or a terminology server, belongs to the
/// server's validation report rather than here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    /// Nothing was entered where something has to be.
    Empty,
    /// The text is not a number.
    NotANumber,
    /// The text is a number written with an exponent, which carries no
    /// decimal place count.
    NotPlainDecimal,
    /// The value is a number where the template requires a whole one.
    NotWhole,
    /// The value sits outside every range the template admits.
    OutsideRange,
    /// The value carries more decimal places than the template admits.
    TooPrecise,
    /// The value is not one of the ones the template enumerates.
    NotEnumerated,
    /// The template admits no value here at all.
    NothingAdmitted,
    /// The URI does not carry the scheme the Reference Model class requires.
    WrongScheme,
    /// The denominator is zero.
    ZeroDenominator,
    /// The value states fewer components than the template requires.
    TooCoarse,
    /// The value states more components than the template admits.
    TooFine,
    /// The value is not a date, a time, or a duration of the shape the
    /// template states.
    Malformed,
}

impl Refusal {
    /// What the refusal says to the person who entered the value.
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::Empty => "Enter a value.",
            Self::NotANumber => "Enter a number.",
            Self::NotPlainDecimal => "Write the number in plain decimal form.",
            Self::NotWhole => "Enter a whole number.",
            Self::OutsideRange => "The template does not admit that value.",
            Self::TooPrecise => "The template admits fewer decimal places than that.",
            Self::NotEnumerated => "Choose one of the values the template lists.",
            Self::NothingAdmitted => "The template admits no value here.",
            Self::WrongScheme => "The Reference Model class requires another URI scheme.",
            Self::ZeroDenominator => "A proportion cannot have a denominator of zero.",
            Self::TooCoarse => "The template requires a finer value than that.",
            Self::TooFine => "The template does not admit a value that fine.",
            Self::Malformed => "That is not a value of the shape the template states.",
        }
    }
}

/// Whether `value` sits inside `range`, honouring each end's inclusivity.
pub(crate) fn within<T: PartialOrd>(range: &Range<T>, value: &T) -> bool {
    if let Some(low) = range.minimum.as_ref() {
        let admitted = if range.minimum_included {
            value >= low
        } else {
            value > low
        };
        if !admitted {
            return false;
        }
    }
    if let Some(high) = range.maximum.as_ref() {
        let admitted = if range.maximum_included {
            value <= high
        } else {
            value < high
        };
        if !admitted {
            return false;
        }
    }
    true
}

/// Whether the ranges the template states admit `value`.
///
/// Several ranges are alternatives, so a value any one of them admits is
/// admitted. No range at all constrains nothing.
pub(crate) fn within_any<T: PartialOrd>(ranges: &[Range<T>], value: &T) -> bool {
    ranges.is_empty() || ranges.iter().any(|range| within(range, value))
}

/// The decimal places plain decimal text states.
///
/// `None` where the text carries an exponent, which states a magnitude
/// without stating a decimal place count. No specification governs the input
/// syntax a control accepts: our own design, and the same value is still
/// enterable written out.
pub(crate) fn decimals(text: &str) -> Option<i64> {
    if text.contains(['e', 'E']) {
        return None;
    }
    let stated = text
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.chars().count());
    i64::try_from(stated).ok()
}

/// Whether the enumeration a text constraint states admits `typed`.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.3 makes a list the
/// whole permitted set unless `C_STRING.list_open` says otherwise, so an open
/// list is a suggestion and a closed one is exhaustive. A constraint that
/// lists nothing enumerates nothing and admits everything.
pub(crate) fn text_enumeration_admits(field: &TextField, typed: &str) -> bool {
    if !field.options_closed || field.options.is_empty() {
        return true;
    }
    field.options.iter().any(|option| option == typed)
}

/// The one regular expression the HTML `pattern` attribute takes.
///
/// Several patterns become one alternation, because the attribute takes a
/// single expression. `None` where the template states none. The patterns
/// arrive with ADL 2's delimiters already stripped.
pub(crate) fn pattern_attribute(patterns: &[String]) -> Option<String> {
    match patterns {
        [] => None,
        [only] => Some(only.clone()),
        many => Some(
            many.iter()
                .map(|pattern| format!("(?:{pattern})"))
                .collect::<Vec<_>>()
                .join("|"),
        ),
    }
}

/// The scheme a URI states, where it states one.
///
/// RFC 3986 section 3.1 spells a scheme as `ALPHA *( ALPHA / DIGIT / "+" /
/// "-" / "." )` before the first colon, and says a scheme is compared
/// case-insensitively, so the answer is lowercased.
pub(crate) fn uri_scheme(uri: &str) -> Option<String> {
    let (scheme, _) = uri.split_once(':')?;
    let mut characters = scheme.chars();
    if !characters.next()?.is_ascii_alphabetic() {
        return None;
    }
    if !characters.all(|part| part.is_ascii_alphanumeric() || matches!(part, '+' | '-' | '.')) {
        return None;
    }
    Some(scheme.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::TextField;
    use ferrochart_form::range::Range;

    use super::{
        decimals, pattern_attribute, text_enumeration_admits, uri_scheme, within, within_any,
    };

    #[test]
    fn an_excluded_end_is_refused_and_an_included_one_is_admitted() {
        let closed = Range::new(Some(0_i64), Some(10), true, true);
        assert!(within(&closed, &0));
        assert!(within(&closed, &10));
        let open = Range::new(Some(0_i64), Some(10), false, false);
        assert!(!within(&open, &0));
        assert!(!within(&open, &10));
        assert!(within(&open, &5));
    }

    #[test]
    fn a_value_outside_every_range_is_refused_and_no_range_admits_everything() {
        let ranges = [
            Range::new(Some(0_i64), Some(4), true, true),
            Range::new(Some(8_i64), Some(9), true, true),
        ];
        assert!(within_any(&ranges, &3));
        assert!(within_any(&ranges, &8));
        assert!(!within_any(&ranges, &6));
        assert!(within_any::<i64>(&[], &1_000_000));
    }

    #[test]
    fn the_decimal_places_are_the_digits_after_the_point() {
        assert_eq!(decimals("120"), Some(0));
        assert_eq!(decimals("120.5"), Some(1));
        assert_eq!(decimals("120.50"), Some(2));
        assert_eq!(decimals("1e3"), None);
    }

    #[test]
    fn a_closed_list_is_the_whole_set_and_an_open_one_is_a_suggestion() {
        let closed = TextField {
            patterns: Vec::new(),
            options: vec!["sitting".to_owned(), "standing".to_owned()],
            options_closed: true,
        };
        assert!(text_enumeration_admits(&closed, "sitting"));
        assert!(!text_enumeration_admits(&closed, "lying"));

        let open = TextField {
            options_closed: false,
            ..closed.clone()
        };
        assert!(text_enumeration_admits(&open, "lying"));

        let nothing_listed = TextField {
            options: Vec::new(),
            ..closed
        };
        assert!(text_enumeration_admits(&nothing_listed, "anything"));
    }

    #[test]
    fn several_patterns_become_one_alternation() {
        assert_eq!(pattern_attribute(&[]), None);
        assert_eq!(
            pattern_attribute(&["[0-9]{4}".to_owned()]),
            Some("[0-9]{4}".to_owned())
        );
        assert_eq!(
            pattern_attribute(&["[0-9]{4}".to_owned(), "[A-Z]{2}".to_owned()]),
            Some("(?:[0-9]{4})|(?:[A-Z]{2})".to_owned())
        );
    }

    #[test]
    fn a_scheme_is_read_case_insensitively_and_only_where_it_is_well_formed() {
        assert_eq!(uri_scheme("EHR://node/path"), Some("ehr".to_owned()));
        assert_eq!(
            uri_scheme("https://example.invalid"),
            Some("https".to_owned())
        );
        assert_eq!(uri_scheme("no-colon-here"), None);
        assert_eq!(uri_scheme("9tel:1"), None, "a scheme starts with a letter");
    }
}
