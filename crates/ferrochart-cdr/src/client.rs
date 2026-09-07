// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The connection to one CDR, and the one place a response becomes an error.

use std::time::Duration;

use http::StatusCode;
use url::Url;

use crate::error::CdrError;
use crate::header;
use crate::ids::VersionUid;

/// The `/v1` prefix every ITS-REST path sits under.
///
/// openEHR ITS-REST Release-1.1.0 declares the server as
/// `https://{baseUrl}/v1`, where `baseUrl` "may contain server name, port and
/// base path prefix".
const API_ROOT: &str = "v1";

/// How long a call waits before it gives up.
const TIMEOUT: Duration = Duration::from_secs(30);

/// A client for one openEHR CDR.
///
/// Nothing here is specific to one implementation. The same client speaks to
/// `EHRbase`, Better, vitagroup or `FerroEHR`, because all of it is the
/// published REST API and none of it is a vendor extension
/// (`docs/architecture.md` section 8).
#[derive(Debug, Clone)]
pub struct CdrClient {
    base: Url,
    http: reqwest::Client,
    authorization: Option<String>,
}

impl CdrClient {
    /// Connects to the CDR whose REST API is served under `base`.
    ///
    /// `base` is the server root rather than the `/v1` path, so
    /// `https://cdr.example/openehr` and `https://cdr.example/openehr/` both
    /// work.
    ///
    /// # Errors
    /// [`CdrError::OpaqueBase`] when `base` cannot carry a path, and
    /// [`CdrError::Transport`] when the HTTP client cannot be built.
    pub fn new(base: &Url) -> Result<Self, CdrError> {
        if base.cannot_be_a_base() {
            return Err(CdrError::OpaqueBase {
                base: base.to_string(),
            });
        }
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|source| CdrError::Transport { source })?;
        Ok(Self {
            base: base.clone(),
            http,
            authorization: None,
        })
    }

    /// Sends `value` as the `Authorization` header on every request.
    ///
    /// openEHR ITS-REST Release-1.1.0 mandates no authentication scheme:
    /// "this specification does not mandate a specific authentication
    /// scheme", while requiring that "Clients MUST send valid `Authorization`
    /// and `Proxy-Authorization` headers in their requests when required". The
    /// whole header value is therefore configured rather than a scheme being
    /// chosen here.
    #[must_use]
    pub fn with_authorization(mut self, value: impl Into<String>) -> Self {
        self.authorization = Some(value.into());
        self
    }

    /// The URL of `segments` under this CDR's `/v1` root.
    ///
    /// Each segment is percent-encoded, because a `template_id` is not
    /// URL-safe by construction: openEHR ITS-REST Release-1.1.0
    /// `definition.html` gives `Vital Signs` as a legacy template identifier
    /// and shows it in a `Location` as `Vital%20Signs`.
    ///
    /// # Errors
    /// [`CdrError::OpaqueBase`] when the base cannot carry a path.
    pub(crate) fn endpoint(&self, segments: &[&str]) -> Result<Url, CdrError> {
        let mut url = self.base.clone();
        {
            let mut path = url.path_segments_mut().map_err(|()| CdrError::OpaqueBase {
                base: self.base.to_string(),
            })?;
            // A base whose path ends in a slash yields a trailing empty
            // segment, which would otherwise become a `//` in the result.
            path.pop_if_empty().push(API_ROOT).extend(segments);
        }
        Ok(url)
    }

    /// A request builder carrying whatever authorization is configured.
    pub(crate) fn request(&self, method: http::Method, url: Url) -> reqwest::RequestBuilder {
        let request = self.http.request(method, url);
        match self.authorization {
            None => request,
            Some(ref value) => request.header(http::header::AUTHORIZATION, value),
        }
    }
}

/// One response header as text, where it is present and is text.
pub(crate) fn header_of(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The version uid a response names in its `ETag`.
pub(crate) fn version_header(response: &reqwest::Response) -> Option<VersionUid> {
    header_of(response, "etag").and_then(|etag| header::version_of_etag(&etag))
}

/// The status a response carries.
///
/// reqwest re-exports `http`'s own type, so this is the same `StatusCode` the
/// error variants hold and a status is compared as a type rather than as a
/// number (`.claude/rules/rust-style.md`).
pub(crate) fn status(response: &reqwest::Response) -> StatusCode {
    response.status()
}

/// The response body, or a note that it could not be read.
///
/// A body that will not read is not a reason to lose the status, which is the
/// part a caller branches on, so the read failure is reported in place of the
/// text rather than replacing the whole error.
pub(crate) async fn body_text(response: reqwest::Response) -> String {
    match response.text().await {
        Ok(text) => text,
        Err(error) => format!("<the response body could not be read: {error}>"),
    }
}

/// The typed error an unsuccessful response becomes.
///
/// `what` names the kind of thing the call addressed, and `id` what it asked
/// for, so a 404 says which. `media_type` is what the request asked the CDR
/// to produce or accept, for the two capability statuses.
pub(crate) async fn failure(
    response: reqwest::Response,
    what: &'static str,
    id: &str,
    media_type: &str,
) -> CdrError {
    let status = status(&response);
    match status {
        StatusCode::NOT_FOUND => CdrError::NotFound {
            what,
            id: id.to_owned(),
        },
        StatusCode::UNAUTHORIZED
        | StatusCode::FORBIDDEN
        | StatusCode::PROXY_AUTHENTICATION_REQUIRED => {
            let challenge = header_of(&response, "www-authenticate")
                .or_else(|| header_of(&response, "proxy-authenticate"));
            CdrError::Unauthorized { status, challenge }
        }
        StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED => CdrError::Unimplemented {
            status,
            body: body_text(response).await,
        },
        StatusCode::NOT_ACCEPTABLE => CdrError::UnsupportedMediaType {
            status,
            what: "Accept",
            media_type: media_type.to_owned(),
        },
        StatusCode::UNSUPPORTED_MEDIA_TYPE => CdrError::UnsupportedMediaType {
            status,
            what: "Content-Type",
            media_type: media_type.to_owned(),
        },
        StatusCode::CONFLICT => {
            let latest = version_header(&response);
            CdrError::Conflict {
                body: body_text(response).await,
                latest,
            }
        }
        StatusCode::BAD_REQUEST => CdrError::BadRequest {
            body: body_text(response).await,
        },
        StatusCode::UNPROCESSABLE_ENTITY => CdrError::Unprocessable {
            body: body_text(response).await,
        },
        StatusCode::REQUEST_TIMEOUT | StatusCode::INTERNAL_SERVER_ERROR => CdrError::Unavailable {
            status,
            body: body_text(response).await,
        },
        other => CdrError::Upstream {
            status: other,
            body: body_text(response).await,
        },
    }
}
