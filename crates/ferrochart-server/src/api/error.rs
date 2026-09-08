// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the HTTP surface answers when it cannot answer with a result.
//!
//! No specification governs this: our own design. openEHR ITS-REST
//! Release-1.1.0 defines a CDR's API and says nothing about the API of a form
//! server in front of one, so the statuses and the error body below are
//! FerroCHART's.
//!
//! Every failure carries its cause through `#[source]`, and an upstream
//! failure carries the CDR's own status and body outward rather than being
//! flattened into a default (`.claude/rules/reliability.md`).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ferrochart_cdr::error::CdrError;
use ferrochart_compose::error::{BuildError, ReadError};
use ferrochart_form::key::NodeKey;
use ferrochart_form::validation::{
    FailureKind, FailureSource, ValidationFailure, ValidationReport,
};
use ferrochart_validate::error::ValidateError;

use crate::commit::CommitError;

/// Why a request did not get the result it asked for.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub(crate) enum ApiError {
    /// The server holds no template of that identifier.
    #[error("this server holds no template `{template_id}`")]
    UnknownTemplate {
        /// The identifier the path named.
        template_id: String,
    },

    /// The request body is not the document the route reads.
    #[error("the request body is not the document this route reads")]
    MalformedBody {
        /// Why it did not read.
        #[source]
        source: serde_json::Error,
    },

    /// The body states a format version this server does not publish.
    #[error("this server publishes format version {expected} and the body states {found}")]
    UnsupportedFormat {
        /// The version the body stated.
        found: u32,
        /// The version this server publishes.
        expected: u32,
    },

    /// A required query parameter is absent.
    #[error("the `{name}` query parameter is required")]
    MissingQuery {
        /// The parameter's name.
        name: &'static str,
    },

    /// The entered values do not conform, so nothing was written.
    #[error("the entered values do not conform to the template ({} failures)", report.failures.len())]
    Refused {
        /// Every failure, keyed onto the form where its path resolved.
        report: Box<ValidationReport>,
    },

    /// The values could not be judged at all.
    #[error("the entered values could not be judged against the template")]
    Unjudgeable {
        /// Why the validator could not run.
        #[source]
        source: Box<ValidateError>,
    },

    /// The CDR refused a document FerroCHART built and validated.
    #[error("the CDR refused a COMPOSITION FerroCHART validated")]
    CdrRejected {
        /// What the CDR said.
        #[source]
        source: Box<CdrError>,
        /// The CDR's error body, read onto the form where it could be.
        diagnosis: Box<ValidationReport>,
    },

    /// The call into the CDR did not produce a result.
    #[error("the request to the CDR did not succeed")]
    CdrUnreachable {
        /// What went wrong.
        #[source]
        source: Box<CdrError>,
    },

    /// The CDR reports the composition as deleted.
    #[error("the CDR reports composition `{uid}` as deleted")]
    CompositionDeleted {
        /// The identifier the path named.
        uid: String,
    },

    /// The stored composition does not read back into this form.
    #[error("the stored COMPOSITION does not read back into this form")]
    Unreadable {
        /// Why it did not read back.
        #[source]
        source: Box<ReadError>,
    },
}

impl ApiError {
    /// The status this failure answers with.
    fn status(&self) -> StatusCode {
        match *self {
            Self::UnknownTemplate { .. } => StatusCode::NOT_FOUND,
            Self::MalformedBody { .. }
            | Self::UnsupportedFormat { .. }
            | Self::MissingQuery { .. } => StatusCode::BAD_REQUEST,
            Self::Refused { .. } | Self::Unreadable { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::CompositionDeleted { .. } => StatusCode::GONE,
            Self::Unjudgeable { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::CdrRejected { .. } | Self::CdrUnreachable { .. } => StatusCode::BAD_GATEWAY,
        }
    }

    /// The stable identifier a client branches on.
    fn code(&self) -> &'static str {
        match *self {
            Self::UnknownTemplate { .. } => "unknown_template",
            Self::MalformedBody { .. } => "malformed_body",
            Self::UnsupportedFormat { .. } => "unsupported_format",
            Self::MissingQuery { .. } => "missing_query_parameter",
            Self::Refused { .. } => "refused",
            Self::Unjudgeable { .. } => "unjudgeable",
            Self::CdrRejected { .. } => "cdr_rejected",
            Self::CdrUnreachable { .. } => "cdr_unreachable",
            Self::CompositionDeleted { .. } => "composition_deleted",
            Self::Unreadable { .. } => "unreadable_composition",
        }
    }
}

impl IntoResponse for ApiError {
    /// Renders the failure as the one error document this surface publishes.
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let mut body = serde_json::Map::new();
        body.insert("error".to_owned(), serde_json::json!(code));
        body.insert("message".to_owned(), serde_json::json!(self.to_string()));
        match self {
            Self::Refused { report } => {
                body.insert("report".to_owned(), serde_json::json!(report));
            }
            Self::CdrRejected {
                ref source,
                diagnosis,
            } => {
                add_upstream(&mut body, source);
                body.insert("report".to_owned(), serde_json::json!(diagnosis));
            }
            Self::CdrUnreachable { ref source } => add_upstream(&mut body, source),
            _ => {}
        }
        // The body can carry what a CDR said about clinical content, so the
        // log takes the shape and never the text.
        tracing::warn!(status = %status, error = code, "the request did not succeed");
        (status, Json(serde_json::Value::Object(body))).into_response()
    }
}

