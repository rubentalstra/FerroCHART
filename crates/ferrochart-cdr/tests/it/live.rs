// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The client against a running CDR.
//!
//! A mock answers whatever it is told to, so it proves the client sends the
//! right request and reads the right response, and nothing about whether a
//! real CDR agrees. These cases run against one.
//!
//! They are `#[ignore]`d, because they need a server: `scripts/test-cdr.sh`
//! starts the compose demo profile, points `FERROCHART_TEST_CDR_URL` at it,
//! and runs them. An ignored case reports as ignored rather than as a pass, so
//! a run without a CDR cannot be mistaken for one with it.

use std::env;

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::error::CdrError;
use ferrochart_cdr::header::Prefer;
use ferrochart_cdr::ids::{CompositionId, EhrId, VersionUid, VersionedObjectUid};
use ferrochart_cdr::template::Generation;
use ferrochart_form::ids::TemplateId;
use openehr_rm::v1_2::composition::composition::Composition;
use url::Url;

/// Synthetic content invented for this test. No patient data.
const COMPOSITION: &str = include_str!("../fixtures/ferro_synthetic_composition.json");

/// Synthetic content invented for this test. No patient data.
const TEMPLATE: &str = include_str!("../fixtures/ferro_live_template.opt");

/// The base URL of the CDR under test, as `scripts/test-cdr.sh` sets it.
const CDR_URL: &str = "FERROCHART_TEST_CDR_URL";

/// The `Authorization` header value, where the CDR under test wants one.
const CDR_AUTHORIZATION: &str = "FERROCHART_TEST_CDR_AUTHORIZATION";

/// An identifier no CDR will hold.
const ABSENT: &str = "00000000-0000-4000-8000-000000000000";

fn client() -> CdrClient {
    let base =
        env::var(CDR_URL).unwrap_or_else(|_| panic!("{CDR_URL} is unset; run scripts/test-cdr.sh"));
    let base = Url::parse(&base).expect("the CDR base is a URL");
    let client = CdrClient::new(&base).expect("the client builds");
    match env::var(CDR_AUTHORIZATION) {
        Ok(value) => client.with_authorization(value),
        Err(_) => client,
    }
}

fn composition() -> Composition {
    serde_json::from_str(COMPOSITION).expect("the fixture is a canonical COMPOSITION")
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_creates_an_ehr_and_reports_it_present() {
    let client = client();
    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    assert!(!ehr.as_str().is_empty(), "the CDR named the EHR it created");
    assert!(
        client.ehr_exists(&ehr).await.expect("the CDR answers"),
        "the EHR the CDR just created is present"
    );
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_reports_an_absent_ehr_as_absent() {
    // The pair with the case above is the point: a fabricated identifier
    // answers 404 and reads as absent rather than as an error, which is what
    // tells the two outcomes apart on a real server.
    assert!(
        !client()
            .ehr_exists(&EhrId::new(ABSENT))
            .await
            .expect("the CDR answers"),
        "an EHR the CDR does not hold reads as absent"
    );
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_refuses_a_composition_that_matches_no_template() {
    // The synthetic COMPOSITION names a template the CDR has never seen, so a
    // conformant CDR refuses it. What is asserted is the mapping: the refusal
    // arrives as a typed error carrying the CDR's own status and body, and is
    // never absorbed into an empty result. openEHR ITS-REST Release-1.1.0
    // `ehr.html` gives 422 for "the underlying template is not known", and 400
    // for a request the CDR could not parse.
    let client = client();
    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");

    let failure = client
        .create_composition(&ehr, &composition(), Prefer::Identifier)
        .await
        .expect_err("the CDR knows no such template");
    match failure {
        CdrError::Unprocessable { ref body } | CdrError::BadRequest { ref body } => {
            assert!(!body.is_empty(), "the CDR said why");
        }
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_reports_an_absent_composition_as_absent() {
    let client = client();
    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    let id = CompositionId::LatestVersionOf(VersionedObjectUid::new(ABSENT));

    match client
        .read_composition(&ehr, &id)
        .await
        .expect_err("the EHR is empty")
    {
        CdrError::NotFound { what, .. } => assert_eq!(what, "composition"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_refuses_an_update_of_a_version_it_does_not_hold() {
    // A fabricated preceding version cannot match, so the CDR answers 412 or
    // 404. Either is correct; what must not happen is a silent success or a
    // retry, and `update_composition` never retries.
    let client = client();
    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    let preceding = VersionUid::new(format!("{ABSENT}::ferro.example::1"));

    let failure = client
        .update_composition(
            &ehr,
            &VersionedObjectUid::new(ABSENT),
            &preceding,
            &composition(),
            Prefer::Identifier,
        )
        .await
        .expect_err("there is no such version to replace");
    assert!(
        matches!(
            failure,
            CdrError::ConcurrentEdit { .. }
                | CdrError::NotFound { .. }
                | CdrError::BadRequest { .. }
                | CdrError::Unprocessable { .. }
        ),
        "the failure is {failure:?}"
    );
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_reports_an_absent_template_as_absent() {
    let id = TemplateId::new("openEHR-EHR-COMPOSITION.ferro_absent.v1.0.0");
    match client()
        .operational_template(&id)
        .await
        .expect_err("the CDR holds no such template")
    {
        CdrError::NotFound { what, .. } => assert_eq!(what, "template"),
        other => panic!("the failure is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_takes_a_template_and_gives_it_back() {
    // The round trip the compiler depends on: FerroCHART can take a template
    // from a CDR as well as from a file on disk.
    let client = client();
    let uploaded = client.upload_template(Generation::Adl14, TEMPLATE).await;
    match uploaded {
        Ok(_) => {}
        // These cases share one CDR, so a template already uploaded by an
        // earlier run is not a new fact about the client.
        Err(CdrError::Conflict { .. }) => {}
        Err(other) => panic!("the CDR would not take the template: {other:?}"),
    }

    let id = TemplateId::new("openEHR-EHR-OBSERVATION.ferro_live.v1.0.0");
    let canonical = client
        .operational_template(&id)
        .await
        .expect("the CDR gives the template back");
    assert!(
        canonical.contains("ferro_live"),
        "the CDR returned a template that is not the one uploaded"
    );
}
