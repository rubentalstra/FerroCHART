// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Every route, over the real router and a real socket.
//!
//! The CDR is a `wiremock` server, so a request that leaves the process hits a
//! stub rather than a network. Nothing here reaches a real CDR: the cases that
//! do are `live.rs`, and they are ignored without one.
//!
//! No specification governs any route here: our own design. openEHR ITS-REST
//! Release-1.1.0 governs only what the stub answers with, which is why the
//! stubs speak its statuses and headers.
//!
//! Synthetic content invented for these tests. No patient data.

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::validation::{FailureKind, FailureSource};
use ferrochart_form::values::{Datum, Entered, FormValues};
use ferrochart_server::commit::Commit;
use reqwest::StatusCode;
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

/// The EHR every case commits into.
const EHR: &str = "ferro-test-ehr";

/// The version uid a stubbed CDR names for what it stored.
const VERSION: &str = "f0a1b2c3-0000-4000-8000-000000000001::ferro.example::1";

/// A submission body carrying `values` against the shared envelope.
fn submission(values: &FormValues) -> Value {
    serde_json::json!({
        "format_version": FORMAT_VERSION,
        "envelope": support::envelope(),
        "values": values,
    })
}

/// The body of a response, read as JSON whatever its status.
async fn body(response: reqwest::Response) -> Value {
    let text = response.text().await.expect("the body reads");
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("the body is JSON: {text}"))
}

#[tokio::test]
async fn the_template_list_names_every_template_the_server_holds() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-list"), &cdr.uri()).await;

    let response = reqwest::get(format!("{}/api/templates", running.base))
        .await
        .expect("the list answers");
    assert_eq!(response.status(), StatusCode::OK);
    let listed = body(response).await;
    assert_eq!(listed["format_version"], serde_json::json!(FORMAT_VERSION));
    assert_eq!(
        listed["templates"],
        serde_json::json!([support::TEMPLATE_ID])
    );
}

#[tokio::test]
async fn a_definition_is_served_for_a_template_the_server_holds() {
    // The route the renderer could not reach: a form definition compiled from
    // an operational template on disk, end to end over HTTP.
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-definition"), &cdr.uri()).await;

    let response = reqwest::get(format!(
        "{}/api/templates/{}/definition",
        running.base,
        urlencoding(support::TEMPLATE_ID)
    ))
    .await
    .expect("the definition answers");
    assert_eq!(response.status(), StatusCode::OK);

    let served: FormDefinition = response
        .json()
        .await
        .expect("the body is a form definition");
    let (compiled, _, _) = support::case();
    assert_eq!(served, compiled);
    assert!(served.is_current_format());
    assert!(served.fields().count() > 0, "the form carries fields");
}

#[tokio::test]
async fn an_unknown_template_is_not_found() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-unknown"), &cdr.uri()).await;

    let response = reqwest::get(format!(
        "{}/api/templates/no-such-template/definition",
        running.base
    ))
    .await
    .expect("the route answers");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(response).await["error"],
        serde_json::json!("unknown_template")
    );
}

#[tokio::test]
async fn entered_values_the_template_admits_are_judged_valid() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-valid"), &cdr.uri()).await;
    let (_, _, values) = support::case();

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/templates/{}/validation",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&values))
        .send()
        .await
        .expect("the validation answers");
    assert_eq!(response.status(), StatusCode::OK);

    let judged = body(response).await;
    assert_eq!(judged["valid"], serde_json::json!(true), "{judged:#?}");
    assert_eq!(judged["report"]["failures"], serde_json::json!([]));
    // Nothing was written: the route makes no request at all.
    assert!(cdr.received_requests().await.unwrap_or_default().is_empty());
}

#[tokio::test]
async fn a_value_the_template_refuses_comes_back_keyed_on_its_field() {
    // The point of the route: a clinician sees the field to correct, not a
    // paragraph the renderer cannot place.
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-refused"), &cdr.uri()).await;
    let (definition, _, mut values) = support::case();
    let key = out_of_range(&definition, &mut values);

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/templates/{}/validation",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&values))
        .send()
        .await
        .expect("the validation answers");
    assert_eq!(response.status(), StatusCode::OK);

    let judged = body(response).await;
    assert_eq!(judged["valid"], serde_json::json!(false));
    let report: ferrochart_form::validation::ValidationReport =
        serde_json::from_value(judged["report"].clone()).expect("the report reads back");
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.kind == FailureKind::RangeError),
        "{:#?}",
        report.failures
    );
    assert!(
        report.at(&key).next().is_some(),
        "the refusal reaches the field a renderer would show it on"
    );
    assert!(cdr.received_requests().await.unwrap_or_default().is_empty());
}

