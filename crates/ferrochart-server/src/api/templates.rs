// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The templates a server holds, and the form each one compiles to.

use axum::Json;
use axum::extract::{Path, State};
use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::ids::TemplateId;
use ferrochart_form::layout::FormLayout;
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

/// `GET /api/templates/{template_id}/layout`.
///
/// A template a person laid out answers with what they authored, and one they
/// did not answers with a layout that decides nothing. The two are the same
/// thing to a renderer, so this route has no "no layout" status: a client that
/// got a `404` here would have to tell it apart from a template this server
/// does not hold, which is the one thing the status does mean.
pub(crate) async fn layout(
    State(state): State<ServerState>,
    Path(template_id): Path<String>,
) -> Result<Json<FormLayout>, ApiError> {
    if state.templates().get(&template_id).is_none() {
        return Err(ApiError::UnknownTemplate { template_id });
    }
    let authored = state.overlays().get(&template_id).cloned();
    // The layout carries its own format version, so the response needs no
    // wrapper to state one.
    Ok(Json(authored.unwrap_or_else(|| {
        FormLayout::new(TemplateId::new(template_id))
    })))
}
