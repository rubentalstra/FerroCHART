// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The expansion cache, per template and language.
//!
//! No specification governs caching a terminology expansion: our own design,
//! and the key is documented on `ferrochart_term::cache::CacheKey`. These
//! cases pin what the key separates and what it does not.
//!
//! Every code and URL below is invented for the test. No patient data.

use ferrochart_form::ids::{LanguageTag, TemplateId};
use ferrochart_form::value::{ExpansionSource, ValueSet};
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::resolve::Resolver;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{client, english, template};

const VALUE_SET: &str = "https://fhir.example.org/ValueSet/ferro-positions";

const EXPANSION: &str = r#"{"resourceType":"ValueSet","status":"active","expansion":{
    "timestamp":"2026-09-07T00:00:00Z","total":1,"contains":[
    {"system":"http://snomed.info/sct","code":"33586001","display":"Sitting"}]}}"#;

fn set() -> ValueSet {
    ValueSet::Expansion(ExpansionSource::ReferenceSet {
        uri: VALUE_SET.to_owned(),
    })
}

async fn server_expecting(calls: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EXPANSION))
        .expect(calls)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn the_same_field_in_the_same_language_is_expanded_once() {
    let server = server_expecting(1).await;
    let resolver = Resolver::with_server(client(&server));

    for _ in 0..3_u8 {
        resolver
            .resolve(&template(), &set(), &english(), &ExpandOptions::new())
            .await
            .expect("the server expands the set");
    }

    assert_eq!(resolver.cache().len().expect("the cache answers"), 1);
}

#[tokio::test]
async fn two_languages_of_one_value_set_are_two_expansions() {
    // `displayLanguage` changes the display text of every member, so serving
    // one language's answer for another would show a clinician the wrong
    // words under the right codes.
    let server = server_expecting(2).await;
    let resolver = Resolver::with_server(client(&server));

    resolver
        .resolve(&template(), &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");
    resolver
        .resolve(
            &template(),
            &set(),
            &LanguageTag::new("nl"),
            &ExpandOptions::new(),
        )
        .await
        .expect("the server expands the set");

    assert_eq!(resolver.cache().len().expect("the cache answers"), 2);
}

#[tokio::test]
async fn a_filtered_expansion_is_never_served_from_an_unfiltered_one() {
    let server = server_expecting(2).await;
    let resolver = Resolver::with_server(client(&server));

    resolver
        .resolve(&template(), &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");
    resolver
        .resolve(
            &template(),
            &set(),
            &english(),
            &ExpandOptions::new().with_filter("sit"),
        )
        .await
        .expect("the server expands the filtered set");

    assert_eq!(resolver.cache().len().expect("the cache answers"), 2);
}

#[tokio::test]
async fn two_templates_binding_one_url_are_two_entries() {
    // A template is recompiled as a whole and a revision can bind the same
    // `ac`-code elsewhere, so entries are dropped per template rather than per
    // URL.
    let server = server_expecting(2).await;
    let resolver = Resolver::with_server(client(&server));
    let other = TemplateId::new("openEHR-EHR-COMPOSITION.ferro_other.v1.0.0");

    resolver
        .resolve(&template(), &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");
    resolver
        .resolve(&other, &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");

    assert_eq!(resolver.cache().len().expect("the cache answers"), 2);
    assert_eq!(
        resolver
            .cache()
            .forget_template(&template())
            .expect("the cache answers"),
        1
    );
    assert_eq!(resolver.cache().len().expect("the cache answers"), 1);
}

#[tokio::test]
async fn recompiling_a_template_makes_the_next_resolution_ask_again() {
    let server = server_expecting(2).await;
    let resolver = Resolver::with_server(client(&server));

    resolver
        .resolve(&template(), &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set");
    resolver
        .cache()
        .forget_template(&template())
        .expect("the cache answers");
    resolver
        .resolve(&template(), &set(), &english(), &ExpandOptions::new())
        .await
        .expect("the server expands the set again");
}

#[tokio::test]
async fn a_failed_expansion_is_never_cached() {
    // Caching a refusal would keep answering with it after the server
    // recovered, and the caller could not tell.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ValueSet/$expand"))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .expect(2)
        .mount(&server)
        .await;
    let resolver = Resolver::with_server(client(&server));

    for _ in 0..2_u8 {
        resolver
            .resolve(&template(), &set(), &english(), &ExpandOptions::new())
            .await
            .expect_err("the server is down");
    }

    assert!(resolver.cache().is_empty().expect("the cache answers"));
}
