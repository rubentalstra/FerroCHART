// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! How a failed read reaches the reader.
//!
//! `docs/architecture.md` section 10.1: a read renders its error inline. The
//! account is built by [`detail`], which is a pure function over the error,
//! so the one thing that decides what a clinician is told is covered by a
//! native test rather than by a browser the workspace run does not have.
//!
//! Nothing here summarizes a failure away. The status, the server's stable
//! code, the CDR's own answer where there was one, and every link of the
//! cause chain are all written out, because the reader is the person who has
//! to say what went wrong to somebody else.

use leptos::prelude::*;

use crate::api::error::ApiError;
use crate::kit::notice::{Notice, Tone};

/// How much of an answer's body is shown before it is cut.
///
/// A CDR's error body runs to kilobytes and the screen is an account rather
/// than a log. The cut is marked, so nobody reads a truncated body as the
/// whole one.
const BODY_SHOWN: usize = 400;

/// Every line the reader is shown about `error`.
pub(crate) fn detail(error: &ApiError) -> Vec<String> {
    let mut lines = vec![error.to_string()];
    lines.push(match error.status() {
        Some(status) => format!("{} answered {status}", error.url()),
        None => format!("{} never answered", error.url()),
    });
    if let Some(refusal) = error.refusal() {
        lines.push(format!("the server reports `{}`", refusal.error));
        match refusal.cdr_status {
            Some(status) => lines.push(format!("the CDR answered {status}")),
            None => {
                if refusal.cdr_body.is_some() {
                    lines.push("the CDR never answered".to_owned());
                }
            }
        }
        if let Some(body) = refusal.cdr_body.as_deref() {
            lines.push(format!("the CDR said: {}", clipped(body)));
        }
        if let Some(report) = refusal.report.as_ref() {
            lines.push(format!("{} failures were reported", report.failures.len()));
        }
    }
    if let Some(body) = error.body() {
        lines.push(format!("the body was: {}", clipped(body)));
    }
    let mut cause = std::error::Error::source(error);
    while let Some(link) = cause {
        lines.push(format!("caused by: {link}"));
        cause = link.source();
    }
    lines
}

/// `text`, cut to what a screen shows, with the cut marked.
fn clipped(text: &str) -> String {
    // Counting characters rather than bytes: a CDR's message is prose in the
    // composition's own language, and a byte cut inside a multi-byte
    // character is the panic `.claude/rules/reliability.md` denies.
    if text.chars().count() <= BODY_SHOWN {
        return text.to_owned();
    }
    let kept: String = text.chars().take(BODY_SHOWN).collect();
    format!("{kept}…")
}

/// The inline account of a read that failed.
#[component]
pub(crate) fn Failed(
    /// The one line a reader takes away, naming what could not be read.
    #[prop(into)]
    title: String,
    /// What went wrong.
    lines: Vec<String>,
) -> impl IntoView {
    let written: Vec<_> = lines
        .into_iter()
        .map(|line| view! { <li>{line}</li> })
        .collect();
    view! {
        <Notice tone=Tone::Danger title=title>
            <ul class="space-y-0.5">{written}</ul>
        </Notice>
    }
}

#[cfg(test)]
mod tests {
    use crate::api::error::{ApiError, refused};

    use super::{BODY_SHOWN, clipped, detail};

    fn joined(error: &ApiError) -> String {
        detail(error).join("\n")
    }

    #[test]
    fn a_transport_failure_says_that_nothing_answered_rather_than_a_status() {
        let error = ApiError::Unreachable {
            url: "/api/templates".to_owned(),
            source: gloo_net::Error::GlooError("the connection was refused".to_owned()),
        };
        let shown = joined(&error);
        assert!(shown.contains("/api/templates never answered"), "{shown}");
        assert!(
            shown.contains("caused by: the connection was refused"),
            "{shown}"
        );
    }

    #[test]
    fn a_refusal_shows_the_stable_code_the_server_sent() {
        let body = r#"{"error":"unknown_template","message":"this server holds no template `x`"}"#;
        let shown = joined(&refused(
            "/api/templates/x/definition",
            404,
            body.to_owned(),
        ));
        assert!(shown.contains("answered 404"), "{shown}");
        assert!(
            shown.contains("the server reports `unknown_template`"),
            "{shown}"
        );
    }

    #[test]
    fn a_cdr_that_never_answered_is_reported_as_such_rather_than_as_a_status() {
        let body = r#"{"error":"cdr_unreachable","message":"the request to the CDR did not succeed","cdr_status":null,"cdr_body":"connection refused"}"#;
        let shown = joined(&refused("/api/x", 502, body.to_owned()));
        assert!(shown.contains("the CDR never answered"), "{shown}");
        assert!(
            shown.contains("the CDR said: connection refused"),
            "{shown}"
        );
        assert!(!shown.contains("the CDR answered"), "{shown}");
    }

    #[test]
    fn a_cdr_status_the_server_did_send_is_shown() {
        let body =
            r#"{"error":"cdr_rejected","message":"refused","cdr_status":422,"cdr_body":"{}"}"#;
        let shown = joined(&refused("/api/x", 502, body.to_owned()));
        assert!(shown.contains("the CDR answered 422"), "{shown}");
    }

    #[test]
    fn a_body_this_client_could_not_read_is_shown_rather_than_swallowed() {
        let shown = joined(&refused(
            "/api/templates",
            503,
            "<html>gateway</html>".to_owned(),
        ));
        assert!(
            shown.contains("the body was: <html>gateway</html>"),
            "{shown}"
        );
    }

    #[test]
    fn a_long_body_is_cut_at_a_character_and_the_cut_is_marked() {
        let long = "é".repeat(BODY_SHOWN + 50);
        let cut = clipped(&long);
        assert!(cut.ends_with('…'), "the cut is unmarked");
        assert_eq!(cut.chars().count(), BODY_SHOWN + 1);
    }

    #[test]
    fn a_short_body_is_shown_whole() {
        assert_eq!(clipped("short"), "short");
    }

    #[test]
    fn a_judgement_says_how_many_failures_came_with_it() {
        let body = r#"{"error":"refused","message":"no","report":{"failures":[{"key":null,"path":"/x","message":"m","kind":"required","source":"template"}]}}"#;
        let shown = joined(&refused("/api/x", 422, body.to_owned()));
        assert!(shown.contains("1 failures were reported"), "{shown}");
    }

    #[test]
    fn every_account_opens_with_what_failed_and_names_the_address() {
        let error = refused("/api/templates", 500, "boom".to_owned());
        let lines = detail(&error);
        assert_eq!(
            lines.first().map(String::as_str),
            Some(error.to_string().as_str())
        );
        assert!(lines.iter().any(|line| line.contains("/api/templates")));
    }
}
