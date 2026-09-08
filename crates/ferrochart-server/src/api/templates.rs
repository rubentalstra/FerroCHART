// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The templates a server holds, and the form each one compiles to.

use axum::Json;
use axum::extract::{Path, State};
use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::ids::TemplateId;
use serde::Serialize;

use crate::api::ServerState;
use crate::api::error::ApiError;

/// The identifiers one server holds.
#[derive(Debug, Serialize)]
pub(crate) struct TemplateList {
    /// The format version the definitions behind these identifiers are
    /// written in.
    format_version: u32,
    /// Every identifier, in identifier order.
    templates: Vec<TemplateId>,
}

/// `GET /api/templates`.
pub(crate) async fn list(State(state): State<ServerState>) -> Json<TemplateList> {
    Json(TemplateList {
        format_version: FORMAT_VERSION,
        templates: state.templates.ids().cloned().collect(),
    })
}

/// `GET /api/templates/{template_id}/definition`.
pub(crate) async fn definition(
    State(state): State<ServerState>,
    Path(template_id): Path<String>,
) -> Result<Json<FormDefinition>, ApiError> {
    let held = state
        .templates
        .get(&template_id)
        .ok_or(ApiError::UnknownTemplate { template_id })?;
    // The definition carries its own format version, so the response needs no
    // wrapper to state one.
    Ok(Json(held.definition.clone()))
}
