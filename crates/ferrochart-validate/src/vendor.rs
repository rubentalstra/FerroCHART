// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Reading a CDR's own refusal, best-effort and vendor-specific.
//!
//! NOTE: openEHR ITS-REST Release-1.1.0 `overview.html` makes an error body
//! optional, conditional on `Prefer: return=representation`, and carries no
//! path on an error entry, so everything below is a guess at one vendor's
//! prose and never a conformance judgement.
//!
//! Nothing here is a substitute for the gate. A CDR refusing a COMPOSITION
//! FerroCHART built and validated is a FerroCHART defect (`CLAUDE.md`), so
//! what this module produces is diagnostic material for that defect, shown
//! beside the form so a clinician is not left with a blank refusal.

use ferrochart_form::validation::{FailureKind, FailureSource, ValidationFailure};
use serde::Deserialize;
use serde_json::Value;

use crate::index::KeyIndex;
use crate::path;

/// The two error-body shapes this reader knows.
///
/// NOTE: neither is normative. The first is the `Error` component schema of
/// the ITS-REST interface bundle (`message` plus `validationErrors`), and the
/// second is the `errors` array of `DV_CODED_TEXT` that `overview.html`
/// describes in prose; a CDR may send either, both, or neither.
#[derive(Debug, Deserialize)]
struct Refusal {
    #[serde(default)]
    message: Option<String>,
    #[serde(default, rename = "validationErrors")]
    validation_errors: Vec<Value>,
    #[serde(default)]
    errors: Vec<Value>,
}

/// Every failure a CDR's error body states, keyed where a path can be read out
/// of it.
///
/// A body this reader cannot parse yields one failure carrying the body
/// verbatim, because a refusal absorbed into an empty list would read as a
/// success (`.claude/rules/reliability.md`).
#[must_use]
pub fn failures(body: &str, index: &KeyIndex, instance: &Value) -> Vec<ValidationFailure> {
    let Ok(refusal) = serde_json::from_str::<Refusal>(body) else {
        return vec![unkeyed(body.to_owned())];
    };
    let mut out = Vec::new();
    for entry in refusal.validation_errors.iter().chain(&refusal.errors) {
        out.push(placed(&text_of(entry), index, instance));
    }
    if out.is_empty() {
        let stated = refusal.message.unwrap_or_else(|| body.to_owned());
        out.push(unkeyed(stated));
    }
    out
}

/// The prose one error entry carries.
///
/// NOTE: `overview.html` shapes an entry as `DV_CODED_TEXT`, whose `value` is
/// the display text (openEHR RM Release-1.1.0 `data_types.html` section
/// 4.2.3), and the interface bundle shapes it as a bare string, so both are
/// read.
fn text_of(entry: &Value) -> String {
    if let Some(text) = entry.as_str() {
        return text.to_owned();
    }
    entry
        .get("value")
        .and_then(Value::as_str)
        .map_or_else(|| entry.to_string(), str::to_owned)
}

/// One entry, keyed onto a field where its prose names a path.
fn placed(text: &str, index: &KeyIndex, instance: &Value) -> ValidationFailure {
    let Some(quoted) = quoted_path(text) else {
        return unkeyed(text.to_owned());
    };
    let normalized = path::against_instance(&quoted, instance);
    ValidationFailure {
        key: index.resolve(&normalized).cloned(),
        path: quoted,
        message: text.to_owned(),
        kind: FailureKind::Other,
        source: FailureSource::Cdr,
    }
}

/// A failure the reader could place nowhere.
fn unkeyed(message: String) -> ValidationFailure {
    ValidationFailure {
        key: None,
        path: String::new(),
        message,
        kind: FailureKind::Other,
        source: FailureSource::Cdr,
    }
}

/// The longest run in `text` that reads as a path with at least one predicate.
///
/// A run starts at a `/` and ends at the first whitespace or quote outside a
/// predicate, so a pinned name holding a space stays inside its own step. A
/// run with no predicate is not a path: `/content` alone names an attribute
/// and would resolve onto whichever node happens to sit under it.
///
/// NOTE: vendor-specific. openEHR ITS-REST Release-1.1.0 publishes no path on
/// an error entry, so a path can only be recognised inside the prose, and a
/// CDR that writes none yields nothing here.
fn quoted_path(text: &str) -> Option<String> {
    let mut best: Option<String> = None;
    let mut current = String::new();
    let mut depth = 0_usize;
    for ch in text.chars() {
        match ch {
            '/' if current.is_empty() => current.push(ch),
            '[' if !current.is_empty() => {
                depth = depth.saturating_add(1);
                current.push(ch);
            }
            ']' if !current.is_empty() => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            _ if current.is_empty() => {}
            _ if depth > 0 => current.push(ch),
            '"' => take(&mut current, &mut best),
            _ if ch.is_whitespace() => take(&mut current, &mut best),
            _ => current.push(ch),
        }
    }
    take(&mut current, &mut best);
    best
}

/// Keeps `current` as the best candidate where it is a path and the longest
/// one seen, and clears it either way.
fn take(current: &mut String, best: &mut Option<String>) {
    if current.contains('[')
        && best
            .as_ref()
            .is_none_or(|found| current.len() > found.len())
    {
        *best = Some(current.clone());
    }
    current.clear();
}

#[cfg(test)]
mod tests {
    use ferrochart_form::validation::FailureSource;
    use serde_json::json;

    use super::{failures, quoted_path};
    use crate::index::KeyIndex;

    #[test]
    fn a_body_that_does_not_parse_is_reported_verbatim_rather_than_dropped() {
        let read = failures("<html>502</html>", &KeyIndex::default(), &json!({}));
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].message, "<html>502</html>");
        assert_eq!(read[0].source, FailureSource::Cdr);
        assert!(read[0].key.is_none());
    }

    #[test]
    fn the_interface_error_shape_yields_one_failure_per_validation_error() {
        let body = json!({
            "message": "invalid composition",
            "validationErrors": [
                "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001] is required",
                "the composer is missing",
            ],
        })
        .to_string();
        let read = failures(&body, &KeyIndex::default(), &json!({}));
        assert_eq!(read.len(), 2);
        assert_eq!(
            read[0].path,
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]"
        );
        assert!(read[1].path.is_empty());
    }

    #[test]
    fn a_coded_text_error_entry_is_read_through_its_display_value() {
        let body = json!({
            "errors": [
                { "value": "/items[at0004] is out of range",
                  "defining_code": { "terminology_id": { "value": "local" },
                                     "code_string": "V-4711" } },
            ],
        })
        .to_string();
        let read = failures(&body, &KeyIndex::default(), &json!({}));
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].path, "/items[at0004]");
    }

    #[test]
    fn a_body_with_no_entries_still_reports_the_refusal() {
        let body = json!({ "message": "the underlying template is not known" }).to_string();
        let read = failures(&body, &KeyIndex::default(), &json!({}));
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].message, "the underlying template is not known");
    }

    #[test]
    fn prose_with_no_predicate_yields_no_path() {
        assert!(quoted_path("the value 12 is out of range").is_none());
        assert!(quoted_path("at /content the type is wrong").is_none());
    }
}
