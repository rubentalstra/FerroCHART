// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Uploading a template to a CDR, and taking one back out.
//!
//! All citations are openEHR ITS-REST Release-1.1.0 `definition.html` unless
//! another page is named.
//!
//! A CDR may implement one ADL generation, the other, or both, so the two
//! upload paths are separate calls rather than one call with a flag, and the
//! request media type differs between them. A client that guessed would report
//! the wrong thing when a server answered 404.

use ferrochart_form::ids::TemplateId;
use http::{Method, StatusCode};
use serde::Deserialize;

use crate::client::{self, CdrClient};
use crate::error::CdrError;

/// The media type of an ADL 1.4 operational template on the wire.
///
/// There is no OPT-specific XML media type: `overview.html` says only that a
/// canonical XML payload "MUST conform to the published XSDs".
const OPT_XML: &str = "application/xml";

/// The media type of ADL 2 source.
const ADL2_TEXT: &str = "text/plain";

/// The media type an upload response uses to name what it stored.
const TEMPLATE_JSON: &str = "application/json";

/// The media type of the web template.
///
/// `overview.html` registers it as "the Operational Template definition as Web
/// Template JSON format". The format itself is a compatibility target rather
/// than a specification: ITS-REST Release-1.1.0 `simplified_formats.html`
/// section 2.2 puts "Web Template itself as a resource" under what the
/// specification does not cover.
const WEB_TEMPLATE_JSON: &str = "application/openehr.wt+json";

/// The `template_id` an upload reports.
#[derive(Debug, Deserialize)]
struct TemplateIdentifier {
    template_id: String,
}

/// Which ADL generation a template is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// ADL 1.4, uploaded as operational template XML.
    Adl14,
    /// ADL 2, uploaded as source text.
    Adl2,
}

impl Generation {
    /// The path segment under `definition/template` for this generation.
    #[must_use]
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::Adl14 => "adl1.4",
            Self::Adl2 => "adl2",
        }
    }

    /// The request media type an upload of this generation carries.
    #[must_use]
    pub fn upload_media_type(self) -> &'static str {
        match self {
            Self::Adl14 => OPT_XML,
            Self::Adl2 => ADL2_TEXT,
        }
    }

    /// The media type this generation's source reads back as.
    #[must_use]
    pub fn source_media_type(self) -> &'static str {
        self.upload_media_type()
    }
}

/// What an upload stored.
#[derive(Debug, Clone)]
pub struct Uploaded {
    /// The identifier the CDR gave the template, "fully qualified".
    pub template_id: TemplateId,
}

impl CdrClient {
    /// Uploads `source` as a template of `generation`.
    ///
    /// `POST /v1/definition/template/adl1.4` takes the operational template
    /// XML as `application/xml`, and `POST /v1/definition/template/adl2` takes
    /// ADL 2 source as `text/plain`. The only success status is 201.
    ///
    /// The response is read under `Accept: application/json`, which is the one
    /// shape that hands back the CDR's canonical identifier without
    /// re-transferring the template.
    ///
    /// # Errors
    /// [`CdrError::Conflict`] when the CDR already holds a template with this
    /// identifier, which is an ordinary outcome rather than a client defect,
    /// [`CdrError::Unimplemented`] when the CDR does not offer this
    /// generation, and one variant per status the specification publishes.
    pub async fn upload_template(
        &self,
        generation: Generation,
        source: &str,
    ) -> Result<Uploaded, CdrError> {
        let url = self.endpoint(&["definition", "template", generation.path_segment()])?;
        let response = self
            .request(Method::POST, url)
            .header(http::header::CONTENT_TYPE, generation.upload_media_type())
            .header(http::header::ACCEPT, TEMPLATE_JSON)
            .header("Prefer", "return=identifier")
            .body(source.to_owned())
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        if client::status(&response) != StatusCode::CREATED {
            return Err(client::failure(
                response,
                "template endpoint",
                generation.path_segment(),
                TEMPLATE_JSON,
            )
            .await);
        }
        let body = response
            .text()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        let identifier: TemplateIdentifier =
            serde_json::from_str(&body).map_err(|source| CdrError::Body {
                expected: "a template identifier",
                source,
            })?;
        Ok(Uploaded {
            template_id: TemplateId::new(identifier.template_id),
        })
    }

    /// Reads back the canonical operational template XML for `id`.
    ///
    /// `GET /v1/definition/template/adl1.4/{template_id}` under
    /// `Accept: application/xml`. This is how FerroCHART takes a template from
    /// a CDR rather than from a file on disk.
    ///
    /// # Errors
    /// [`CdrError::NotFound`] when the CDR holds no such template, and one
    /// variant per status the specification publishes.
    pub async fn operational_template(&self, id: &TemplateId) -> Result<String, CdrError> {
        self.template_as(id, Generation::Adl14, OPT_XML).await
    }

    /// Reads back the web template for `id`.
    ///
    /// The same path under `Accept: application/openehr.wt+json`. The web
    /// template is a compatibility target rather than a specification, so a
    /// CDR may answer 406, which is a capability answer and not a defect.
    ///
    /// # Errors
    /// [`CdrError::UnsupportedMediaType`] when the CDR offers no web template,
    /// [`CdrError::NotFound`] when it holds no such template, and one variant
    /// per status the specification publishes.
    pub async fn web_template(&self, id: &TemplateId) -> Result<String, CdrError> {
        self.template_as(id, Generation::Adl14, WEB_TEMPLATE_JSON)
            .await
    }

    /// Reads back the ADL 2 source of `id`.
    ///
    /// `GET /v1/definition/template/adl2/{template_id}` under
    /// `Accept: text/plain`.
    ///
    /// # Errors
    /// [`CdrError::NotFound`] when the CDR holds no such template, and one
    /// variant per status the specification publishes.
    pub async fn adl2_template(&self, id: &TemplateId) -> Result<String, CdrError> {
        self.template_as(id, Generation::Adl2, ADL2_TEXT).await
    }

    /// One template retrieval, under one `Accept`.
    async fn template_as(
        &self,
        id: &TemplateId,
        generation: Generation,
        accept: &str,
    ) -> Result<String, CdrError> {
        let url = self.endpoint(&[
            "definition",
            "template",
            generation.path_segment(),
            id.as_str(),
        ])?;
        let response = self
            .request(Method::GET, url)
            .header(http::header::ACCEPT, accept)
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        if client::status(&response) != StatusCode::OK {
            return Err(client::failure(response, "template", id.as_str(), accept).await);
        }
        response
            .text()
            .await
            .map_err(|source| CdrError::Transport { source })
    }
}
