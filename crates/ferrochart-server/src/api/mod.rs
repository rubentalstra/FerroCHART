// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The HTTP surface: the templates a server holds, and what a clinician
//! enters against one.
//!
//! No specification governs any route here: our own design. openEHR ITS-REST
//! Release-1.1.0 defines the CDR's API, and a form server sitting in front of
//! one is outside it, so the paths, the bodies and the statuses are
//! FerroCHART's own. The documents the bodies carry are not: a form definition
//! and a set of entered values are `ferrochart-form`'s published contract, and
//! this surface transports them unchanged.
//!
//! Every request body and every response body states
//! [`ferrochart_form::definition::FORMAT_VERSION`], so a client reading one
//! can tell which version of that contract it is holding, and a body stating
//! another version is refused rather than guessed at.

pub(crate) mod compositions;
pub(crate) mod error;
pub(crate) mod templates;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use ferrochart_cdr::client::CdrClient;
use ferrochart_form::definition::FORMAT_VERSION;
use ferrochart_form::envelope::Envelope;
use ferrochart_form::values::FormValues;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::store::TemplateStore;

/// What every handler shares: the templates, and the CDR to commit to.
#[derive(Debug, Clone)]
pub struct ServerState {
    /// The compiled templates this server serves.
    templates: Arc<TemplateStore>,
    /// The client for the configured CDR.
    cdr: CdrClient,
}

impl ServerState {
    /// Holds `templates` and commits through `cdr`.
    #[must_use]
    pub fn new(templates: Arc<TemplateStore>, cdr: CdrClient) -> Self {
        Self { templates, cdr }
    }

    /// The templates this server serves.
    #[must_use]
    pub fn templates(&self) -> &TemplateStore {
        &self.templates
    }
}

/// The routes of the form surface, over `state`.
pub fn routes(state: ServerState) -> Router {
    Router::new()
        .route("/api/templates", get(templates::list))
        .route(
            "/api/templates/{template_id}/definition",
            get(templates::definition),
        )
        .route(
            "/api/templates/{template_id}/validation",
            post(compositions::validation),
        )
        .route(
            "/api/ehrs/{ehr_id}/templates/{template_id}/compositions",
            post(compositions::create),
        )
        .route(
            "/api/ehrs/{ehr_id}/compositions/{uid}/values",
            get(compositions::values),
        )
        .with_state(state)
}

/// What a clinician entered, and the Reference Model values a form never
/// shows.
///
/// The envelope travels with the values because most of it is per-session:
/// openEHR RM Release-1.1.0 `ehr.html` section 5.2.2 makes
/// `COMPOSITION.composer` mandatory and nothing authorises a server to invent
/// it, and `EVENT_CONTEXT.start_time` is the moment the clinician recorded.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Submission {
    /// The format version the body is written in.
    format_version: u32,
    /// The Reference Model values a form never shows.
    envelope: Envelope,
    /// The entered values.
    values: FormValues,
}

impl Submission {
    /// Reads a submission out of a request body.
    ///
    /// The body is parsed here rather than through an extractor so a
    /// malformed one answers with the error document this surface publishes
    /// and carries the parse failure as its cause.
    fn read(body: &[u8]) -> Result<Self, ApiError> {
        let submission: Self =
            serde_json::from_slice(body).map_err(|source| ApiError::MalformedBody { source })?;
        if submission.format_version == FORMAT_VERSION {
            return Ok(submission);
        }
        Err(ApiError::UnsupportedFormat {
            found: submission.format_version,
            expected: FORMAT_VERSION,
        })
    }
}
