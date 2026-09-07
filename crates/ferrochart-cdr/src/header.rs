// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The headers that decide what a write does and what it gives back.

use crate::ids::VersionUid;

/// What the client asks a write to return.
///
/// openEHR ITS-REST Release-1.1.0 `overview.html` warns that a server may be
/// configured to change the default, so FerroCHART sends the header on every
/// write rather than relying on one: "clients are encouraged to always include
/// the `Prefer` request header explicitly".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefer {
    /// `return=minimal`. The response body is empty, and an update answers
    /// 204 rather than 200.
    Minimal,
    /// `return=identifier`. The response carries only the identifier of the
    /// affected resource, which is what the next `If-Match` needs. The status
    /// is 201 or 200, "never 204 No Content". This is what a normal save asks
    /// for.
    Identifier,
    /// `return=representation`. The response carries the stored document.
    /// Asked for only when the form has to re-render from the server's
    /// canonicalisation, because it costs a whole document on every save.
    Representation,
}

impl Prefer {
    /// The header value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "return=minimal",
            Self::Identifier => "return=identifier",
            Self::Representation => "return=representation",
        }
    }
}

/// The `If-Match` value for an update replacing the version `preceding`.
///
/// openEHR ITS-REST Release-1.1.0 `overview.html`: "The format is always an
/// `version_uid` identifier enclosed by double quotes." The value carries no
/// `W/` prefix, which a response `ETag` does, so the two spellings are not
/// interchangeable and this is the only place the request form is built.
#[must_use]
pub fn if_match(preceding: &VersionUid) -> String {
    format!("\"{preceding}\"")
}

/// The version uid inside an `ETag`, or `None` where the header is not one.
///
/// openEHR ITS-REST Release-1.1.0 made the `ETag` weak: "all `ETag` headers
/// that hold a resource identifier MUST include a weakness indicator `W/`".
/// Release-1.0.3 had no prefix and 1.1.0 still permits its absence, so both
/// are read. The quotes have been required since Release-1.0.2 and an
/// unquoted value is refused rather than guessed at.
///
/// The specification also permits an `ETag` that is not an identifier at all:
/// "Servers MAY add additional `ETag` response headers, consisting of an
/// opaque quoted string". A caller that gets `None` reads the identifier from
/// the response body instead of propagating an opaque string into the next
/// `If-Match`.
#[must_use]
pub fn version_of_etag(etag: &str) -> Option<VersionUid> {
    unquote(etag).map(VersionUid::new)
}

/// A header value with its weak prefix and its quotes removed, or `None` where
/// it is not a quoted string.
#[must_use]
pub fn unquote(value: &str) -> Option<&str> {
    let value = value.trim();
    let value = value.strip_prefix("W/").unwrap_or(value);
    value.strip_prefix('"')?.strip_suffix('"')
}
