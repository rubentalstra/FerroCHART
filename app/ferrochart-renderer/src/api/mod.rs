// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The one door to the network.
//!
//! `docs/architecture.md` section 10 fixes the shape: `gloo-net` rather than
//! a server-shaped HTTP client, ONE module owning every request, and typed
//! errors carrying the upstream status and body. No other module of this
//! crate opens a request, and the `no_other_module_opens_a_request` test
//! below proves it from the sources rather than from intent.
//!
//! # What is tested, and what cannot be
//!
//! `wasm-bindgen-test` needs a browser, so the transport itself is exercised
//! by neither the workspace test run nor this module. Everything that decides
//! an outcome is therefore a pure function a native test calls: the address
//! in [`route`], the error document and the status mapping in [`error`], and
//! the format check. What is left untested is the three lines that hand a
//! built request to `fetch` and read the answer's status and text back.
//!
//! # The documents
//!
//! The bodies carry `ferrochart-form`'s published types unchanged. Only the
//! envelopes around them are restated here, because they are
//! `ferrochart-server`'s and the renderer links `ferrochart-form` and nothing
//! else of this tree (`scripts/checks/crate-closure.sh`).

pub(crate) mod error;
pub(crate) mod route;

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::ids::TemplateId;
use ferrochart_form::validation::ValidationReport;
use ferrochart_form::values::FormValues;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;

/// The media type every body on this surface is written in.
const JSON: &str = "application/json";

/// A document that states which format version it is written in.
trait Versioned {
    /// The version the document states.
    fn format_version(&self) -> u32;
}

/// The identifiers one server holds.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TemplateList {
    /// The format version the definitions behind these identifiers are
    /// written in.
    pub(crate) format_version: u32,
    /// Every identifier, in identifier order.
    pub(crate) templates: Vec<TemplateId>,
}

/// What the judgement answered.
#[derive(Debug, Clone, Deserialize)]
#[expect(
    dead_code,
    reason = "the controls that submit are issue #137; the expectation clears itself the moment one reads this"
)]
pub(crate) struct Judgement {
    /// The format version the report is written in.
    pub(crate) format_version: u32,
    /// Whether the entries may be committed.
    pub(crate) valid: bool,
    /// Every failure, keyed onto the form where its path resolved.
    pub(crate) report: ValidationReport,
}

/// What a commit wrote.
#[derive(Debug, Clone, Deserialize)]
#[expect(
    dead_code,
    reason = "the controls that commit are issue #137; the expectation clears itself the moment one reads this"
)]
pub(crate) struct Committed {
    /// The format version this surface publishes.
    pub(crate) format_version: u32,
    /// The version uid the CDR assigned.
    pub(crate) version_uid: String,
    /// The versioned object the version belongs to.
    pub(crate) versioned_object_uid: String,
}

/// What a read-back found.
#[derive(Debug, Clone, Deserialize)]
#[expect(
    dead_code,
    reason = "the screen that fills a form from a stored composition is issue #137; the expectation clears itself the moment one reads this"
)]
pub(crate) struct ReadBack {
    /// The format version the values are written in.
    pub(crate) format_version: u32,
    /// The values, keyed the way the form definition is keyed.
    pub(crate) values: FormValues,
    /// The Reference Model path of every node this form does not cover.
    pub(crate) uncovered: Vec<String>,
    /// Every node the reader had to place by position.
    pub(crate) ambiguous: Vec<String>,
}

/// What a clinician entered, and the Reference Model values a form never
/// shows.
///
/// The envelope travels as opaque JSON. `ferrochart-compose` owns its type
/// and the renderer may not link that crate, so this module transports what a
/// caller hands it rather than keeping a second copy of a model this
/// repository already owns.
#[derive(Debug, Clone, Serialize)]
#[allow(
    dead_code,
    reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
)]
pub(crate) struct Submission<'v> {
    /// The format version this body is written in.
    format_version: u32,
    /// The Reference Model values a form never shows.
    envelope: &'v serde_json::Value,
    /// The entered values.
    values: &'v FormValues,
}

