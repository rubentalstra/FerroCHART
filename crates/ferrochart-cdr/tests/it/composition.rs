// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Create, read and update against a mock CDR.
//!
//! Every case asserts what went onto the wire as well as what came back,
//! because a client that sends the wrong header and is answered anyway is
//! still wrong. All citations are openEHR ITS-REST Release-1.1.0 `ehr.html`
//! unless another page is named.

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::error::CdrError;
use ferrochart_cdr::header::Prefer;
use ferrochart_cdr::ids::{CompositionId, EhrId, VersionUid, VersionedObjectUid};
use http::StatusCode;
use openehr_rm::v1_2::composition::composition::Composition;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Synthetic content invented for this test. No patient data.
const COMPOSITION: &str = include_str!("../fixtures/ferro_synthetic_composition.json");

const EHR: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const OBJECT: &str = "8849182c-82ad-4088-a07f-48ead4180515";
const VERSION: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2";
const NEXT_VERSION: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::3";

fn composition() -> Composition {
    serde_json::from_str(COMPOSITION).expect("the fixture is a canonical COMPOSITION")
}

fn client(server: &MockServer) -> CdrClient {
    let base = Url::parse(&server.uri()).expect("the mock server has a URL");
    CdrClient::new(&base).expect("the client builds")
}

fn ehr() -> EhrId {
    EhrId::new(EHR)
}

fn object() -> VersionedObjectUid {
    VersionedObjectUid::new(OBJECT)
}

#[tokio::test]
async fn a_create_posts_under_v1_and_asks_for_the_identifier() {
    // POST /v1/ehr/{ehr_id}/composition, whose only success status is 201.
    // `Prefer` is sent explicitly on every write, because `overview.html`
    // warns that a server may be configured to change the default.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/ehr/{EHR}/composition")))
        .and(header("Prefer", "return=identifier"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(201).insert_header("ETag", format!("W/\"{VERSION}\"")))
        .expect(1)
        .mount(&server)
        .await;

    let written = client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect("the create succeeds");

    assert_eq!(written.version, VersionUid::new(VERSION));
    assert!(
        written.composition.is_none(),
        "return=identifier asked for no document, so none is reported"
    );
}

#[tokio::test]
async fn a_create_reads_the_version_out_of_the_body_when_there_is_no_etag() {
    // The `ETag` is only a SHOULD, and `overview.html` defines the
    // `return=identifier` body as "a single JSON object with a single `uid`
    // attribute". A conformant CDR can send one and not the other.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(201).set_body_string(format!("{{\"uid\":\"{VERSION}\"}}")),
        )
        .mount(&server)
        .await;

    let written = client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect("the create succeeds");
    assert_eq!(written.version, VersionUid::new(VERSION));
}

#[tokio::test]
async fn a_create_asking_for_a_representation_reports_the_stored_document() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("Prefer", "return=representation"))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("ETag", format!("\"{VERSION}\""))
                .set_body_string(COMPOSITION),
        )
        .expect(1)
        .mount(&server)
        .await;

    let written = client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Representation)
        .await
        .expect("the create succeeds");

    // The ETag came back unweakened, which Release-1.0.3 did and 1.1.0 still
    // permits, and the version is read out of it all the same.
    assert_eq!(written.version, VersionUid::new(VERSION));
    assert_eq!(written.composition, Some(composition()));
}