/// Puts what the CDR answered into the error document.
///
/// `cdr_status` is null where the call never reached an answer, which is the
/// honest report of a transport failure: inventing a status the CDR never
/// sent would be the flattening `.claude/rules/reliability.md` refuses.
fn add_upstream(body: &mut serde_json::Map<String, serde_json::Value>, source: &CdrError) {
    let (status, text) = upstream_answer(source);
    let named = match status {
        Some(status) => serde_json::json!(status.as_u16()),
        None => serde_json::Value::Null,
    };
    body.insert("cdr_status".to_owned(), named);
    body.insert("cdr_body".to_owned(), serde_json::json!(text));
}

/// The status and the text the CDR answered with, where it answered.
///
/// `None` says the call never reached an answer, which a transport failure and
/// an unbuildable URL both are. Neither is flattened into a status the CDR
/// never sent.
fn upstream_answer(error: &CdrError) -> (Option<StatusCode>, String) {
    match *error {
        CdrError::BadRequest { ref body } => (Some(StatusCode::BAD_REQUEST), body.clone()),
        CdrError::Unprocessable { ref body } => {
            (Some(StatusCode::UNPROCESSABLE_ENTITY), body.clone())
        }
        CdrError::Conflict { ref body, .. } => (Some(StatusCode::CONFLICT), body.clone()),
        CdrError::NotFound { .. } => (Some(StatusCode::NOT_FOUND), error.to_string()),
        CdrError::ConcurrentEdit { .. } => {
            (Some(StatusCode::PRECONDITION_FAILED), error.to_string())
        }
        CdrError::Unauthorized { status, .. } | CdrError::UnnamedVersion { status } => {
            (Some(status), error.to_string())
        }
        CdrError::UnsupportedMediaType { status, .. } => (Some(status), error.to_string()),
        CdrError::Unimplemented { status, ref body }
        | CdrError::Unavailable { status, ref body }
        | CdrError::Upstream { status, ref body } => (Some(status), body.clone()),
        _ => (None, error.to_string()),
    }
}

/// One commit outcome, as the failure the client is shown.
///
/// A build refusal becomes a report too, so a required field left empty
/// reaches the renderer on the field rather than as prose it cannot place.
pub(crate) fn from_commit(error: CommitError) -> ApiError {
    match error {
        CommitError::Build { source } => ApiError::Refused {
            report: Box::new(build_report(&source)),
        },
        CommitError::Refused { report } => ApiError::Refused { report },
        CommitError::Judge { source } => ApiError::Unjudgeable {
            source: Box::new(source),
        },
        CommitError::Rejected { source, diagnosis } => ApiError::CdrRejected { source, diagnosis },
        CommitError::Wire { source } => ApiError::CdrUnreachable { source },
    }
}

/// A builder refusal, keyed onto the field it names.
///
/// No specification governs this mapping: our own design. Every variant that
/// carries a [`NodeKey`] resolves onto the form, and the two that carry a code
/// instead are reported unplaced rather than dropped.
pub(crate) fn build_report(error: &BuildError) -> ValidationReport {
    let (key, kind) = match *error {
        BuildError::UnknownField { ref key } => (Some(key.clone()), FailureKind::Unexpected),
        BuildError::WrongDatum { ref key, .. } => (Some(key.clone()), FailureKind::WrongType),
        BuildError::Missing { ref key } => (Some(key.clone()), FailureKind::Required),
        BuildError::Occurrences { ref key, .. } => (Some(key.clone()), FailureKind::Occurrences),
        BuildError::Invariant { ref key, .. } => (Some(key.clone()), FailureKind::Invariant),
        BuildError::UnknownNullFlavour { .. } | BuildError::UnknownCategory { .. } => {
            (None, FailureKind::Terminology)
        }
        _ => (None, FailureKind::Other),
    };
    ValidationReport {
        failures: vec![ValidationFailure {
            path: key.as_ref().map_or_else(String::new, NodeKey::to_string),
            key,
            message: error.to_string(),
            kind,
            source: FailureSource::Builder,
        }],
    }
}
