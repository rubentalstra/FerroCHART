// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Creating, reading and updating a COMPOSITION.
//!
//! All citations are openEHR ITS-REST Release-1.1.0 `ehr.html` unless another
//! page is named.

use http::{Method, StatusCode};
use openehr_rm::v1_2::composition::composition::Composition;
use serde::Deserialize;

use crate::client::{self, CdrClient};
use crate::error::CdrError;
use crate::header::{self, Prefer};
use crate::ids::{CompositionId, EhrId, VersionUid, VersionedObjectUid};

/// The canonical JSON media type for a COMPOSITION.
///
/// A service "MUST support at least one of the openEHR XML or JSON canonical
/// formats" (`overview.html`), and canonical JSON is what
/// `docs/architecture.md` section 8 chose: its fidelity is guaranteed by the
/// Reference Model rather than by a mapping document.
const COMPOSITION_JSON: &str = "application/json";

/// The `return=identifier` response body.
///
/// `overview.html`: "the response body will be a single JSON object with a
/// single `uid` attribute". This is the fallback when a conformant response
/// carries no `ETag`, which is only a `SHOULD`.
#[derive(Debug, Deserialize)]
struct Identifier {
    uid: String,
}

/// What a write gives back.
#[derive(Debug, Clone)]
pub struct Written {
    /// The version uid the CDR assigned. The next update sends it in
    /// `If-Match`.
    pub version: VersionUid,
    /// The stored document, present only where the write asked for a
    /// representation.
    pub composition: Option<Composition>,
}

impl CdrClient {
    /// Creates a COMPOSITION in `ehr`.
    ///
    /// `POST /v1/ehr/{ehr_id}/composition`, whose only success status is 201.
    ///
    /// # Errors
    /// [`CdrError::NotFound`] when the CDR has no such EHR,
    /// [`CdrError::Unprocessable`] when it refuses the document, and one
    /// variant per status the specification publishes. A refusal is always
    /// FerroCHART's bug: the form validated the values before they got here.
    pub async fn create_composition(
        &self,
        ehr: &EhrId,
        composition: &Composition,
        prefer: Prefer,
    ) -> Result<Written, CdrError> {
        let url = self.endpoint(&["ehr", ehr.as_str(), "composition"])?;
        let response = self
            .request(Method::POST, url)
            .header(http::header::CONTENT_TYPE, COMPOSITION_JSON)
            .header(http::header::ACCEPT, COMPOSITION_JSON)
            .header("Prefer", prefer.as_str())
            .json(composition)
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        if client::status(&response) != StatusCode::CREATED {
            return Err(client::failure(response, "ehr", ehr.as_str(), COMPOSITION_JSON).await);
        }
        written(response, prefer).await
    }

    /// Reads a COMPOSITION out of `ehr`.
    ///
    /// `GET /v1/ehr/{ehr_id}/composition/{uid_based_id}`, where the identifier
    /// takes either form: a versioned object uid reads the latest version, and
    /// a full version uid reads that one version.
    ///
    /// Returns `None` where the CDR answers 204, which it does when the
    /// resource "has been deleted". That is an answer rather than a failure,
    /// and it is distinct from the 404 an absent composition earns.
    ///
    /// # Errors
    /// [`CdrError::NotFound`] when the EHR or the composition is absent,
    /// [`CdrError::Body`] when the response is not a COMPOSITION, and one
    /// variant per status the specification publishes.
    pub async fn read_composition(
        &self,
        ehr: &EhrId,
        id: &CompositionId,
    ) -> Result<Option<Composition>, CdrError> {
        let url = self.endpoint(&["ehr", ehr.as_str(), "composition", id.as_str()])?;
        let response = self
            .request(Method::GET, url)
            .header(http::header::ACCEPT, COMPOSITION_JSON)
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        let status = client::status(&response);
        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        if status != StatusCode::OK {
            return Err(
                client::failure(response, "composition", id.as_str(), COMPOSITION_JSON).await,
            );
        }
        let body = response
            .text()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        serde_json::from_str(&body)
            .map(Some)
            .map_err(|source| CdrError::Body {
                expected: "a COMPOSITION",
                source,
            })
    }

