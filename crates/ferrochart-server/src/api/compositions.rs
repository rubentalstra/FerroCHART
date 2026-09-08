// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Judging what a clinician entered, committing it, and reading it back.
//!
//! The three routes share one gate. [`crate::commit::Commit`] is the only path
//! in this workspace from a clinician's entries to a CDR write, so the
//! validation route is the same gate with the request left unmade.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use ferrochart_cdr::ids::{CompositionId, EhrId, VersionUid, VersionedObjectUid};
use ferrochart_compose::read;
use ferrochart_form::definition::FORMAT_VERSION;
use ferrochart_form::validation::ValidationReport;
use ferrochart_form::values::FormValues;
use serde::{Deserialize, Serialize};

use crate::api::error::{self, ApiError};
use crate::api::{ServerState, Submission};
use crate::commit::{Commit, CommitError};
use crate::store::HeldTemplate;

/// The separator between the object uid and the rest of a version uid.
///
/// openEHR RM Release-1.1.0 `common.html` section 4.3 spells an
/// `OBJECT_VERSION_ID` as the object uid, the creating system id and the
/// version tree id joined by `::`, so an identifier carrying it names one
/// version and one without it names the versioned object.
const VERSION_SEPARATOR: &str = "::";

/// What the judgement answered.
#[derive(Debug, Serialize)]
pub(crate) struct Judgement {
    /// The format version the report is written in.
    format_version: u32,
    /// Whether the entries may be committed.
    valid: bool,
    /// Every failure, keyed onto the form where its path resolved.
    report: ValidationReport,
}

/// What a commit wrote.
#[derive(Debug, Serialize)]
pub(crate) struct Committed {
    /// The format version this surface publishes.
    format_version: u32,
    /// The version uid the CDR assigned, which the next update sends in
    /// `If-Match`.
    version_uid: String,
    /// The versioned object the version belongs to, which addresses the
    /// latest version of the document.
    versioned_object_uid: String,
}

/// Which form a stored composition is read back into.
#[derive(Debug, Deserialize)]
pub(crate) struct ReadQuery {
    /// The template identifier, absent when the request omitted it.
    template: Option<String>,
}

/// What a read-back found.
#[derive(Debug, Serialize)]
pub(crate) struct ReadBackAnswer {
    /// The format version the values are written in.
    format_version: u32,
    /// The values, keyed the way the form definition is keyed.
    values: FormValues,
    /// The Reference Model path of every node this form does not cover.
    uncovered: Vec<String>,
    /// Every node the reader had to place by position.
    ambiguous: Vec<String>,
}

/// `POST /api/templates/{template_id}/validation`.
pub(crate) async fn validation(
    State(state): State<ServerState>,
    Path(template_id): Path<String>,
    body: Bytes,
) -> Result<Json<Judgement>, ApiError> {
    let held = held(&state, template_id)?;
    let submission = Submission::read(&body)?;
    let gate = Commit {
        definition: &held.definition,
        validator: &held.validator,
        envelope: &submission.envelope,
    };
    let report = match gate.validated(&submission.values) {
        Ok(_) => ValidationReport::default(),
        Err(CommitError::Refused { report }) => *report,
        Err(CommitError::Build { source }) => error::build_report(&source),
        Err(other) => return Err(error::from_commit(other)),
    };
    Ok(Json(Judgement {
        format_version: FORMAT_VERSION,
        valid: report.is_valid(),
        report,
    }))
}

/// `POST /api/ehrs/{ehr_id}/templates/{template_id}/compositions`.
pub(crate) async fn create(
    State(state): State<ServerState>,
    Path((ehr_id, template_id)): Path<(String, String)>,
    body: Bytes,
) -> Result<(StatusCode, Json<Committed>), ApiError> {
    let held = held(&state, template_id)?;
    let submission = Submission::read(&body)?;
    let gate = Commit {
        definition: &held.definition,
        validator: &held.validator,
        envelope: &submission.envelope,
    };
    let written = gate
        .create(&state.cdr, &EhrId::new(ehr_id), &submission.values)
        .await
        .map_err(error::from_commit)?;
    Ok((
        StatusCode::CREATED,
        Json(Committed {
            format_version: FORMAT_VERSION,
            version_uid: written.version.as_str().to_owned(),
            versioned_object_uid: written.version.object().as_str().to_owned(),
        }),
    ))
}

/// `GET /api/ehrs/{ehr_id}/compositions/{uid}/values`.
pub(crate) async fn values(
    State(state): State<ServerState>,
    Path((ehr_id, uid)): Path<(String, String)>,
    Query(query): Query<ReadQuery>,
) -> Result<Json<ReadBackAnswer>, ApiError> {
    let template_id = query
        .template
        .ok_or(ApiError::MissingQuery { name: "template" })?;
    let held = held(&state, template_id)?;
    let stored = state
        .cdr
        .read_composition(&EhrId::new(ehr_id), &composition_id(&uid))
        .await
        .map_err(|source| ApiError::CdrUnreachable {
            source: Box::new(source),
        })?
        .ok_or(ApiError::CompositionDeleted { uid })?;
    let found = read::values(&held.definition, &stored).map_err(|source| ApiError::Unreadable {
        source: Box::new(source),
    })?;
    Ok(Json(ReadBackAnswer {
        format_version: FORMAT_VERSION,
        values: found.values,
        uncovered: found.uncovered,
        ambiguous: found.ambiguous,
    }))
}

/// The template `template_id` names, or the 404 that says the server has no
/// such form.
fn held(state: &ServerState, template_id: String) -> Result<&HeldTemplate, ApiError> {
    state
        .templates
        .get(&template_id)
        .ok_or(ApiError::UnknownTemplate { template_id })
}

/// The identifier a path segment names.
fn composition_id(uid: &str) -> CompositionId {
    if uid.contains(VERSION_SEPARATOR) {
        CompositionId::Version(VersionUid::new(uid))
    } else {
        CompositionId::LatestVersionOf(VersionedObjectUid::new(uid))
    }
}

#[cfg(test)]
mod tests {
    use super::{VERSION_SEPARATOR, composition_id};
    use ferrochart_cdr::ids::CompositionId;

    #[test]
    fn an_identifier_carrying_the_separator_names_one_version() {
        let uid = format!("f0a1{VERSION_SEPARATOR}ferro.example{VERSION_SEPARATOR}1");
        assert!(matches!(composition_id(&uid), CompositionId::Version(_)));
    }

    #[test]
    fn an_identifier_without_it_names_the_versioned_object() {
        assert!(matches!(
            composition_id("f0a1"),
            CompositionId::LatestVersionOf(_)
        ));
    }
}
