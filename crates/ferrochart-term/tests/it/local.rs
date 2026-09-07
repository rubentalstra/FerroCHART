// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The value sets that resolve without touching the network.
//!
//! `docs/architecture.md` section 7 measures 97.6% of coded fields as getting
//! their membership from the template, so the load-bearing assertion here is
//! not that the codes are right but that **no request was made**. The mock
//! server mounts nothing and every case ends by reading back what it received.
//!
//! Every code and rubric below is invented for the test. No patient data.

use ferrochart_form::ids::{LanguageTag, LocalCode, TerminologyName, local_terminology};
use ferrochart_form::value::{EnumeratedSet, ValueSet};
use ferrochart_term::expand::ExpandOptions;
use ferrochart_term::local;
use ferrochart_term::resolution::Origin;
use ferrochart_term::resolve::Resolver;
use std::collections::BTreeMap;
use wiremock::MockServer;

use crate::support::{client, english, option, template};

fn model_code(code: &str) -> ferrochart_compile::model::ids::LocalCode {
    ferrochart_compile::model::ids::LocalCode::new(code)
}

fn model_language(tag: &str) -> ferrochart_compile::model::ids::LanguageTag {
    ferrochart_compile::model::ids::LanguageTag::new(tag)
}

/// An archetype terminology of the shape openEHR AM Release-2.3.0
/// `AOM1.4.html` section 7.3.2 defines: an `ac`-code naming a set, and an
/// `ARCHETYPE_TERM` per member per language.
fn archetype() -> ferrochart_compile::model::terminology::Terminology {
    use ferrochart_compile::model::terminology::{TermDefinition, Terminology};
    let mut terminology = Terminology::new();
    terminology.insert_value_set(
        model_code("ac0.1"),
        vec![model_code("at0011"), model_code("at0012")],
    );
    terminology.insert_definition(
        model_language("en"),
        model_code("at0011"),
        TermDefinition::new("Fingertip", Some("On a finger".to_owned()), BTreeMap::new()),
    );
    terminology.insert_definition(
        model_language("en"),
        model_code("at0012"),
        TermDefinition::new("Earlobe", None, BTreeMap::new()),
    );
    terminology.insert_definition(
        model_language("nl"),
        model_code("at0011"),
        TermDefinition::new("Vingertop", None, BTreeMap::new()),
    );
    terminology.insert_definition(
        model_language("nl"),
        model_code("at0012"),
        TermDefinition::new("Oorlel", None, BTreeMap::new()),
    );
    terminology
}

#[tokio::test]
async fn an_archetype_local_value_set_resolves_with_no_request_at_all() {
    // The mock mounts nothing, so any request would be recorded and the last
    // assertion would fail. That is the point of the case: the local path
    // never opens a socket.
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));

    let resolution = resolver
        .resolve_in_template(
            &template(),
            &archetype(),
            &ValueSet::Expansion(ferrochart_form::value::ExpansionSource::ConstraintCode {
                code: LocalCode::new("ac0.1"),
                bindings: BTreeMap::new(),
                pinned_terminology: None,
            }),
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("the archetype enumerates the set");

    assert_eq!(resolution.origin, Origin::Template);
    let shown: Vec<Option<&str>> = resolution
        .codes
        .iter()
        .map(|code| code.display.as_deref())
        .collect();
    assert_eq!(shown, [Some("Fingertip"), Some("Earlobe")]);
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "a local value set issued a request"
    );
}

#[tokio::test]
async fn the_rubrics_come_back_in_the_language_that_was_asked_for() {
    let server = MockServer::start().await;
    let resolution = local::archetype_value_set(
        &archetype(),
        &LocalCode::new("ac0.1"),
        &LanguageTag::new("nl"),
    )
    .expect("the archetype enumerates the set");

    let shown: Vec<Option<&str>> = resolution
        .codes
        .iter()
        .map(|code| code.display.as_deref())
        .collect();
    assert_eq!(shown, [Some("Vingertop"), Some("Oorlel")]);
    assert_eq!(resolution.language, LanguageTag::new("nl"));
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "reading the template terminology issued a request"
    );
}

#[tokio::test]
async fn an_enumerated_local_set_in_the_form_definition_resolves_with_no_request() {
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Enumerated(EnumeratedSet {
        terminology: local_terminology(),
        options: vec![
            option("local", "at0004", Some("Sitting")),
            option("local", "at0005", Some("Standing")),
        ],
        needs_display_lookup: false,
    });

    let resolution = resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect("a local set needs nothing else");

    assert_eq!(resolution.origin, Origin::Template);
    assert_eq!(resolution.codes.len(), 2);
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "an enumerated local set issued a request"
    );
}

#[tokio::test]
async fn openehr_terminology_codes_get_their_display_text_without_a_call() {
    // openEHR TERM Release-3.0.0 publishes the composition-category group, and
    // `openehr-term` embeds it, so `433` is named without a server. The
    // template states no rubric for it, which is why this case matters.
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));
    let set = ValueSet::Enumerated(EnumeratedSet {
        terminology: TerminologyName::new("openehr"),
        options: vec![option("openehr", "433", None)],
        needs_display_lookup: true,
    });

    let resolution = resolver
        .resolve(&template(), &set, &english(), &ExpandOptions::new())
        .await
        .expect("openEHR's own terminology is embedded");

    assert_eq!(resolution.origin, Origin::OpenehrTerminology);
    assert!(
        resolution
            .codes
            .first()
            .is_some_and(|code| code.display.is_some()),
        "the embedded terminology named the code"
    );
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "an openEHR terminology code issued a request"
    );
}

#[tokio::test]
async fn an_openehr_group_enumerates_from_the_embedded_terminology() {
    let server = MockServer::start().await;
    let resolution = local::openehr_group("null_flavours", &english())
        .expect("the RM-mandated group is embedded");

    assert_eq!(resolution.origin, Origin::OpenehrTerminology);
    assert!(!resolution.codes.is_empty());
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "an openEHR group issued a request"
    );
}

#[tokio::test]
async fn an_unconstrained_field_says_so_rather_than_asking_a_server() {
    let server = MockServer::start().await;
    let resolver = Resolver::with_server(client(&server));

    let resolution = resolver
        .resolve(
            &template(),
            &ValueSet::Unconstrained,
            &english(),
            &ExpandOptions::new(),
        )
        .await
        .expect("an unconstrained field has no set to resolve");

    assert_eq!(resolution.origin, Origin::Unconstrained);
    assert!(resolution.codes.is_empty());
    assert!(
        server
            .received_requests()
            .await
            .is_some_and(|r| r.is_empty()),
        "an unconstrained field issued a request"
    );
}
