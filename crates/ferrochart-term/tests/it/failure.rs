// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What each upstream status becomes, and that nothing becomes an empty
//! picker.
//!
//! HL7 FHIR R4 4.0.1 `http.html` section 3.1.0.4.2 lists the statuses a server
//! rejects with, and `operations.html` section 3.2.0.6.2 says an error
//! "SHOULD" carry an `OperationOutcome`, so a body that is not one is still
//! conformant and the raw text is kept either way.
//!
//! Every code and URL below is invented for the test. No patient data.

use ferrochart_term::error::TermError;
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::target::ExpansionTarget;
use http::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english};

const VALUE_SET: &str = "https://fhir.example.org/ValueSet/ferro-positions";

/// An `OperationOutcome` saying `text`.
fn outcome(code: &str, text: &str) -> String {
    format!(
        r#"{{"resourceType":"OperationOutcome","issue":[{{"severity":"error",
            "code":"{code}","diagnostics":"{text}"}}]}}"#
    )
}

async fn expand_against(status: u16, body: &str) -> TermError {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect_err("the server refused")
}

#[tokio::test]
async fn a_value_set_the_server_does_not_hold_names_itself() {
    match expand_against(404, &outcome("not-found", "ValueSet not found")).await {
        TermError::UnknownValueSet {
            status,
            target,
            diagnostics,
            ..
        } => {
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(target, VALUE_SET);
            assert_eq!(diagnostics.as_deref(), Some("ValueSet not found"));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_404_that_is_not_an_operation_outcome_reads_as_an_operation_the_server_lacks() {
    // A FHIR server that does not offer `$expand` answers 404 on the endpoint
    // itself, which is a capability answer rather than a missing value set,
    // and the two have to be told apart on the field.
    match expand_against(404, "<html>Not Found</html>").await {
        TermError::Unimplemented { status, body } => {
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert!(body.contains("Not Found"));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn an_expansion_the_server_refuses_as_too_costly_carries_what_it_said() {
    // The specification names `too-costly` as the outcome code for an
    // expansion that is too large, and a form has to say so rather than show
    // an empty picker.
    match expand_against(
        422,
        &outcome("too-costly", "The expansion has 500000 concepts"),
    )
    .await
    {
        TermError::Refused {
            status,
            diagnostics,
            body,
            ..
        } => {
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(diagnostics.is_some_and(|text| text.contains("500000")));
            assert!(body.contains("too-costly"), "the body is kept verbatim");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_422_saying_the_value_set_is_not_found_is_a_missing_value_set() {
    // A conformant server may answer either status for a value set it does not
    // hold, so the outcome text is what separates "I do not have it" from "I
    // will not do it".
    match expand_against(
        422,
        &outcome(
            "not-found",
            "ValueSet not found: http://snomed.info/sct?fhir_vs=ecl/x",
        ),
    )
    .await
    {
        TermError::UnknownValueSet { status, .. } => {
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_refused_request_keeps_its_body_even_where_it_is_not_an_operation_outcome() {
    match expand_against(400, "the request is not valid FHIR").await {
        TermError::Refused {
            diagnostics, body, ..
        } => {
            assert_eq!(diagnostics, None);
            assert_eq!(body, "the request is not valid FHIR");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_server_that_is_down_is_not_a_server_that_refused() {
    match expand_against(503, "upstream unavailable").await {
        TermError::Unavailable { status, body } => {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body, "upstream unavailable");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_challenge_is_carried_rather_than_interpreted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("WWW-Authenticate", "Bearer realm=\"tx\"")
                .set_body_string("no credentials"),
        )
        .mount(&server)
        .await;

    match client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect_err("the server wants credentials")
    {
        TermError::Unauthorized { status, challenge } => {
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(challenge.as_deref(), Some("Bearer realm=\"tx\""));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_status_the_specification_does_not_publish_still_arrives_typed() {
    match expand_against(429, "slow down").await {
        TermError::Upstream { status, body, .. } => {
            assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(body, "slow down");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_body_that_is_not_the_resource_the_call_reads_is_reported_by_its_element() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"resourceType":"Patient","id":"ferro"}"#),
        )
        .mount(&server)
        .await;

    match client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect_err("a Patient is not a ValueSet")
    {
        TermError::Body { expected, .. } => assert_eq!(expected, "a ValueSet"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_body_that_is_not_json_at_all_says_so() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<ValueSet/>"))
        .mount(&server)
        .await;

    assert!(matches!(
        client(&server)
            .expand(
                &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
                &english(),
                &ExpandOptions::new(),
            )
            .await,
        Err(TermError::NotJson { .. })
    ));
}