#[tokio::test]
async fn a_create_the_cdr_finds_invalid_is_a_ferrochart_bug_and_keeps_the_body() {
    // 422: "the content type and syntax is correct ... but there are semantic
    // validation errors". A CDR rejecting a COMPOSITION FerroCHART built and
    // validated is always FerroCHART's bug, so the body is kept verbatim for
    // the report.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(422).set_body_string("value out of range at /data"))
        .mount(&server)
        .await;

    match client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect_err("the create is refused")
    {
        CdrError::Unprocessable { body } => assert_eq!(body, "value out of range at /data"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_malformed_request_is_told_apart_from_an_invalid_document() {
    // 400 and 422 are different answers: one says the request could not be
    // parsed, the other says the document is semantically wrong. An update
    // also answers 400 when `If-Match` is absent, which is why the body is
    // carried.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("missing required header"))
        .mount(&server)
        .await;

    match client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect_err("the request is malformed")
    {
        CdrError::BadRequest { body } => assert_eq!(body, "missing required header"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_create_into_an_absent_ehr_names_the_ehr() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    match client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect_err("there is no such EHR")
    {
        CdrError::NotFound { what, id } => {
            assert_eq!(what, "ehr");
            assert_eq!(id, EHR);
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_create_that_names_no_version_is_a_half_success_and_says_so() {
    // The document is stored and the client cannot address it. Reporting
    // success would lose the version the next update needs.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    match client(&server)
        .create_composition(&ehr(), &composition(), Prefer::Identifier)
        .await
        .expect_err("there is no version to report")
    {
        CdrError::UnnamedVersion { status } => assert_eq!(status, StatusCode::CREATED),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_read_accepts_both_id_forms() {
    // The read path's `uid_based_id` takes either a versioned object uid,
    // which names the latest version, or a full version uid, which names one
    // version.
    let server = MockServer::start().await;
    for id in [OBJECT, VERSION] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/ehr/{EHR}/composition/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(COMPOSITION))
            .expect(1)
            .mount(&server)
            .await;
    }

    let client = client(&server);
    let latest = CompositionId::LatestVersionOf(object());
    let pinned = CompositionId::Version(VersionUid::new(VERSION));
    assert_eq!(
        client
            .read_composition(&ehr(), &latest)
            .await
            .expect("the latest version reads"),
        Some(composition())
    );
    assert_eq!(
        client
            .read_composition(&ehr(), &pinned)
            .await
            .expect("the pinned version reads"),
        Some(composition())
    );
}

#[tokio::test]
async fn a_composition_deleted_at_the_requested_time_reads_as_absent_not_as_a_failure() {
    // 204: "the resource identified by the request parameters (at specified
    // `version_at_time`) time has been deleted". That is an answer, and it is
    // a different one from the 404 a composition that never existed earns.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    assert_eq!(
        client(&server)
            .read_composition(&ehr(), &id)
            .await
            .expect("a deleted composition is an answer"),
        None
    );
}

#[tokio::test]
async fn a_read_of_an_absent_composition_names_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    match client(&server)
        .read_composition(&ehr(), &id)
        .await
        .expect_err("there is no such composition")
    {
        CdrError::NotFound { what, id } => {
            assert_eq!(what, "composition");
            assert_eq!(id, VERSION);
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_read_of_something_that_is_not_a_composition_is_not_absorbed() {
    // An upstream that answers 200 with the wrong body is a failure, not an
    // empty result (`.claude/rules/reliability.md`).
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"_type\":\"EHR_STATUS\"}"))
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    match client(&server)
        .read_composition(&ehr(), &id)
        .await
        .expect_err("that is not a composition")
    {
        CdrError::Body { expected, .. } => assert_eq!(expected, "a COMPOSITION"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn an_update_addresses_the_object_and_names_the_version_in_if_match() {
    // The update path's identifier "can take only a form of an HIER_OBJECT_ID
    // identifier taken from VERSIONED_OBJECT.uid.value", so the path segment
    // is the object uid while `If-Match` carries the full version uid. The
    // published example is
    // `If-Match: "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"`.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(format!("/v1/ehr/{EHR}/composition/{OBJECT}")))
        .and(header("If-Match", format!("\"{VERSION}\"").as_str()))
        .and(header("Prefer", "return=identifier"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("ETag", format!("W/\"{NEXT_VERSION}\"")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let written = client(&server)
        .update_composition(
            &ehr(),
            &object(),
            &VersionUid::new(VERSION),
            &composition(),
            Prefer::Identifier,
        )
        .await
        .expect("the update succeeds");

    assert_eq!(written.version, VersionUid::new(NEXT_VERSION));
}

#[tokio::test]
async fn an_update_asking_for_the_minimum_succeeds_on_204() {
    // "`204 No Content` is returned when the update operation was successful
    // and the `Prefer` header is missing or is set to `return=minimal`." A
    // client that treated 204 as failure would report a successful write as a
    // failed one.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(header("Prefer", "return=minimal"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("ETag", format!("W/\"{NEXT_VERSION}\"")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let written = client(&server)
        .update_composition(
            &ehr(),
            &object(),
            &VersionUid::new(VERSION),
            &composition(),
            Prefer::Minimal,
        )
        .await
        .expect("204 is a success");
    assert_eq!(written.version, VersionUid::new(NEXT_VERSION));
}

#[tokio::test]
async fn a_412_is_a_concurrent_edit_carrying_the_version_the_cdr_now_holds() {
    // The composition moved on since the version the update named. The form
    // shows this to the clinician; retrying would overwrite whatever the other
    // writer stored, which is the failure `If-Match` exists to prevent. The
    // 412 "returns also latest `version_uid` in the `ETag` header".
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(
            ResponseTemplate::new(412).insert_header("ETag", format!("W/\"{NEXT_VERSION}\"")),
        )
        // Exactly once: the point of the case is that nothing retries.
        .expect(1)
        .mount(&server)
        .await;

    match client(&server)
        .update_composition(
            &ehr(),
            &object(),
            &VersionUid::new(VERSION),
            &composition(),
            Prefer::Identifier,
        )
        .await
        .expect_err("the composition has changed")
    {
        CdrError::ConcurrentEdit { expected, latest } => {
            assert_eq!(expected, VersionUid::new(VERSION));
            assert_eq!(latest, Some(VersionUid::new(NEXT_VERSION)));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_412_without_an_etag_is_still_a_concurrent_edit() {
    // The `ETag` on a 412 is only a SHOULD, so a client that required it would
    // turn a concurrent edit into an unrelated failure.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(412))
        .mount(&server)
        .await;

    match client(&server)
        .update_composition(
            &ehr(),
            &object(),
            &VersionUid::new(VERSION),
            &composition(),
            Prefer::Identifier,
        )
        .await
        .expect_err("the composition has changed")
    {
        CdrError::ConcurrentEdit { latest, .. } => assert_eq!(latest, None),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn an_authentication_challenge_is_carried_rather_than_interpreted() {
    // `overview.html` mandates no authentication scheme, so the client keeps
    // what the CDR asked for instead of assuming one.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(401).insert_header("WWW-Authenticate", "Basic realm=\"cdr\""),
        )
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    match client(&server)
        .read_composition(&ehr(), &id)
        .await
        .expect_err("the CDR wants credentials")
    {
        CdrError::Unauthorized { status, challenge } => {
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(challenge.as_deref(), Some("Basic realm=\"cdr\""));
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_configured_authorization_header_reaches_every_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("Authorization", "Basic Zm9vOmJhcg=="))
        .respond_with(ResponseTemplate::new(200).set_body_string(COMPOSITION))
        .expect(1)
        .mount(&server)
        .await;

    let base = Url::parse(&server.uri()).expect("the mock server has a URL");
    let client = CdrClient::new(&base)
        .expect("the client builds")
        .with_authorization("Basic Zm9vOmJhcg==");
    let id = CompositionId::Version(VersionUid::new(VERSION));
    client
        .read_composition(&ehr(), &id)
        .await
        .expect("the authorized read succeeds");
}

#[tokio::test]
async fn a_cdr_that_cannot_serve_the_requested_format_says_which_header_it_refused() {
    // 406 is about `Accept` and 415 about `Content-Type`. Both are capability
    // answers rather than failures: a CDR that serves only XML is conformant.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(406))
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    match client(&server)
        .read_composition(&ehr(), &id)
        .await
        .expect_err("the CDR cannot serve canonical JSON")
    {
        CdrError::UnsupportedMediaType {
            status,
            what,
            media_type,
        } => {
            assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
            assert_eq!(what, "Accept");
            assert_eq!(media_type, "application/json");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_status_the_specification_does_not_publish_still_carries_its_status_and_body() {
    // `overview.html`: "Additional status codes MAY be used as long as they do
    // not conflict with the predefined codes." An exhaustive match with no
    // catch-all would be a specification violation dressed as a compile-time
    // guarantee.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503).set_body_string("the CDR is draining"))
        .mount(&server)
        .await;

    let id = CompositionId::Version(VersionUid::new(VERSION));
    match client(&server)
        .read_composition(&ehr(), &id)
        .await
        .expect_err("the CDR is unavailable")
    {
        CdrError::Upstream { status, body } => {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body, "the CDR is draining");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
async fn a_deployment_path_prefix_survives_the_join() {
    // `baseUrl` "may contain server name, port and base path prefix", so a
    // deployment's own prefix has to be extended rather than replaced.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/openehr/v1/ehr/{EHR}/composition/{VERSION}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(COMPOSITION))
        .expect(1)
        .mount(&server)
        .await;

    let base = Url::parse(&format!("{}/openehr", server.uri())).expect("a base URL");
    let client = CdrClient::new(&base).expect("the client builds");
    let id = CompositionId::Version(VersionUid::new(VERSION));
    client
        .read_composition(&ehr(), &id)
        .await
        .expect("the prefixed path reads");
}

#[tokio::test]
async fn a_trailing_slash_on_the_base_does_not_double_the_separator() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/ehr/{EHR}/composition/{VERSION}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(COMPOSITION))
        .expect(1)
        .mount(&server)
        .await;

    let base = Url::parse(&format!("{}/", server.uri())).expect("a base URL");
    let client = CdrClient::new(&base).expect("the client builds");
    let id = CompositionId::Version(VersionUid::new(VERSION));
    client
        .read_composition(&ehr(), &id)
        .await
        .expect("the read succeeds");
}
