// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Creating the EHR a COMPOSITION is committed into.
//!
//! All citations are openEHR ITS-REST Release-1.1.0 `ehr.html`.

use http::{Method, StatusCode};
use serde::Deserialize;

use crate::client::{self, CdrClient};
use crate::error::CdrError;
use crate::header;
use crate::ids::EhrId;

/// The canonical JSON media type.
const EHR_JSON: &str = "application/json";

/// The `return=identifier` response body of an EHR create.
#[derive(Debug, Deserialize)]
struct Identifier {
    uid: String,
}

impl CdrClient {
    /// Creates an EHR and returns the identifier the CDR assigned it.
    ///
    /// `POST /v1/ehr`. FerroCHART does not manage records, so this exists for
    /// the one case that needs it: a COMPOSITION has to be committed into an
    /// EHR, and a first run or a test has none.
    ///
    /// The identifier comes from the `ETag` where there is one and from the
    /// `return=identifier` body otherwise, because the header is only a
    /// `SHOULD`.
    ///
    /// # Errors
    /// [`CdrError::UnnamedVersion`] when the response names no identifier, and
    /// one variant per status the specification publishes.
    pub async fn create_ehr(&self) -> Result<EhrId, CdrError> {
        let url = self.endpoint(&["ehr"])?;
        let response = self
            .request(Method::POST, url)
            .header(http::header::CONTENT_TYPE, EHR_JSON)
            .header(http::header::ACCEPT, EHR_JSON)
            .header("Prefer", "return=identifier")
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        let status = client::status(&response);
        if status != StatusCode::CREATED {
            return Err(client::failure(response, "ehr endpoint", "ehr", EHR_JSON).await);
        }
        // The EHR's ETag holds `EHR.ehr_id.value`, which is a plain
        // identifier rather than a version uid, so it is unquoted here rather
        // than parsed as one.
        if let Some(tagged) = client::header_of(&response, "etag")
            .as_deref()
            .and_then(header::unquote)
            .map(EhrId::new)
        {
            return Ok(tagged);
        }
        let body = response
            .text()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        if body.trim().is_empty() {
            return Err(CdrError::UnnamedVersion { status });
        }
        let identifier: Identifier =
            serde_json::from_str(&body).map_err(|source| CdrError::Body {
                expected: "a resource identifier",
                source,
            })?;
        Ok(EhrId::new(identifier.uid))
    }

    /// Whether the CDR holds an EHR with this identifier.
    ///
    /// `GET /v1/ehr/{ehr_id}`.
    ///
    /// # Errors
    /// One variant per status the specification publishes, for anything other
    /// than a success or a 404: an EHR whose presence the CDR will not report
    /// is not the same as an EHR that is absent.
    pub async fn ehr_exists(&self, ehr: &EhrId) -> Result<bool, CdrError> {
        let url = self.endpoint(&["ehr", ehr.as_str()])?;
        let response = self
            .request(Method::GET, url)
            .header(http::header::ACCEPT, EHR_JSON)
            .send()
            .await
            .map_err(|source| CdrError::Transport { source })?;
        let status = client::status(&response);
        if status.is_success() {
            return Ok(true);
        }
        if status == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        Err(client::failure(response, "ehr", ehr.as_str(), EHR_JSON).await)
    }
}