    /// Updates the COMPOSITION `id` in `ehr`, replacing the version
    /// `preceding`.
    ///
    /// `PUT /v1/ehr/{ehr_id}/composition/{uid_based_id}`. The path identifier
    /// here "can take only a form of an `HIER_OBJECT_ID` identifier taken from
    /// `VERSIONED_OBJECT.uid.value`", which is why this takes a
    /// [`VersionedObjectUid`] where the read takes either form. `If-Match`
    /// carries `preceding`, quoted and unprefixed, and is required.
    ///
    /// Both 200 and 204 are success, chosen by `prefer`.
    ///
    /// # Errors
    /// [`CdrError::ConcurrentEdit`] when the composition has moved on since
    /// `preceding` was read, which the form shows to the clinician rather than
    /// retrying, and one variant per status the specification publishes.
    pub async fn update_composition(
        &self,
        ehr: &EhrId,
        id: &VersionedObjectUid,
        preceding: &VersionUid,
        composition: &Composition,
        prefer: Prefer,
    ) -> Result<Written, CdrError> {
        let url = self.endpoint(&["ehr", ehr.as_str(), "composition", id.as_str()])?;
        let response = self
            .request(Method::PUT, url)
            .header(http::header::CONTENT_TYPE, COMPOSITION_JSON)
            .header(http::header::ACCEPT, COMPOSITION_JSON)
            .header(http::header::IF_MATCH, header::if_match(preceding))
            .header("Prefer", prefer.as_str())
            .json(composition)
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        let status = client::status(&response);
        if status == StatusCode::PRECONDITION_FAILED {
            return Err(CdrError::ConcurrentEdit {
                expected: preceding.clone(),
                latest: client::version_header(&response),
            });
        }
        if status != StatusCode::OK && status != StatusCode::NO_CONTENT {
            return Err(
                client::failure(response, "composition", id.as_str(), COMPOSITION_JSON).await,
            );
        }
        written(response, prefer).await
    }
}

/// The outcome of a successful write.
///
/// The version comes from the `ETag` where there is one, and from the
/// `return=identifier` body otherwise. Both are `SHOULD` rather than `MUST`,
/// and an `ETag` may be "an opaque quoted string" that is not an identifier at
/// all, so neither is assumed.
async fn written(response: reqwest::Response, prefer: Prefer) -> Result<Written, CdrError> {
    let status = client::status(&response);
    let tagged = client::version_header(&response);
    let composition = match prefer {
        Prefer::Minimal | Prefer::Identifier => None,
        Prefer::Representation => {
            let body = response
                .text()
                .await
                .map_err(|source| CdrError::Transport { source })?;
            let composition = serde_json::from_str(&body).map_err(|source| CdrError::Body {
                expected: "a COMPOSITION",
                source,
            })?;
            return match tagged {
                Some(version) => Ok(Written {
                    version,
                    composition: Some(composition),
                }),
                None => Err(CdrError::UnnamedVersion { status }),
            };
        }
    };
    let version = match tagged {
        Some(version) => version,
        None if prefer == Prefer::Identifier => identifier(response).await?,
        None => return Err(CdrError::UnnamedVersion { status }),
    };
    Ok(Written {
        version,
        composition,
    })
}

/// The version uid out of a `return=identifier` body.
async fn identifier(response: reqwest::Response) -> Result<VersionUid, CdrError> {
    let status = client::status(&response);
    let body = response
        .text()
        .await
        .map_err(|source| CdrError::Transport { source })?;
    if body.trim().is_empty() {
        return Err(CdrError::UnnamedVersion { status });
    }
    let identifier: Identifier = serde_json::from_str(&body).map_err(|source| CdrError::Body {
        expected: "a resource identifier",
        source,
    })?;
    Ok(VersionUid::new(identifier.uid))
}
