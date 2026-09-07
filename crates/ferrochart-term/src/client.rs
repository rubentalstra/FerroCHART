// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The connection to one FHIR terminology server, and the one place a response
//! becomes an error.

use std::time::Duration;

use fhir_types::codec::{Json, Object, Path};
use fhir_types::r4::operation_outcome::OperationOutcome;
use fhir_types::r4::parameters::Parameters;
use http::{Method, StatusCode};
use url::Url;

use crate::error::TermError;
use crate::system::SystemMap;

/// The media type a FHIR JSON request and response carries.
///
/// HL7 FHIR R4 4.0.1 `http.html` section 3.1.0.1.9: "The formal MIME-type for
/// FHIR resources is `application/fhir+xml` or `application/fhir+json`. The
/// correct mime type SHALL be used by clients and servers".
pub(crate) const FHIR_JSON: &str = "application/fhir+json";

/// How long a call waits before it gives up.
const TIMEOUT: Duration = Duration::from_secs(30);

/// A client for one FHIR R4 terminology server.
///
/// Nothing here is specific to one implementation. The same client speaks to
/// any conformant server, because all of it is the published terminology
/// service API and none of it is a vendor extension
/// (`docs/architecture.md` section 7.1).
#[derive(Debug, Clone)]
pub struct TermClient {
    base: Url,
    http: reqwest::Client,
    authorization: Option<String>,
    systems: SystemMap,
}

impl TermClient {
    /// Connects to the terminology server whose FHIR API is served under
    /// `base`.
    ///
    /// `base` is the FHIR service root, so `https://tx.example/r4` and
    /// `https://tx.example/r4/` both work.
    ///
    /// # Errors
    /// [`TermError::OpaqueBase`] when `base` cannot carry a path, and
    /// [`TermError::Transport`] when the HTTP client cannot be built.
    pub fn new(base: &Url) -> Result<Self, TermError> {
        if base.cannot_be_a_base() {
            return Err(TermError::OpaqueBase {
                base: base.to_string(),
            });
        }
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|source| TermError::Transport { source })?;
        Ok(Self {
            base: base.clone(),
            http,
            authorization: None,
            systems: SystemMap::new(),
        })
    }

    /// Sends `value` as the `Authorization` header on every request.
    ///
    /// No specification governs which scheme a terminology server wants: our
    /// own design is to configure the whole header value, so a deployment
    /// behind bearer tokens, basic auth or an API gateway all work without a
    /// change here.
    #[must_use]
    pub fn with_authorization(mut self, value: impl Into<String>) -> Self {
        self.authorization = Some(value.into());
        self
    }

    /// Uses `systems` to turn an openEHR `terminology_id` into a FHIR system
    /// URI.
    #[must_use]
    pub fn with_systems(mut self, systems: SystemMap) -> Self {
        self.systems = systems;
        self
    }

    /// Records that the openEHR terminology `terminology` is the FHIR code
    /// system at `uri`.
    #[must_use]
    pub fn with_system(
        mut self,
        terminology: &ferrochart_form::ids::TerminologyName,
        uri: impl Into<String>,
    ) -> Self {
        self.systems.insert(terminology, uri);
        self
    }

    /// The table this client turns an openEHR `terminology_id` into a FHIR
    /// system URI with.
    #[must_use]
    pub fn systems(&self) -> &SystemMap {
        &self.systems
    }

    /// The URL of `segments` under this server's FHIR root.
    ///
    /// Each segment is percent-encoded, because an operation name is not
    /// URL-safe by construction and a value set canonical URL never appears in
    /// a path here at all.
    ///
    /// # Errors
    /// [`TermError::OpaqueBase`] when the base cannot carry a path.
    pub(crate) fn endpoint(&self, segments: &[&str]) -> Result<Url, TermError> {
        let mut url = self.base.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|()| TermError::OpaqueBase {
                    base: self.base.to_string(),
                })?;
            // A base whose path ends in a slash yields a trailing empty
            // segment, which would otherwise become a `//` in the result.
            path.pop_if_empty().extend(segments);
        }
        Ok(url)
    }

    /// A request builder carrying whatever authorization is configured.
    pub(crate) fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let request = self.http.request(method, url);
        match self.authorization {
            None => request,
            Some(ref value) => request.header(http::header::AUTHORIZATION, value),
        }
    }

    /// Invokes the operation at `segments` with `parameters`, and reads the
    /// JSON object it answers with.
    ///
    /// HL7 FHIR R4 4.0.1 `operations.html` section 3.2.0.6.1 gives POST with a
    /// `Parameters` resource as the general invocation form, and permits GET
    /// only where every parameter is primitive. A value set supplied with the
    /// request is not, so every call here is a POST and the wire form does not
    /// change with the arguments.
    ///
    /// # Errors
    /// [`TermError::Transport`] when the request gets no answer,
    /// [`TermError::NotJson`] when the body is not JSON, and one variant per
    /// status family the specification publishes.
    pub(crate) async fn invoke(
        &self,
        segments: &[&str],
        parameters: &Parameters,
        target: &str,
    ) -> Result<Object, TermError> {
        let url = self.endpoint(segments)?;
        let request = parameters
            .to_json()
            .map_err(|source| TermError::Request { source })?;
        let body =
            serde_json::to_string(&request).map_err(|source| TermError::NotJson { source })?;
        tracing::debug!(
            operation = segments.join("/"),
            target,
            "terminology request"
        );
        let response = self
            .request(Method::POST, url)
            .header(http::header::CONTENT_TYPE, FHIR_JSON)
            .header(http::header::ACCEPT, FHIR_JSON)
            .body(body)
            .send()
            .await
            .map_err(|source| TermError::Transport { source })?;
        if response.status() != StatusCode::OK {
            return Err(failure(response, target).await);
        }
        let text = response
            .text()
            .await
            .map_err(|source| TermError::Transport { source })?;
        object_of(&text)
    }
}

