// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Putting an identifier into a URL without letting it change the URL.
//!
//! A template identifier is free text: openEHR AM Release-2.3.0 `AOM2.html`
//! section 4.1.4 constrains the archetype identifier and nothing constrains
//! the template's, and the committed corpus carries identifiers with spaces
//! in them. Concatenating one into an address puts its tail in the next
//! segment or in the query string, so every caller-supplied part goes through
//! this module.
//!
//! Two consumers: [`crate::api::route`] builds the server's addresses, and
//! [`crate::nav`] builds the renderer's own.

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

/// What a path segment may not carry.
///
/// RFC 3986 section 3.3 spells a segment as `pchar`s, which admits the
/// sub-delimiters and `:` and `@`. This set is the complement a caller can
/// actually produce: the controls, the space, the delimiter that ends a
/// segment (`/`), the two that end a path (`?` and `#`), the four the URL
/// standard forbids in a path (`"`, `<`, `>`, `` ` ``), and `%` itself, so an
/// identifier that already reads like an escape is not decoded twice.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`');

/// What a query value may not carry.
///
/// The segment set plus the three characters that separate one parameter from
/// the next, from its value, or that decode as a space.
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
const QUERY_VALUE: &AsciiSet = &SEGMENT.add(b'&').add(b'+').add(b'=');

/// `raw` as one path segment.
pub(crate) fn segment(raw: &str) -> String {
    utf8_percent_encode(raw, SEGMENT).to_string()
}

/// `raw` as one query value.
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
pub(crate) fn query_value(raw: &str) -> String {
    utf8_percent_encode(raw, QUERY_VALUE).to_string()
}

#[cfg(test)]
mod tests {
    use super::{query_value, segment};

    #[test]
    fn a_space_never_reaches_the_address() {
        assert_eq!(
            segment("IDCR - Adverse Reaction List.v1"),
            "IDCR%20-%20Adverse%20Reaction%20List.v1"
        );
    }

    #[test]
    fn a_separator_in_an_identifier_never_becomes_a_path_step() {
        assert_eq!(segment("a/b?c#d"), "a%2Fb%3Fc%23d");
    }

    #[test]
    fn an_already_escaped_identifier_is_not_decoded_twice() {
        // A literal `%20` in an identifier is five characters, and the server
        // has to read back those five rather than a space.
        assert_eq!(segment("a%20b"), "a%2520b");
    }

    #[test]
    fn a_version_uid_keeps_the_colons_that_name_its_version() {
        // openEHR RM Release-1.1.0 `common.html` section 4.3 joins the three
        // parts of an OBJECT_VERSION_ID with `::`, and RFC 3986 section 3.3
        // admits `:` in a segment, so the identifier survives unaltered.
        assert_eq!(segment("f0a1::ferro.example::1"), "f0a1::ferro.example::1");
    }

    #[test]
    fn a_query_value_gives_up_the_three_characters_a_segment_keeps() {
        assert_eq!(query_value("a&b=c+d"), "a%26b%3Dc%2Bd");
        assert_eq!(segment("a&b=c+d"), "a&b=c+d");
    }

    #[test]
    fn text_outside_ascii_travels_as_its_utf8_bytes() {
        assert_eq!(segment("bloedøruk"), "bloed%C3%B8ruk");
    }
}