impl<'v> Submission<'v> {
    /// One submission of `values` under `envelope`.
    #[allow(
        dead_code,
        reason = "dead only outside the test configuration, whose non-test caller is the control of issue #137"
    )]
    pub(crate) fn new(envelope: &'v serde_json::Value, values: &'v FormValues) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            envelope,
            values,
        }
    }
}

impl Versioned for TemplateList {
    fn format_version(&self) -> u32 {
        self.format_version
    }
}

impl Versioned for FormDefinition {
    fn format_version(&self) -> u32 {
        self.format_version
    }
}

impl Versioned for Judgement {
    fn format_version(&self) -> u32 {
        self.format_version
    }
}

impl Versioned for Committed {
    fn format_version(&self) -> u32 {
        self.format_version
    }
}

impl Versioned for ReadBack {
    fn format_version(&self) -> u32 {
        self.format_version
    }
}

/// The templates this server holds.
///
/// # Errors
///
/// Returns the status and the body whenever the server refuses, and
/// [`ApiError::Unreachable`] when the request never reached an answer.
pub(crate) async fn templates() -> Result<TemplateList, ApiError> {
    let url = route::templates();
    read(&url).await
}

/// The form one template compiles to.
///
/// # Errors
///
/// As [`templates`].
pub(crate) async fn definition(template_id: &str) -> Result<FormDefinition, ApiError> {
    let url = route::definition(template_id);
    read(&url).await
}

/// Judges `submission` against one template without writing anything.
///
/// # Errors
///
/// As [`templates`]. A judgement that finds failures is a RESULT rather than
/// an error: the report comes back inside [`Judgement`], and only a request
/// the server refused outright becomes an [`ApiError`].
#[expect(
    dead_code,
    reason = "the controls that submit are issue #137; the expectation clears itself the moment one calls this"
)]
pub(crate) async fn validation(
    template_id: &str,
    submission: &Submission<'_>,
) -> Result<Judgement, ApiError> {
    let url = route::validation(template_id);
    write(&url, submission).await
}

/// Commits `submission` to one EHR.
///
/// # Errors
///
/// As [`templates`]. A refusal carries the server's report, and a CDR failure
/// carries the CDR's own status and body, through
/// [`crate::api::error::Refusal`].
#[expect(
    dead_code,
    reason = "the controls that commit are issue #137; the expectation clears itself the moment one calls this"
)]
pub(crate) async fn commit(
    ehr_id: &str,
    template_id: &str,
    submission: &Submission<'_>,
) -> Result<Committed, ApiError> {
    let url = route::compositions(ehr_id, template_id);
    write(&url, submission).await
}

/// The entered values one stored composition reads back into.
///
/// # Errors
///
/// As [`templates`].
#[expect(
    dead_code,
    reason = "the screen that fills a form from a stored composition is issue #137; the expectation clears itself the moment one calls this"
)]
pub(crate) async fn read_back(
    ehr_id: &str,
    uid: &str,
    template_id: &str,
) -> Result<ReadBack, ApiError> {
    let url = route::values(ehr_id, uid, template_id);
    read(&url).await
}

/// Asks `url` and reads its answer as the document `T`.
async fn read<T>(url: &str) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned + Versioned,
{
    let request = Request::get(url).header("Accept", JSON).build();
    answered(url, request).await
}

