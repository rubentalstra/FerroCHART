// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `ValueSet/$validate-code` against a mock server.
//!
//! All citations are HL7 FHIR R4 4.0.1
//! `valueset-operation-validate-code.html` section 4.9.15.2.
//!
//! Every code and URL below is invented for the test. No patient data.

use ferrochart_form::ids::TerminologyName;
use ferrochart_form::value::Code;
use ferrochart_term::error::TermError;
use ferrochart_term::target::ExpansionTarget;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english};

const VALUE_SET: &str = "https://fhir.example.org/ValueSet/ferro-positions";

fn target() -> ExpansionTarget {
    ExpansionTarget::Canonical(VALUE_SET.to_owned())
}

fn chosen() -> Code {
    Code::new(TerminologyName::new("SNOMED-CT"), "33586001")
}

#[tokio::test]
async fn a_member_of_the_set_confirms() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$validate-code"))
        .and(body_string_contains("\"valueCode\":\"33586001\""))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"resourceType":"Parameters","parameter":[
                {"name":"result","valueBoolean":true},
                {"name":"display","valueString":"Sitting"}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let verdict = client(&server)
        .validate_code(&target(), &chosen(), &english())
        .await
        .expect("the server answers");

    assert!(verdict.valid);
    assert_eq!(verdict.display.as_deref(), Some("Sitting"));
}

#[tokio::test]
async fn a_code_the_set_refuses_is_an_answer_rather_than_a_failure() {
    // `result` is a `1..1` boolean and `message` is "error details, if result
    // = false", so a refusal is a 200 the client reads. Turning it into an
    // error would make a wrong choice indistinguishable from a broken server.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$validate-code"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"resourceType":"Parameters","parameter":[
                {"name":"result","valueBoolean":false},
                {"name":"message","valueString":"Not in the value set"}]}"#,
        ))
        .mount(&server)
        .await;

    let verdict = client(&server)
        .validate_code(&target(), &chosen(), &english())
        .await
        .expect("the server answers");

    assert!(!verdict.valid);
    assert_eq!(verdict.message.as_deref(), Some("Not in the value set"));
}

#[tokio::test]
async fn a_server_that_does_not_hold_the_value_set_is_told_apart_from_a_refusal() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$validate-code"))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"resourceType":"OperationOutcome","issue":[{"severity":"error",
                "code":"not-found","diagnostics":"ValueSet not found"}]}"#,
        ))
        .mount(&server)
        .await;

    match client(&server)
        .validate_code(&target(), &chosen(), &english())
        .await
        .expect_err("the server holds no such value set")
    {
        TermError::UnknownValueSet { target, .. } => assert_eq!(target, VALUE_SET),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_local_code_carries_no_system_because_there_is_none_to_send() {
    // openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 scopes `local`
    // to one archetype, so there is no FHIR code system for it and the request
    // names the value set alone.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$validate-code"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"resourceType":"Parameters","parameter":[{"name":"result","valueBoolean":true}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let verdict = client(&server)
        .validate_code(
            &target(),
            &Code::new(ferrochart_form::ids::local_terminology(), "at0004"),
            &english(),
        )
        .await
        .expect("the server answers");

    assert!(verdict.valid);
    let sent = server
        .received_requests()
        .await
        .expect("the mock records requests");
    let body = String::from_utf8_lossy(&sent.first().expect("one request").body).to_string();
    assert!(!body.contains("\"name\":\"system\""), "{body}");
}