#[tokio::test]
async fn a_required_field_left_empty_is_reported_on_that_field() {
    // A builder refusal is a judgement too, so it is keyed onto the form
    // rather than returned as prose.
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-empty"), &cdr.uri()).await;

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/templates/{}/validation",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&FormValues::new()))
        .send()
        .await
        .expect("the validation answers");
    assert_eq!(response.status(), StatusCode::OK);

    let judged = body(response).await;
    assert_eq!(judged["valid"], serde_json::json!(false));
    let report: ferrochart_form::validation::ValidationReport =
        serde_json::from_value(judged["report"].clone()).expect("the report reads back");
    assert!(!report.failures.is_empty());
    assert!(
        report
            .failures
            .iter()
            .all(|failure| failure.source == FailureSource::Builder
                || failure.source == FailureSource::Template),
        "{:#?}",
        report.failures
    );
}

#[tokio::test]
async fn a_malformed_body_is_a_bad_request() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-malformed"), &cdr.uri()).await;

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/templates/{}/validation",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{ not json")
        .send()
        .await
        .expect("the route answers");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body(response).await["error"],
        serde_json::json!("malformed_body")
    );
}

#[tokio::test]
async fn a_body_stating_another_format_version_is_refused() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-version"), &cdr.uri()).await;
    let (_, _, values) = support::case();
    let mut stale = submission(&values);
    stale["format_version"] = serde_json::json!(FORMAT_VERSION + 1);

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/templates/{}/validation",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&stale)
        .send()
        .await
        .expect("the route answers");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body(response).await["error"],
        serde_json::json!("unsupported_format")
    );
}

#[tokio::test]
async fn a_commit_reaches_the_cdr_and_names_the_version() {
    let cdr = MockServer::start().await;
    // openEHR ITS-REST Release-1.1.0 `ehr.html`: a create answers 201, and the
    // version uid comes from the `ETag`, which the specification has required
    // to be quoted and weak.
    Mock::given(method("POST"))
        .and(path(format!("/v1/ehr/{EHR}/composition")))
        .respond_with(
            ResponseTemplate::new(StatusCode::CREATED.as_u16())
                .insert_header("ETag", format!("W/\"{VERSION}\"").as_str()),
        )
        .mount(&cdr)
        .await;
    let running = support::serve(support::template_dir("api-commit"), &cdr.uri()).await;
    let (_, _, values) = support::case();

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/ehrs/{EHR}/templates/{}/compositions",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&values))
        .send()
        .await
        .expect("the commit answers");
    assert_eq!(response.status(), StatusCode::CREATED);

    let written = body(response).await;
    assert_eq!(written["version_uid"], serde_json::json!(VERSION));
    assert_eq!(
        written["versioned_object_uid"],
        serde_json::json!("f0a1b2c3-0000-4000-8000-000000000001")
    );
}

#[tokio::test]
async fn a_cdr_refusal_carries_its_status_and_body_outward() {
    // The upstream failure is never flattened: the CDR's own status and body
    // reach the client (`.claude/rules/reliability.md`).
    let cdr = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/ehr/{EHR}/composition")))
        .respond_with(
            ResponseTemplate::new(StatusCode::UNPROCESSABLE_ENTITY.as_u16())
                .set_body_string("the stub refuses this document"),
        )
        .mount(&cdr)
        .await;
    let running = support::serve(support::template_dir("api-rejected"), &cdr.uri()).await;
    let (_, _, values) = support::case();

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/ehrs/{EHR}/templates/{}/compositions",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&values))
        .send()
        .await
        .expect("the commit answers");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let refused = body(response).await;
    assert_eq!(refused["error"], serde_json::json!("cdr_rejected"));
    assert_eq!(
        refused["cdr_status"],
        serde_json::json!(StatusCode::UNPROCESSABLE_ENTITY.as_u16())
    );
    assert_eq!(
        refused["cdr_body"],
        serde_json::json!("the stub refuses this document")
    );
}

#[tokio::test]
async fn a_cdr_that_never_answers_reaches_the_client_as_an_upstream_failure() {
    // Port 9 is the discard service, which no CDR runs on, so the call cannot
    // reach an answer and no status may be claimed for it.
    let running = support::serve(
        support::template_dir("api-unreachable"),
        "http://127.0.0.1:9/openehr",
    )
    .await;
    let (_, _, values) = support::case();

    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/ehrs/{EHR}/templates/{}/compositions",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .json(&submission(&values))
        .send()
        .await
        .expect("the commit answers");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let failed = body(response).await;
    assert_eq!(failed["error"], serde_json::json!("cdr_unreachable"));
    assert_eq!(failed["cdr_status"], Value::Null);
}