/// The JSON object `text` holds.
///
/// # Errors
/// [`TermError::NotJson`] when the text is not a JSON object.
pub(crate) fn object_of(text: &str) -> Result<Object, TermError> {
    serde_json::from_str(text).map_err(|source| TermError::NotJson { source })
}

/// One response header as text, where it is present and is text.
fn header_of(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The response body, or a note that it could not be read.
///
/// A body that will not read is not a reason to lose the status, which is the
/// part a caller branches on, so the read failure is reported in place of the
/// text rather than replacing the whole error.
async fn body_text(response: reqwest::Response) -> String {
    match response.text().await {
        Ok(text) => text,
        Err(error) => format!("<the response body could not be read: {error}>"),
    }
}

/// What the `OperationOutcome` in `body` says, where the body is one.
///
/// HL7 FHIR R4 4.0.1 `operations.html` section 3.2.0.6.2: "An HTTP status code
/// of 4xx or 5xx indicates an error, and an `OperationOutcome` SHOULD be
/// returned with details." It is a `SHOULD`, and `http.html` section 3.1.0.1.5
/// says the outcome "may be returned with any HTTP 4xx or 5xx response, but
/// this is not required", so a body that is not one is not a defect and the
/// raw text is kept either way.
#[must_use]
pub(crate) fn diagnostics_of(body: &str) -> Option<String> {
    let object: Object = serde_json::from_str(body).ok()?;
    let mut path = Path::lenient("OperationOutcome");
    let outcome = OperationOutcome::from_json(&object, &mut path).ok()?;
    let said: Vec<String> = outcome
        .issue
        .iter()
        .filter_map(|issue| {
            issue
                .diagnostics
                .as_ref()
                .and_then(|text| text.value.clone())
                .or_else(|| {
                    issue
                        .details
                        .as_ref()
                        .and_then(|details| details.text.as_ref())
                        .and_then(|text| text.value.clone())
                })
        })
        .collect();
    if said.is_empty() {
        return None;
    }
    Some(said.join("; "))
}

/// The typed error an unsuccessful response becomes.
///
/// `target` names what the request asked about, so a refusal says which value
/// set it was about.
pub(crate) async fn failure(response: reqwest::Response, target: &str) -> TermError {
    let status = response.status();
    match status {
        StatusCode::UNAUTHORIZED
        | StatusCode::FORBIDDEN
        | StatusCode::PROXY_AUTHENTICATION_REQUIRED => {
            let challenge = header_of(&response, "www-authenticate")
                .or_else(|| header_of(&response, "proxy-authenticate"));
            TermError::Unauthorized { status, challenge }
        }
        StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED => TermError::Unimplemented {
            status,
            body: body_text(response).await,
        },
        // A 404 from an operation endpoint says the server does not offer the
        // operation, and a 404 from an operation that ran says it does not
        // hold the value set. The two are told apart by whether the body is an
        // `OperationOutcome`, which section 3.2.0.6.2 asks an operation to
        // answer with.
        StatusCode::NOT_FOUND => {
            let body = body_text(response).await;
            match diagnostics_of(&body) {
                Some(diagnostics) => TermError::UnknownValueSet {
                    status,
                    target: target.to_owned(),
                    diagnostics: Some(diagnostics),
                    body,
                },
                None => TermError::Unimplemented { status, body },
            }
        }
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            let body = body_text(response).await;
            let diagnostics = diagnostics_of(&body);
            if is_not_found(diagnostics.as_deref()) {
                return TermError::UnknownValueSet {
                    status,
                    target: target.to_owned(),
                    diagnostics,
                    body,
                };
            }
            TermError::Refused {
                status,
                target: target.to_owned(),
                diagnostics,
                body,
            }
        }
        StatusCode::REQUEST_TIMEOUT
        | StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => TermError::Unavailable {
            status,
            body: body_text(response).await,
        },
        other => {
            let body = body_text(response).await;
            let diagnostics = diagnostics_of(&body);
            TermError::Upstream {
                status: other,
                diagnostics,
                body,
            }
        }
    }
}

