// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `ValueSet/$expand` against a mock server.
//!
//! All citations are HL7 FHIR R4 4.0.1 `valueset-operation-expand.html`
//! section 4.9.15.1 unless another page is named.
//!
//! Every code, URL and rubric below is invented for the test. No patient data.

use ferrochart_form::ids::TerminologyName;
use ferrochart_form::value::{ExpansionSource, ValueSet};
use ferrochart_term::error::TermError;
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::resolution::Origin;
use ferrochart_term::resolve::Resolver;
use ferrochart_term::target::ExpansionTarget;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english, template};

const VALUE_SET: &str = "https://fhir.example.org/ValueSet/ferro-positions";

/// An expansion of two codes, one of them nested under a grouper.
const EXPANSION: &str = r#"{
  "resourceType": "ValueSet",
  "url": "https://fhir.example.org/ValueSet/ferro-positions",
  "status": "active",
  "expansion": {
    "timestamp": "2026-09-07T00:00:00Z",
    "total": 3,
    "contains": [
      {"system": "http://snomed.info/sct", "code": "33586001", "display": "Sitting"},
      {"system": "http://snomed.info/sct", "code": "10904000", "display": "Standing",
       "contains": [
         {"system": "http://snomed.info/sct", "code": "102538003", "display": "Recumbent"}
       ]}
    ]
  }
}"#;

fn expand_mock() -> Mock {
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .and(header("Content-Type", "application/fhir+json"))
        .and(header("Accept", "application/fhir+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/fhir+json")
                .set_body_string(EXPANSION),
        )
}

#[tokio::test]
async fn a_reference_set_binding_expands_over_the_wire() {
    let server = MockServer::start().await;
    expand_mock().expect(1).mount(&server).await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Expansion(ExpansionSource::ReferenceSet {
        uri: VALUE_SET.to_owned(),
    });

    let resolution = resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");

    assert_eq!(resolution.origin, Origin::Server);
    let codes: Vec<&str> = resolution
        .codes
        .iter()
        .map(|code| code.code.code.as_str())
        .collect();
    assert_eq!(codes, ["33586001", "10904000", "102538003"]);
    assert_eq!(
        resolution.codes.first().and_then(|c| c.display.as_deref()),
        Some("Sitting")
    );
}

#[tokio::test]
async fn the_expansion_carries_the_system_the_server_named_rather_than_the_openehr_one() {
    // `ValueSet.expansion.contains.system` is the FHIR code system, and the
    // composition stores what the picker emits, so the code has to come back
    // under the system the server answered with.
    let server = MockServer::start().await;
    expand_mock().mount(&server).await;
    let resolution = client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("the server expands the set");

    assert!(
        resolution
            .codes
            .iter()
            .all(|code| code.code.terminology == TerminologyName::new("http://snomed.info/sct"))
    );
}

#[tokio::test]
async fn a_paged_expansion_says_the_set_is_larger_than_the_page() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"resourceType":"ValueSet","status":"active","expansion":{
                "timestamp":"2026-09-07T00:00:00Z","total":50,"contains":[
                {"system":"http://snomed.info/sct","code":"33586001","display":"Sitting"}]}}"#,
        ))
        .mount(&server)
        .await;

    let resolution = client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new().with_count(1),
        )
        .await
        .expect("the server expands one page");

    assert_eq!(resolution.total, Some(50));
    assert!(resolution.is_paged(), "one of fifty codes is a page");
}

#[tokio::test]
async fn a_filter_and_an_offset_reach_the_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .and(body_string_contains("\"valueString\":\"abdo\""))
        .and(body_string_contains("\"name\":\"offset\""))
        .respond_with(ResponseTemplate::new(200).set_body_string(EXPANSION))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .expand(
            &ExpansionTarget::Canonical(VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new().with_filter("abdo").at_offset(40),
        )
        .await
        .expect("the server expands the filtered page");
}

#[tokio::test]
async fn a_binding_with_no_server_configured_is_a_typed_error_and_not_an_empty_picker() {
    let resolver = Resolver::local();
    let set = ValueSet::Expansion(ExpansionSource::ReferenceSet {
        uri: VALUE_SET.to_owned(),
    });

    match resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect_err("nothing local can answer this")
    {
        TermError::NoServer { target } => assert_eq!(target, VALUE_SET),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_reference_set_uri_no_fhir_request_can_name_is_reported_verbatim() {
    // This exact shape is in `corpus/templates`. No openEHR specification
    // defines it, so the client says so on the field rather than sending a
    // request it invented.
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));
    let uri = "terminology:SNOMEDCT?subset=%20LOINC%20Answer%20List%20LL2338-3&language=en-GB";
    let set = ValueSet::Expansion(ExpansionSource::ReferenceSet {
        uri: uri.to_owned(),
    });

    match resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect_err("no FHIR request names this")
    {
        TermError::UnresolvableBinding { uri: reported } => assert_eq!(reported, uri),
        other => panic!("the failure is {other:?}"),
    }
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "a binding the client cannot read still went to the server"
    );
}
