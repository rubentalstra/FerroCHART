// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A SNOMED CT expression constraint reaches the server unread.
//!
//! FerroCHART does not parse the expression constraint language. HL7 FHIR R4
//! 4.0.1 `snomedct.html` section 4.3.1.0.9 defines `?fhir_vs=ecl/[ecl]` as
//! "all concept ids that match the supplied (URI-encoded) expression
//! constraint", and section 4.3.1.0.8.3 defines the equivalent `constraint`
//! filter whose result "is the result of executing the given SNOMED CT
//! Expression Constraint". Executing it is a terminology server's work.
//!
//! Every expression and URL below is invented for the test. No patient data.

use ferrochart_form::ids::LanguageTag;
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::target::ExpansionTarget;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english};

const EXPANSION: &str = r#"{"resourceType":"ValueSet","status":"active","expansion":{
    "timestamp":"2026-09-07T00:00:00Z","total":1,"contains":[
    {"system":"http://snomed.info/sct","code":"73211009","display":"Diabetes mellitus"}]}}"#;

/// An expression using every character that would break a hand-built URL:
/// spaces, angle brackets, pipes and a comma.
const EXPRESSION: &str = "<< 73211009 |Diabetes mellitus| MINUS << 46635009 |Type 1|";

#[tokio::test]
async fn the_expression_is_sent_as_the_implicit_value_set_and_never_parsed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EXPANSION))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .expand(
            &ExpansionTarget::SnomedExpression(EXPRESSION.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("the server answers");

    let sent = server
        .received_requests()
        .await
        .expect("the mock records requests");
    let body = String::from_utf8_lossy(&sent.first().expect("one request").body).to_string();
    assert!(
        body.contains("http://snomed.info/sct?fhir_vs=ecl/"),
        "the implicit value set of section 4.3.1.0.9 is what went out: {body}"
    );
    assert!(
        !body.contains("MINUS "),
        "the expression is URI-encoded rather than pasted in: {body}"
    );
}

#[tokio::test]
async fn one_expression_in_two_languages_is_two_cache_entries_and_two_calls() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EXPANSION))
        .expect(2)
        .mount(&server)
        .await;
    let term = client(&server);

    for language in [english(), LanguageTag::new("nl")] {
        term.expand(
            &ExpansionTarget::SnomedExpression(EXPRESSION.to_owned()),
            &language,
            &ExpandOptions::new(),
        )
        .await
        .expect("the server answers");
    }
}
