// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The headers that decide what a write does, and what a client must strip
//! before it compares one.
//!
//! All citations are openEHR ITS-REST Release-1.1.0 `overview.html`.

use ferrochart_cdr::header::{self, Prefer};
use ferrochart_cdr::ids::VersionUid;

const EXAMPLE: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2";

#[test]
fn if_match_quotes_the_version_and_does_not_weaken_it() {
    // "The format is always an `version_uid` identifier enclosed by double
    // quotes." The request form carries no `W/`, which the response form
    // must, so the two spellings are not interchangeable.
    let value = header::if_match(&VersionUid::new(EXAMPLE));
    assert_eq!(value, format!("\"{EXAMPLE}\""));
    assert!(!value.starts_with("W/"), "an If-Match is never weak");
}

#[test]
fn both_etag_spellings_yield_the_same_version() {
    // Release-1.1.0 requires the `W/` prefix, Release-1.0.3 had none, and
    // 1.1.0 still permits its absence, so a client that reads only one finds
    // the versions unequal against one release or the other.
    let expected = Some(VersionUid::new(EXAMPLE));
    for spelling in [
        format!("W/\"{EXAMPLE}\""),
        format!("\"{EXAMPLE}\""),
        format!("  W/\"{EXAMPLE}\"  "),
    ] {
        assert_eq!(
            header::version_of_etag(&spelling),
            expected,
            "the ETag {spelling} did not read as the version it names"
        );
    }
}

#[test]
fn an_unquoted_etag_is_refused_rather_than_guessed_at() {
    // The quotes have been required since Release-1.0.2, and the
    // specification permits an ETag that is "an opaque quoted string" rather
    // than an identifier at all. Guessing would propagate whatever the header
    // held into the next If-Match, which is a write against the wrong version.
    assert_eq!(header::version_of_etag(EXAMPLE), None);
    assert_eq!(header::version_of_etag(&format!("W/{EXAMPLE}")), None);
    // The published `ETag_COMPOSITION` example is itself missing its closing
    // quote, so this exact string reaches a client that copies the spec.
    assert_eq!(header::version_of_etag(&format!("W/\"{EXAMPLE}")), None);
}

#[test]
fn a_version_uid_names_the_object_it_versions() {
    // openEHR RM Release-1.1.0 common.html section 4.3: an OBJECT_VERSION_ID
    // is the object uid, the creating system id and the version tree id,
    // joined by `::`. An update addresses the object, not the version.
    let version = VersionUid::new(EXAMPLE);
    assert_eq!(
        version.object().as_str(),
        "8849182c-82ad-4088-a07f-48ead4180515"
    );
}

#[test]
fn an_unseparated_uid_names_itself() {
    // A CDR that issued a uid with no separator has named the object with it,
    // so splitting yields the whole string rather than nothing.
    let version = VersionUid::new("no-separator-here");
    assert_eq!(version.object().as_str(), "no-separator-here");
}

#[test]
fn prefer_spells_all_three_values_the_specification_defines() {
    assert_eq!(Prefer::Minimal.as_str(), "return=minimal");
    assert_eq!(Prefer::Identifier.as_str(), "return=identifier");
    assert_eq!(Prefer::Representation.as_str(), "return=representation");
}