/// Whether an `OperationOutcome`'s text says the value set is the thing that
/// is missing.
///
/// HL7 FHIR R4 4.0.1 `valueset-operation-expand.html` section 4.9.15.1 makes
/// a server that cannot expand answer with an error and leaves the status to
/// the server, so a value set the server does not hold arrives as a 400, a 422
/// or a 404 depending on the implementation. The outcome text is what
/// separates that from a request the server understood and refused.
fn is_not_found(diagnostics: Option<&str>) -> bool {
    diagnostics.is_some_and(|text| text.to_ascii_lowercase().contains("not found"))
}

#[cfg(test)]
mod tests {
    use super::{TermClient, diagnostics_of, is_not_found};
    use url::Url;

    fn client(base: &str) -> TermClient {
        let base = Url::parse(base).expect("the base is a URL");
        TermClient::new(&base).expect("the client builds")
    }

    #[test]
    fn an_operation_name_keeps_its_dollar_in_the_path() {
        // `$expand` is a path segment, and percent-encoding the `$` would
        // address an endpoint no server serves.
        let url = client("https://tx.example/r4")
            .endpoint(&["ValueSet", "$expand"])
            .expect("the endpoint builds");
        assert_eq!(url.as_str(), "https://tx.example/r4/ValueSet/$expand");
    }

    #[test]
    fn a_base_with_a_trailing_slash_does_not_double_it() {
        let url = client("https://tx.example/r4/")
            .endpoint(&["CodeSystem", "$lookup"])
            .expect("the endpoint builds");
        assert_eq!(url.as_str(), "https://tx.example/r4/CodeSystem/$lookup");
    }

    #[test]
    fn an_operation_outcome_gives_up_what_the_server_said() {
        let body = r#"{"resourceType":"OperationOutcome","issue":[{"severity":"error",
            "code":"not-found","diagnostics":"ValueSet not found: https://example.org/vs"}]}"#;
        assert_eq!(
            diagnostics_of(body).as_deref(),
            Some("ValueSet not found: https://example.org/vs")
        );
        assert!(is_not_found(diagnostics_of(body).as_deref()));
    }

    #[test]
    fn a_body_that_is_not_an_operation_outcome_yields_no_diagnostics() {
        assert_eq!(diagnostics_of("<html>gateway error</html>"), None);
        assert_eq!(diagnostics_of(r#"{"resourceType":"ValueSet"}"#), None);
    }
}
