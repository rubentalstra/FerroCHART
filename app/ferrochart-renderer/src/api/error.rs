// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Why a request to the form surface did not produce its result.
//!
//! Every variant carries the status the server answered with and the bytes it
//! answered with, and every wrapping variant carries its cause through
//! `#[source]`. Nothing here turns a failure into an empty value, a default,
//! or a `None` that loses the reason: `.claude/rules/reliability.md` names
//! that the silent-wrongness path this project exists to prevent, and a
//! browser is the last place a refusal may go missing.
//!
//! No specification governs the error document these variants read: it is the
//! one `ferrochart-server` publishes, whose `error` member is the stable code
//! a client branches on and whose `cdr_status` is null where the call into
//! the CDR never reached an answer.

use ferrochart_form::validation::ValidationReport;
use serde::Deserialize;

/// The error document the form surface answers a failure with.
///
/// Unknown members are kept out of the type rather than refused, so a member
/// the server adds later does not turn a readable refusal into an
/// unreadable one.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct Refusal {
    /// The stable identifier a client branches on, verbatim from the
    /// document's `error` member.
    pub(crate) error: String,
    /// What went wrong, in prose, for a reader rather than for a branch.
    pub(crate) message: String,
    /// Every failure keyed onto the form, where the refusal is a judgement.
    pub(crate) report: Option<ValidationReport>,
    /// The status the CDR answered with, where the failure came from one.
    ///
    /// `None` covers both an absent member and an explicit null, and the
    /// null is the server's honest report that the call never reached an
    /// answer.
    pub(crate) cdr_status: Option<u16>,
    /// The body the CDR answered with, where the failure came from one.
    pub(crate) cdr_body: Option<String>,
}

/// Why one request did not produce its result.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub(crate) enum ApiError {
    /// The request never reached an answer, so there is no status to report.
    #[error("the request to `{url}` never reached an answer")]
    Unreachable {
        /// The address that was asked.
        url: String,
        /// What the browser said went wrong.
        #[source]
        source: gloo_net::Error,
    },

    /// The body of a request could not be written, so nothing was sent.
    #[error("the body for `{url}` could not be written")]
    #[expect(
        dead_code,
        reason = "only a request with a body reaches it, and the controls that make one are issue #137"
    )]
    Unsendable {
        /// The address that would have been asked.
        url: String,
        /// Where the write stopped.
        #[source]
        source: serde_json::Error,
    },

    /// The server refused, and said why in the document it publishes.
    #[error("`{url}` answered {status}: {}", refusal.message)]
    Refused {
        /// The address that was asked.
        url: String,
        /// The status it answered with.
        status: u16,
        /// What it said.
        refusal: Box<Refusal>,
    },

    /// The server refused with a body that is not that document.
    #[error("`{url}` answered {status} with a body this client does not read")]
    Unexplained {
        /// The address that was asked.
        url: String,
        /// The status it answered with.
        status: u16,
        /// The body, verbatim, because it is the only account of the refusal
        /// that survives.
        body: String,
    },

    /// The server answered with a result that is not the document the route
    /// publishes.
    #[error("the body `{url}` answered with is not the document this route reads")]
    Malformed {
        /// The address that was asked.
        url: String,
        /// The status it answered with.
        status: u16,
        /// The body, verbatim.
        body: String,
        /// Where the read stopped.
        #[source]
        source: serde_json::Error,
    },

    /// The answer arrived and its body could not be read at all.
    #[error("the body `{url}` answered with could not be read")]
    Unreadable {
        /// The address that was asked.
        url: String,
        /// The status it answered with.
        status: u16,
        /// What the browser said went wrong.
        #[source]
        source: gloo_net::Error,
    },

    /// The document states a format version this client does not read.
    #[error("this client reads format version {expected} and `{url}` answered {found}")]
    UnsupportedFormat {
        /// The address that was asked.
        url: String,
        /// The status it answered with.
        status: u16,
        /// The version the document states.
        found: u32,
        /// The version this client reads.
        expected: u32,
    },
}

impl ApiError {
    /// The status the server answered with.
    ///
    /// `None` says the request never reached an answer, which is the honest
    /// report of a transport failure. Inventing a status the server never
    /// sent would be the flattening `.claude/rules/reliability.md` refuses.
    pub(crate) fn status(&self) -> Option<u16> {
        match *self {
            Self::Unreachable { .. } | Self::Unsendable { .. } => None,
            Self::Refused { status, .. }
            | Self::Unexplained { status, .. }
            | Self::Malformed { status, .. }
            | Self::Unreadable { status, .. }
            | Self::UnsupportedFormat { status, .. } => Some(status),
        }
    }

    /// The address that was asked.
    pub(crate) fn url(&self) -> &str {
        match *self {
            Self::Unreachable { ref url, .. }
            | Self::Unsendable { ref url, .. }
            | Self::Refused { ref url, .. }
            | Self::Unexplained { ref url, .. }
            | Self::Malformed { ref url, .. }
            | Self::Unreadable { ref url, .. }
            | Self::UnsupportedFormat { ref url, .. } => url,
        }
    }

    /// What the server said, where it said it in the document it publishes.
    pub(crate) fn refusal(&self) -> Option<&Refusal> {
        match *self {
            Self::Refused { ref refusal, .. } => Some(refusal),
            _ => None,
        }
    }

    /// The body the server answered with, where this client kept it verbatim.
    pub(crate) fn body(&self) -> Option<&str> {
        match *self {
            Self::Unexplained { ref body, .. } | Self::Malformed { ref body, .. } => Some(body),
            _ => None,
        }
    }

