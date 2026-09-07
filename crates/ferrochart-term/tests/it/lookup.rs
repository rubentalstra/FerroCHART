// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `CodeSystem/$lookup` against a mock server.
//!
//! All citations are HL7 FHIR R4 4.0.1 `codesystem-operation-lookup.html`
//! section 4.8.21.1.
//!
//! Every code, URL and rubric below is invented for the test. No patient data.

use ferrochart_form::ids::TerminologyName;
use ferrochart_form::value::{EnumeratedSet, ValueSet};
use ferrochart_term::error::TermError;
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::resolution::Origin;
use ferrochart_term::resolve::Resolver;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english, option, template};

/// The out parameters the operation declares, and nothing else.
fn answer(display: &str) -> String {
    format!(
        r#"{{"resourceType":"Parameters","parameter":[
            {{"name":"name","valueString":"Ferro Synthetic Codes"}},
            {{"name":"version","valueString":"1.0.0"}},
            {{"name":"display","valueString":"{display}"}}]}}"#
    )
}

#[tokio::test]
async fn an_enumerated_external_code_gets_its_display_text_from_the_server() {
    // `docs/architecture.md` section 7: an enumerated external code carries no
    // rubric in the template, so membership stays a template question and only
    // the text is a server one.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/CodeSystem/$lookup"))
        .and(body_string_contains("\"valueUri\":\"http://loinc.org\""))
        .respond_with(ResponseTemplate::new(200).set_body_string(answer("Bicarbonate")))
        .expect(1)
        .mount(&server)
        .await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Enumerated(EnumeratedSet {
        terminology: TerminologyName::new("LOINC(2.65)"),
        options: vec![option("LOINC", "1963-8", None)],
        needs_display_lookup: true,
    });

    let resolution = resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect("the server names the code");

    assert_eq!(resolution.origin, Origin::TemplateDisplayedByServer);
    assert_eq!(
        resolution
            .codes
            .first()
            .and_then(|code| code.display.as_deref()),
        Some("Bicarbonate")
    );
}

#[tokio::test]
async fn a_code_the_template_already_names_is_never_looked_up() {
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Enumerated(EnumeratedSet {
        terminology: TerminologyName::new("LOINC"),
        options: vec![option("LOINC", "1963-8", Some("Bicarbonate"))],
        needs_display_lookup: false,
    });

    let resolution = resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect("the template already names the code");

    assert_eq!(resolution.origin, Origin::Template);
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "a code the template names was looked up anyway"
    );
}

#[tokio::test]
async fn a_versioned_openehr_terminology_sends_the_system_version_it_names() {
    // openEHR BASE Release-1.2.0 `base_types.html` section 5 spells a
    // `TERMINOLOGY_ID` as `name [ '(' version ')' ]`, and the corpus states
    // `LOINC(2.65)`, so the version reaches the wire as the operation's own
    // `version` parameter rather than as part of the system URI.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/CodeSystem/$lookup"))
        .and(body_string_contains("\"valueString\":\"2.65\""))
        .respond_with(ResponseTemplate::new(200).set_body_string(answer("Bicarbonate")))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .lookup(&TerminologyName::new("LOINC(2.65)"), "1963-8", &english())
        .await
        .expect("the server names the code");
}

#[tokio::test]
async fn a_terminology_with_no_configured_system_is_a_typed_error() {
    // FHIR R4 `terminologies-systems.html` section 4.3.0 defers ICD-10 to the
    // national modification, so there is no single URI to send and guessing
    // one would ask about the wrong code system.
    let server = MockServer::start().await;

    match client(&server)
        .lookup(&TerminologyName::new("ICD10"), "J18.9", &english())
        .await
        .expect_err("no FHIR system URI is configured")
    {
        TermError::UnknownSystem { terminology } => assert_eq!(terminology.as_str(), "ICD10"),
        other => panic!("the failure is {other:?}"),
    }
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "a terminology with no system still went to the server"
    );
}

#[tokio::test]
async fn a_deployer_configured_system_is_used() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/CodeSystem/$lookup"))
        .and(body_string_contains(
            "\"valueUri\":\"http://hl7.org/fhir/sid/icd-10\"",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(answer("Pneumonia, unspecified")))
        .expect(1)
        .mount(&server)
        .await;

    let concept = client(&server)
        .with_system(
            &TerminologyName::new("ICD10"),
            "http://hl7.org/fhir/sid/icd-10",
        )
        .lookup(&TerminologyName::new("ICD10"), "J18.9", &english())
        .await
        .expect("the configured system is sent");

    assert_eq!(concept.display, "Pneumonia, unspecified");
}

#[tokio::test]
async fn a_display_lookup_that_fails_is_reported_rather_than_leaving_a_bare_code() {
    // The specification answers an unknown code with 400 and an
    // `OperationOutcome`. Swallowing that would show the clinician a picker of
    // raw codes with no sign anything went wrong.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/CodeSystem/$lookup"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"resourceType":"OperationOutcome","issue":[{"severity":"error",
                "code":"code-invalid","diagnostics":"Unknown code '1963-8'"}]}"#,
        ))
        .mount(&server)
        .await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Enumerated(EnumeratedSet {
        terminology: TerminologyName::new("LOINC"),
        options: vec![option("LOINC", "1963-8", None)],
        needs_display_lookup: true,
    });

    match resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect_err("the server does not know the code")
    {
        TermError::Refused { diagnostics, .. } => {
            assert!(diagnostics.is_some_and(|text| text.contains("1963-8")));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn an_enumerated_external_code_keeps_its_membership_when_no_server_is_configured() {
    // Without a server the field still permits exactly what the template says;
    // only the display text is absent, and absent is not empty.
    let resolution = Resolver::local()
        .resolve(
            &template(),
            &ValueSet::Enumerated(EnumeratedSet {
                terminology: TerminologyName::new("LOINC"),
                options: vec![option("LOINC", "1963-8", None)],
                needs_display_lookup: true,
            }),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("membership comes from the template");

    assert_eq!(resolution.origin, Origin::Template);
    assert_eq!(resolution.codes.len(), 1);
    assert_eq!(
        resolution
            .codes
            .first()
            .and_then(|code| code.display.as_deref()),
        None
    );
}
