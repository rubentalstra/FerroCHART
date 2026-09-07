// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What can go wrong on the wire, as types a caller can branch on.
//!
//! Nothing here flattens an upstream failure into an empty value or a default.
//! A refusal, a timeout and a partial write are each an error carrying what the
//! CDR said (`docs/architecture.md` section 8,
//! `.claude/rules/reliability.md`).
//!
//! The variants cover the whole status subset openEHR ITS-REST Release-1.1.0
//! `overview.html` publishes, and one catch-all, because that section also
//! says "Additional status codes MAY be used as long as they do not conflict
//! with the predefined codes". An exhaustive match with no catch-all would be
//! a specification violation dressed up as a compile-time guarantee.

use http::StatusCode;
use thiserror::Error;

use crate::ids::VersionUid;

/// A failure of one ITS-REST call.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CdrError {
    /// The base URL and the path did not join into a URL.
    #[error("the CDR base URL and the path `{path}` do not join into a URL")]
    Url {
        /// The path that was being appended.
        path: String,
        /// Why it did not join.
        #[source]
        source: url::ParseError,
    },

    /// The base URL cannot carry a path, so no endpoint can be built from it.
    #[error("the CDR base URL `{base}` cannot carry a path")]
    OpaqueBase {
        /// The URL as configured.
        base: String,
    },

    /// The request never got an answer: a connection failure, a timeout, or a
    /// body that could not be read.
    #[error("the request to the CDR failed")]
    Transport {
        /// Why the request failed.
        #[source]
        source: reqwest::Error,
    },

    /// The CDR answered, and the body was not what the call expects.
    #[error("the CDR's response body is not {expected}")]
    Body {
        /// What the call was reading, for example "a COMPOSITION".
        expected: &'static str,
        /// Why it could not be read.
        #[source]
        source: serde_json::Error,
    },

    /// The write succeeded and the CDR named no version for it.
    ///
    /// The `ETag` and the `Location` headers are both `SHOULD` rather than
    /// `MUST`, and a `return=identifier` body is a `SHOULD` too, so all three
    /// can be absent from a response that is otherwise conformant. The
    /// document is stored and the client cannot address it, which is a
    /// half-success and is reported as one rather than as a success.
    #[error("the CDR's {status} response names no version of the document it stored")]
    UnnamedVersion {
        /// The status it answered with.
        status: StatusCode,
    },

    /// The COMPOSITION has moved on since the version the update named.
    ///
    /// A concurrent edit. openEHR ITS-REST Release-1.1.0 `overview.html`
    /// requires the server not to perform the write, and the specification
    /// prescribes no client recovery, so the recovery is FerroCHART's own
    /// design (`docs/architecture.md` section 8): the form shows it to the
    /// clinician, who decides. Retrying with `latest` would overwrite whatever
    /// the other writer stored, which is the exact failure `If-Match` exists
    /// to prevent.
    #[error("the composition has changed since version {expected} was read")]
    ConcurrentEdit {
        /// The version the update sent in `If-Match`.
        expected: VersionUid,
        /// The version the CDR now holds, where its 412 named one. The
        /// `ETag` on a 412 is a `SHOULD`, so it can be absent.
        latest: Option<VersionUid>,
    },

    /// The CDR has no such EHR, composition, or template.
    ///
    /// On an update this status means the EHR or the composition, and the CDR
    /// does not say which, so `what` records what the call was addressing
    /// rather than claiming to know.
    #[error("the CDR has no {what} `{id}`")]
    NotFound {
        /// The kind of thing, for example "composition".
        what: &'static str,
        /// What was asked for.
        id: String,
    },

    /// The request was malformed, or a precondition the CDR expects was
    /// missing.
    ///
    /// openEHR ITS-REST Release-1.1.0 `overview.html`: "The client SHOULD NOT
    /// repeat the request without modifications." An update also answers 400
    /// when `If-Match` is absent, so the body is what tells the two apart.
    #[error("the CDR refused the request as malformed: {body}")]
    BadRequest {
        /// The response body, verbatim.
        body: String,
    },

    /// The CDR refused the document as semantically invalid.
    ///
    /// 422: "the content type and syntax is correct, could be converted to a
    /// resource, but there are semantic validation errors, such as the
    /// underlying template is not known or is not validating the supplied
    /// resource".
    ///
    /// A CDR rejecting a COMPOSITION FerroCHART built and validated is always
    /// a FerroCHART bug (`CLAUDE.md`), so this is triaged as a compiler defect
    /// rather than as user error, and the body is kept verbatim for the
    /// report.
    #[error("the CDR refused the document as invalid: {body}")]
    Unprocessable {
        /// The response body, verbatim.
        body: String,
    },

    /// The resource already exists, or the identifier does not name the
    /// latest version.
    ///
    /// On a template upload this is the ordinary outcome of uploading a
    /// template the CDR already holds, which is not a failure of the client.
    #[error("the CDR reports a conflict: {body}")]
    Conflict {
        /// The response body, verbatim.
        body: String,
        /// The version the CDR holds, where it named one.
        latest: Option<VersionUid>,
    },

    /// The request carried no valid credentials, or the ones it carried are
    /// not permitted to do this.
    ///
    /// openEHR ITS-REST Release-1.1.0 mandates no authentication scheme, so
    /// the challenge the CDR sent is carried rather than interpreted.
    #[error("the CDR refused the request with {status}")]
    Unauthorized {
        /// 401, 403 or 407.
        status: StatusCode,
        /// The `WWW-Authenticate` or `Proxy-Authenticate` challenge, where the
        /// CDR sent one.
        challenge: Option<String>,
    },

    /// The CDR does not offer this call, or does not offer it this way.
    ///
    /// 405 when the method is not allowed on the path and 501 when the CDR
    /// does not implement it at all. Either means the CDR is conformant and
    /// simply does not do this, which is not a defect: a CDR may implement one
    /// ADL generation and not the other.
    #[error("the CDR does not implement this call ({status})")]
    Unimplemented {
        /// 405 or 501.
        status: StatusCode,
        /// The response body, verbatim.
        body: String,
    },

    /// The CDR cannot produce what the request asked for, or cannot read what
    /// it sent.
    ///
    /// 406 for the `Accept` and 415 for the `Content-Type`. These are
    /// capability answers rather than failures: a CDR that serves only XML, or
    /// that offers no web template, is conformant.
    #[error("the CDR does not support the {what} the request named ({status})")]
    UnsupportedMediaType {
        /// 406 or 415.
        status: StatusCode,
        /// `"Accept"` or `"Content-Type"`.
        what: &'static str,
        /// The media type that was asked for.
        media_type: String,
    },

    /// The CDR gave up on the request, or failed inside itself.
    ///
    /// 408 and 500. Both may succeed on a later attempt, and neither is
    /// retried here: a write that timed out may have been applied, so a retry
    /// is the caller's decision with the caller's knowledge of what it sent.
    #[error("the CDR answered {status}: {body}")]
    Unavailable {
        /// 408 or 500.
        status: StatusCode,
        /// The response body, verbatim.
        body: String,
    },

    /// Any other status the CDR answered with.
    ///
    /// openEHR ITS-REST Release-1.1.0 permits statuses beyond the subset it
    /// publishes, so this variant is required rather than defensive.
    #[error("the CDR answered {status}: {body}")]
    Upstream {
        /// The status.
        status: StatusCode,
        /// The response body, verbatim.
        body: String,
    },
}