#[tokio::test]
async fn a_committed_composition_reads_back_into_the_values_that_built_it() {
    // The round trip the whole surface exists for, over HTTP: the values the
    // commit route would send come back out of the read-back route.
    let (definition, validator, values) = support::case();
    let envelope = support::envelope();
    let composition = Commit {
        definition: &definition,
        validator: &validator,
        envelope: &envelope,
    }
    .validated(&values)
    .expect("the filled form builds a conforming document");

    let cdr = MockServer::start().await;
    let uid = "f0a1b2c3-0000-4000-8000-000000000002";
    Mock::given(method("GET"))
        .and(path(format!("/v1/ehr/{EHR}/composition/{uid}")))
        .respond_with(
            ResponseTemplate::new(StatusCode::OK.as_u16())
                .set_body_json(serde_json::to_value(&composition).expect("it serializes")),
        )
        .mount(&cdr)
        .await;
    let running = support::serve(support::template_dir("api-readback"), &cdr.uri()).await;

    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/ehrs/{EHR}/compositions/{uid}/values?template={}",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .send()
        .await
        .expect("the read-back answers");
    assert_eq!(response.status(), StatusCode::OK);

    let read = body(response).await;
    assert_eq!(read["format_version"], serde_json::json!(FORMAT_VERSION));
    let returned: FormValues =
        serde_json::from_value(read["values"].clone()).expect("the values read back");
    for (slot, entered) in values.iter() {
        assert_eq!(
            returned.get_in(&slot.key, &slot.group_path, slot.occurrence),
            Some(entered),
            "the value at {} group path {:?} occurrence {} did not come back",
            slot.key,
            slot.group_path,
            slot.occurrence
        );
    }
}

#[tokio::test]
async fn the_read_back_route_needs_the_form_to_read_into() {
    let cdr = MockServer::start().await;
    let running = support::serve(support::template_dir("api-no-template"), &cdr.uri()).await;

    let response = reqwest::get(format!(
        "{}/api/ehrs/{EHR}/compositions/f0a1/values",
        running.base
    ))
    .await
    .expect("the route answers");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body(response).await["error"],
        serde_json::json!("missing_query_parameter")
    );
}

#[tokio::test]
async fn a_composition_the_cdr_reports_as_deleted_is_gone() {
    // openEHR ITS-REST Release-1.1.0 `ehr.html` answers 204 where the
    // resource "has been deleted", which is an answer rather than a failure
    // and is distinct from the 404 an absent composition earns.
    let cdr = MockServer::start().await;
    let uid = "f0a1b2c3-0000-4000-8000-000000000003";
    Mock::given(method("GET"))
        .and(path(format!("/v1/ehr/{EHR}/composition/{uid}")))
        .respond_with(ResponseTemplate::new(StatusCode::NO_CONTENT.as_u16()))
        .mount(&cdr)
        .await;
    let running = support::serve(support::template_dir("api-deleted"), &cdr.uri()).await;

    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/ehrs/{EHR}/compositions/{uid}/values?template={}",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .send()
        .await
        .expect("the route answers");
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(
        body(response).await["error"],
        serde_json::json!("composition_deleted")
    );
}

#[tokio::test]
async fn a_composition_the_cdr_does_not_hold_is_not_found() {
    let cdr = MockServer::start().await;
    let uid = "f0a1b2c3-0000-4000-8000-000000000004";
    Mock::given(method("GET"))
        .and(path(format!("/v1/ehr/{EHR}/composition/{uid}")))
        .respond_with(
            ResponseTemplate::new(StatusCode::NOT_FOUND.as_u16())
                .set_body_string("the stub holds no such composition"),
        )
        .mount(&cdr)
        .await;
    let running = support::serve(support::template_dir("api-absent"), &cdr.uri()).await;

    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/ehrs/{EHR}/compositions/{uid}/values?template={}",
            running.base,
            urlencoding(support::TEMPLATE_ID)
        ))
        .send()
        .await
        .expect("the route answers");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let failed = body(response).await;
    assert_eq!(failed["error"], serde_json::json!("cdr_unreachable"));
    assert_eq!(
        failed["cdr_status"],
        serde_json::json!(StatusCode::NOT_FOUND.as_u16())
    );
}

/// Puts a quantity outside every range its template states, and names the
/// field it went into.
///
/// The form's own field carries those ranges, so this is a value the form
/// refuses, put in behind the form's back.
fn out_of_range(
    definition: &FormDefinition,
    values: &mut FormValues,
) -> ferrochart_form::key::NodeKey {
    let field = definition
        .fields()
        .find(|field| {
            matches!(field.kind, ferrochart_form::field::FieldKind::Quantity(ref it)
                if it.units.iter().any(|unit| unit.magnitude.is_some()))
        })
        .expect("the template states a bounded quantity");
    let units = match field.kind {
        ferrochart_form::field::FieldKind::Quantity(ref it) => it
            .units
            .first()
            .expect("the field admits a unit")
            .units
            .clone(),
        ref other => panic!("the field is {other:?}"),
    };
    let group_path = values
        .iter()
        .find(|(slot, _)| slot.key == field.key)
        .map_or_else(Vec::new, |(slot, _)| slot.group_path.clone());
    values.set_in(
        field.key.clone(),
        group_path,
        0,
        Entered::Value(Datum::Quantity {
            magnitude: 1_000_000.0,
            units,
            precision: None,
        }),
    );
    field.key.clone()
}

/// A path segment for a template identifier, which carries spaces.
///
/// openEHR ITS-REST Release-1.1.0 `definition.html` shows `Vital Signs` as a
/// legacy template identifier and spells it `Vital%20Signs` in a `Location`.
fn urlencoding(value: &str) -> String {
    value.replace(' ', "%20")
}