/// Posts `submission` to `url` and reads its answer as the document `T`.
#[expect(
    dead_code,
    reason = "the controls that submit are issue #137; the expectation clears itself the moment one calls this"
)]
async fn write<T>(url: &str, submission: &Submission<'_>) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned + Versioned,
{
    // The body is written here rather than through `RequestBuilder::json`, so
    // a failure to write it carries the serde cause instead of the transport
    // crate's own wrapper around it.
    let body = serde_json::to_string(submission).map_err(|source| ApiError::Unsendable {
        url: url.to_owned(),
        source,
    })?;
    let request = Request::post(url)
        .header("Content-Type", JSON)
        .header("Accept", JSON)
        .body(body);
    answered(url, request).await
}

/// Sends `request` and reads its answer as the document `T`.
///
/// A request that could not be built and one that never reached the server
/// are the same outcome, [`ApiError::Unreachable`]: no answer arrived, so
/// there is no status to report and none is invented.
async fn answered<T>(url: &str, request: Result<Request, gloo_net::Error>) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned + Versioned,
{
    let response = match request {
        Ok(built) => built.send().await,
        Err(source) => Err(source),
    }
    .map_err(|source| ApiError::Unreachable {
        url: url.to_owned(),
        source,
    })?;
    // The status and the body are read BEFORE the outcome is decided, so a
    // refusal reaches the caller with the bytes that explain it rather than
    // as a bare status.
    let status = response.status();
    let succeeded = response.ok();
    let body = response
        .text()
        .await
        .map_err(|source| ApiError::Unreadable {
            url: url.to_owned(),
            status,
            source,
        })?;
    if !succeeded {
        return Err(error::refused(url, status, body));
    }
    let document: T = error::decoded(url, status, &body)?;
    error::format_checked(url, status, document.format_version())?;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// The crate's own sources.
    fn sources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// Every `.rs` file under `root`, in path order.
    fn rust_files(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                panic!("the crate's own sources are unreadable at {directory:?}");
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|name| name == "rs") {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }

    /// The part of a source file that ships, which is everything above its
    /// test module.
    ///
    /// A test builds a transport error to assert what a reader is shown, and
    /// that opens no request. `scripts/checks/palette-utilities.sh` draws the
    /// same line at `#[cfg(test)]` for the same reason.
    fn shipped(text: &str) -> &str {
        text.split_once("\n#[cfg(test)]")
            .map_or(text, |(above, _)| above)
    }

    #[test]
    fn no_other_module_opens_a_request() {
        let sources = sources();
        let door = sources.join("api");
        let mut trespassers = Vec::new();
        for file in rust_files(&sources) {
            if file.starts_with(&door) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            let shipped = shipped(&text);
            if shipped.contains("gloo_net") || shipped.contains("gloo-net") {
                trespassers.push(file);
            }
        }
        assert!(
            trespassers.is_empty(),
            "src/api is the one module that opens a request; these name the transport too: {trespassers:?}"
        );
    }

    #[test]
    fn the_scan_would_catch_a_module_that_reached_for_the_transport() {
        let reached = "use gloo_net::http::Request;\nfn f() {}\n";
        assert!(shipped(reached).contains("gloo_net"));
        let only_in_a_test = "fn f() {}\n#[cfg(test)]\nmod tests {\n    use gloo_net::Error;\n}\n";
        assert!(!shipped(only_in_a_test).contains("gloo_net"));
    }

    #[test]
    fn the_scan_reads_the_whole_crate_rather_than_one_directory() {
        let files = rust_files(&sources());
        assert!(files.len() > 5, "the scan found only {} files", files.len());
        assert!(
            files.iter().any(|file| file.ends_with("api/mod.rs")),
            "the scan missed this very file"
        );
        assert!(
            files.iter().any(|file| file.ends_with("app.rs")),
            "the scan missed the module that would most likely reach the network"
        );
    }

    #[test]
    fn a_submission_states_the_format_version_the_server_reads() {
        let envelope = serde_json::json!({ "language": "en" });
        let values = ferrochart_form::values::FormValues::new();
        let submission = super::Submission::new(&envelope, &values);
        let written = serde_json::to_value(&submission).unwrap();
        assert_eq!(
            written.get("format_version"),
            Some(&serde_json::json!(
                ferrochart_form::definition::FORMAT_VERSION
            ))
        );
        assert_eq!(written.get("envelope"), Some(&envelope));
        assert!(written.get("values").is_some());
        // `ferrochart-server`'s `Submission` is `deny_unknown_fields`, so a
        // fourth member here would be refused with a 400.
        assert_eq!(written.as_object().map(serde_json::Map::len), Some(3));
    }
}