    /// Every failure the refusal keyed onto the form.
    #[allow(
        dead_code,
        reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
    )]
    pub(crate) fn report(&self) -> Option<&ValidationReport> {
        self.refusal()?.report.as_ref()
    }
}

/// The failure a status outside the success range becomes.
///
/// The body is read as the error document the surface publishes, and a body
/// that is not one becomes [`ApiError::Unexplained`] carrying those bytes.
/// The refusal is never reduced to its status: a judgement travels in the
/// document's `report`, and a CDR's own answer in `cdr_status` and
/// `cdr_body`.
pub(crate) fn refused(url: &str, status: u16, body: String) -> ApiError {
    match serde_json::from_str::<Refusal>(&body) {
        Ok(refusal) => ApiError::Refused {
            url: url.to_owned(),
            status,
            refusal: Box::new(refusal),
        },
        Err(_) => ApiError::Unexplained {
            url: url.to_owned(),
            status,
            body,
        },
    }
}

/// The document `body` carries, or the failure that says why it does not.
pub(crate) fn decoded<T: serde::de::DeserializeOwned>(
    url: &str,
    status: u16,
    body: &str,
) -> Result<T, ApiError> {
    serde_json::from_str(body).map_err(|source| ApiError::Malformed {
        url: url.to_owned(),
        status,
        body: body.to_owned(),
        source,
    })
}

/// Refuses a document written in a format version this client does not read.
pub(crate) fn format_checked(url: &str, status: u16, found: u32) -> Result<(), ApiError> {
    if found == ferrochart_form::definition::FORMAT_VERSION {
        return Ok(());
    }
    Err(ApiError::UnsupportedFormat {
        url: url.to_owned(),
        status,
        found,
        expected: ferrochart_form::definition::FORMAT_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use ferrochart_form::validation::FailureKind;

    use super::{ApiError, decoded, format_checked, refused};

    /// The error document the server publishes for a judgement, with one
    /// failure keyed onto a field.
    const REFUSED_BODY: &str = r#"{
        "error": "refused",
        "message": "the entered values do not conform to the template (1 failures)",
        "report": {
            "failures": [
                {
                    "key": null,
                    "path": "/content[at0001]",
                    "message": "a required node is absent",
                    "kind": "required",
                    "source": "template"
                }
            ]
        }
    }"#;

    /// The error document the server publishes when the CDR never answered.
    const UNREACHABLE_CDR_BODY: &str = r#"{
        "error": "cdr_unreachable",
        "message": "the request to the CDR did not succeed",
        "cdr_status": null,
        "cdr_body": "the connection was refused"
    }"#;

    #[test]
    fn a_refusal_reaches_the_caller_as_a_code_and_a_message_rather_than_prose() {
        let error = refused("/api/templates/x/validation", 422, REFUSED_BODY.to_owned());
        let refusal = error.refusal().unwrap();
        assert_eq!(refusal.error, "refused");
        assert!(refusal.message.contains("do not conform"));
        assert_eq!(error.status(), Some(422));
        assert_eq!(error.url(), "/api/templates/x/validation");
    }

    #[test]
    fn a_judgement_travels_with_the_refusal_rather_than_being_dropped() {
        let error = refused("/api/templates/x/validation", 422, REFUSED_BODY.to_owned());
        let report = error.report().unwrap();
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.unplaced().count(), 1);
        let Some(failure) = report.failures.first() else {
            panic!("the report lost the failure it was built with");
        };
        assert_eq!(failure.kind, FailureKind::Required);
    }

    #[test]
    fn a_cdr_that_never_answered_carries_a_null_status_and_its_body() {
        let error = refused(
            "/api/ehrs/e/templates/t/compositions",
            502,
            UNREACHABLE_CDR_BODY.to_owned(),
        );
        let refusal = error.refusal().unwrap();
        assert_eq!(refusal.error, "cdr_unreachable");
        assert_eq!(refusal.cdr_status, None);
        assert_eq!(
            refusal.cdr_body.as_deref(),
            Some("the connection was refused")
        );
    }

    #[test]
    fn a_cdr_status_the_server_did_send_reaches_the_caller() {
        let body =
            r#"{"error":"cdr_rejected","message":"refused","cdr_status":422,"cdr_body":"{}"}"#;
        let error = refused("/api/x", 502, body.to_owned());
        assert_eq!(error.refusal().unwrap().cdr_status, Some(422));
    }

    #[test]
    fn a_refusal_this_client_cannot_read_keeps_its_bytes() {
        let error = refused("/api/templates", 503, "<html>gateway</html>".to_owned());
        assert!(error.refusal().is_none());
        assert_eq!(error.body(), Some("<html>gateway</html>"));
        assert_eq!(error.status(), Some(503));
    }

    #[test]
    fn a_result_this_client_cannot_read_keeps_its_bytes_and_its_cause() {
        let error = decoded::<u32>("/api/templates", 200, "not json").unwrap_err();
        assert_eq!(error.body(), Some("not json"));
        assert_eq!(error.status(), Some(200));
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn a_transport_failure_reports_no_status_rather_than_inventing_one() {
        let error = ApiError::Unreachable {
            url: "/api/templates".to_owned(),
            source: gloo_net::Error::GlooError("the connection was refused".to_owned()),
        };
        assert_eq!(error.status(), None);
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn a_document_in_another_format_version_is_refused_rather_than_guessed_at() {
        let error = format_checked("/api/templates", 200, 99).unwrap_err();
        assert!(matches!(
            error,
            ApiError::UnsupportedFormat { found: 99, .. }
        ));
        assert!(format_checked("/api/templates", 200, 1).is_ok());
    }
}
