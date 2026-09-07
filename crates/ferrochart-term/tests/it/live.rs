// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The client against a running FHIR terminology server.
//!
//! A mock answers whatever it is told to, so it proves the client sends the
//! right request and reads the right response, and nothing about whether a
//! real server agrees. These cases run against one.
//!
//! They are `#[ignore]`d, because they need a server: `scripts/test-term.sh`
//! points `FERROCHART_TEST_TERM_URL` at HL7's public R4 terminology service by
//! default, and at the compose demo profile with `--ferroterm`. An ignored
//! case reports as ignored rather than as a pass, so a run without a server
//! cannot be mistaken for one with it.
//!
//! Everything asked for here is published by HL7 FHIR R4 4.0.1 itself, so the
//! cases are read-only and small, and they carry no patient data.

use std::env;

use ferrochart_form::ids::TerminologyName;
use ferrochart_form::value::Code;
use ferrochart_term::client::TermClient;
use ferrochart_term::error::TermError;
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::resolution::Origin;
use ferrochart_term::target::ExpansionTarget;
use url::Url;

/// The base URL of the terminology server under test.
const TERM_URL: &str = "FERROCHART_TEST_TERM_URL";

/// The `Authorization` header value, where the server under test wants one.
const TERM_AUTHORIZATION: &str = "FERROCHART_TEST_TERM_AUTHORIZATION";

/// A value set every R4 server holds: it is published by the specification.
const GENDER_VALUE_SET: &str = "http://hl7.org/fhir/ValueSet/administrative-gender";

/// The code system behind it.
const GENDER_SYSTEM: &str = "http://hl7.org/fhir/administrative-gender";

/// A canonical URL no server holds.
const ABSENT: &str = "http://ferro.example/ValueSet/does-not-exist";

fn client() -> TermClient {
    let base = env::var(TERM_URL)
        .unwrap_or_else(|_| panic!("{TERM_URL} is unset; run scripts/test-term.sh"));
    let base = Url::parse(&base).expect("the terminology base is a URL");
    let client = TermClient::new(&base).expect("the client builds");
    match env::var(TERM_AUTHORIZATION) {
        Ok(value) => client.with_authorization(value),
        Err(_) => client,
    }
}

fn english() -> ferrochart_form::ids::LanguageTag {
    ferrochart_form::ids::LanguageTag::new("en")
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_expands_a_value_set_the_specification_publishes() {
    let resolution = client()
        .expand(
            &ExpansionTarget::Canonical(GENDER_VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("the server expands a value set R4 publishes");

    assert_eq!(resolution.origin, Origin::Server);
    let codes: Vec<&str> = resolution
        .codes
        .iter()
        .map(|code| code.code.code.as_str())
        .collect();
    assert!(codes.contains(&"female"), "the expansion is {codes:?}");
    assert!(
        resolution.codes.iter().all(|code| code.display.is_some()),
        "every member came back with a display text"
    );
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_names_a_code_the_template_carries_no_rubric_for() {
    let concept = client()
        .lookup(&TerminologyName::new(GENDER_SYSTEM), "female", &english())
        .await
        .expect("the server names the code");

    assert_eq!(concept.display, "Female");
    assert!(
        !concept.system_name.is_empty(),
        "the server named the system"
    );
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_confirms_a_member_and_refuses_a_non_member() {
    // The pair is the point: a member and a non-member are both 200 answers
    // with different `result` values, so a wrong choice is never confused with
    // a broken server.
    let term = client();
    let target = ExpansionTarget::Canonical(GENDER_VALUE_SET.to_owned());
    let system = TerminologyName::new(GENDER_SYSTEM);

    let member = term
        .validate_code(&target, &Code::new(system.clone(), "female"), &english())
        .await
        .expect("the server answers");
    assert!(member.valid, "female is in the gender value set");

    let stranger = term
        .validate_code(&target, &Code::new(system, "ferro-not-a-code"), &english())
        .await
        .expect("the server answers");
    assert!(!stranger.valid, "an invented code is not in the value set");
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_reports_an_absent_value_set_as_absent_rather_than_as_empty() {
    // The acceptance criterion this case exists for: an unresolvable binding
    // is a typed error carrying the upstream status and body, never an empty
    // picker.
    match client()
        .expand(
            &ExpansionTarget::Canonical(ABSENT.to_owned()),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect_err("no server holds this value set")
    {
        TermError::UnknownValueSet { target, body, .. } => {
            assert_eq!(target, ABSENT);
            assert!(!body.is_empty(), "the server said why");
        }
        TermError::Refused { body, .. } => {
            assert!(!body.is_empty(), "the server said why");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_receives_a_snomed_expression_as_the_implicit_value_set() {
    // The point is the shape, not the answer. FerroCHART never parses the
    // expression, so what has to hold is that a real server takes the URL as
    // one canonical value set. A server without a SNOMED CT release answers
    // that it holds no such value set and echoes the URL it was given, which
    // is the same evidence.
    let expression = "<<73211009";
    match client()
        .expand(
            &ExpansionTarget::SnomedExpression(expression.to_owned()),
            &english(),
            &ExpandOptions::new().with_count(5),
        )
        .await
    {
        Ok(resolution) => assert!(
            !resolution.codes.is_empty(),
            "a server with SNOMED CT loaded expands the expression"
        ),
        Err(TermError::UnknownValueSet { body, .. } | TermError::Refused { body, .. }) => {
            assert!(
                body.contains("fhir_vs=ecl/%3C%3C73211009"),
                "the server decoded the implicit value set FerroCHART sent: {body}"
            );
        }
        Err(other) => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running terminology server: scripts/test-term.sh"]
async fn a_real_server_answers_a_page_of_a_larger_expansion() {
    let resolution = client()
        .expand(
            &ExpansionTarget::Canonical(GENDER_VALUE_SET.to_owned()),
            &english(),
            &ExpandOptions::new().with_count(1),
        )
        .await
        .expect("the server expands one page");

    assert_eq!(resolution.codes.len(), 1, "the page holds one code");
}
