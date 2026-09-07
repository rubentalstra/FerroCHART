// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Template upload and retrieval, for both ADL generations.
//!
//! All citations are openEHR ITS-REST Release-1.1.0 `definition.html`.

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::error::CdrError;
use ferrochart_cdr::template::Generation;
use ferrochart_form::ids::TemplateId;
use http::StatusCode;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEMPLATE: &str = "openEHR-EHR-COMPOSITION.ferro_synthetic.v1.0.0";

/// Synthetic content invented for this test. No patient data.
const OPT: &str = "<template><template_id><value>ferro</value></template_id></template>";

/// The `return=identifier` body an upload answers with.
const IDENTIFIER: &str = "{\"template_id\":\"openEHR-EHR-COMPOSITION.ferro_synthetic.v1.0.0\"}";

fn client(server: &MockServer) -> CdrClient {
    let base = Url::parse(&server.uri()).expect("the mock server has a URL");
    CdrClient::new(&base).expect("the client builds")
}

#[tokio::test]
async fn each_generation_uploads_to_its_own_path_and_media_type() {
    // A CDR may implement one generation, the other, or both, so the request
    // media type differs: the ADL 1.4 path takes the operational template XML
    // and the ADL 2 path takes source text.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/definition/template/adl1.4"))
        .and(header("Content-Type", "application/xml"))
        .respond_with(ResponseTemplate::new(201).set_body_string(IDENTIFIER))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/definition/template/adl2"))
        .and(header("Content-Type", "text/plain"))
        .respond_with(ResponseTemplate::new(201).set_body_string(IDENTIFIER))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert_eq!(
        client
            .upload_template(Generation::Adl14, OPT)
            .await
            .expect("the ADL 1.4 upload succeeds")
            .template_id
            .as_str(),
        TEMPLATE
    );
    assert_eq!(
        client
            .upload_template(Generation::Adl2, "archetype (adl_version=2.0.5)")
            .await
            .expect("the ADL 2 upload succeeds")
            .template_id
            .as_str(),
        TEMPLATE
    );
}

#[tokio::test]
async fn a_cdr_that_does_not_implement_a_generation_is_reported_as_it_answered() {
    // A 404 on the upload path means this CDR does not implement this
    // generation. That is a fact about the server, not a template defect, so
    // it is never flattened into a refusal.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/definition/template/adl2"))
        .respond_with(ResponseTemplate::new(404).set_body_string("no such endpoint"))
        .mount(&server)
        .await;

    match client(&server)
        .upload_template(Generation::Adl2, "archetype (adl_version=2.0.5)")
        .await
        .expect_err("this CDR has no ADL 2 endpoint")
    {
        CdrError::NotFound { what, id } => {
            assert_eq!(what, "template endpoint");
            assert_eq!(id, "adl2");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_template_the_cdr_refuses_carries_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("the template does not parse"))
        .mount(&server)
        .await;

    match client(&server)
        .upload_template(Generation::Adl14, OPT)
        .await
        .expect_err("the template is refused")
    {
        CdrError::BadRequest { body } => assert_eq!(body, "the template does not parse"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_template_the_cdr_already_holds_is_a_conflict_not_a_refusal() {
    // "`409 Conflict` is returned when a template with same `template_id`
    // already exists." Uploading a template twice is an ordinary outcome, and
    // the caller decides what it means, so it does not read as a defect in the
    // template.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(409).set_body_string("already uploaded"))
        .mount(&server)
        .await;

    match client(&server)
        .upload_template(Generation::Adl14, OPT)
        .await
        .expect_err("the CDR already holds it")
    {
        CdrError::Conflict { body, .. } => assert_eq!(body, "already uploaded"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_template_id_that_is_not_url_safe_is_encoded() {
    // `definition.html` gives `Vital Signs` as a legacy template identifier
    // and shows it in a `Location` as `Vital%20Signs`, so a template id is not
    // URL-safe by construction.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/definition/template/adl1.4/Vital%20Signs"))
        .respond_with(ResponseTemplate::new(200).set_body_string(OPT))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .operational_template(&TemplateId::new("Vital Signs"))
        .await
        .expect("the legacy identifier resolves");
}

#[tokio::test]
async fn one_path_serves_the_canonical_xml_and_the_web_template_by_accept() {
    // GET /v1/definition/template/adl1.4/{template_id} returns the canonical
    // OPT XML under `Accept: application/xml` and the web template under
    // `Accept: application/openehr.wt+json`. The web template is a
    // compatibility target rather than a specification.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/definition/template/adl1.4/{TEMPLATE}")))
        .and(header("Accept", "application/xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(OPT))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/definition/template/adl1.4/{TEMPLATE}")))
        .and(header("Accept", "application/openehr.wt+json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"templateId\":\"ferro\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    let id = TemplateId::new(TEMPLATE);
    assert_eq!(
        client
            .operational_template(&id)
            .await
            .expect("the canonical XML reads"),
        OPT
    );
    assert_eq!(
        client
            .web_template(&id)
            .await
            .expect("the web template reads"),
        "{\"templateId\":\"ferro\"}"
    );
}

#[tokio::test]
async fn a_cdr_that_offers_no_web_template_is_reported_rather_than_guessed_at() {
    // A 406 here means this CDR does not offer web templates. It is a
    // compatibility target, so its absence is not a defect, and the client
    // says which status it got rather than returning nothing.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("Accept", "application/openehr.wt+json"))
        .respond_with(ResponseTemplate::new(406))
        .mount(&server)
        .await;

    match client(&server)
        .web_template(&TemplateId::new(TEMPLATE))
        .await
        .expect_err("this CDR offers no web template")
    {
        CdrError::UnsupportedMediaType {
            status,
            what,
            media_type,
        } => {
            assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
            assert_eq!(what, "Accept");
            assert_eq!(media_type, "application/openehr.wt+json");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn adl2_source_reads_back_from_its_own_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/definition/template/adl2/{TEMPLATE}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("archetype (adl_version=2.0.5)"))
        .expect(1)
        .mount(&server)
        .await;

    assert_eq!(
        client(&server)
            .adl2_template(&TemplateId::new(TEMPLATE))
            .await
            .expect("the ADL 2 source reads"),
        "archetype (adl_version=2.0.5)"
    );
}

#[tokio::test]
async fn an_absent_template_names_the_template() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    match client(&server)
        .operational_template(&TemplateId::new(TEMPLATE))
        .await
        .expect_err("the CDR holds no such template")
    {
        CdrError::NotFound { what, id } => {
            assert_eq!(what, "template");
            assert_eq!(id, TEMPLATE);
        }
        other => panic!("the failure is {other:?}"),
    }
}
