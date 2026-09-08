// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The gate against a running CDR.
//!
//! This is the strongest case in the repository, because it is the only one
//! that can falsify the product's central promise: a CDR rejecting a
//! COMPOSITION FerroCHART built and validated is always a FerroCHART bug
//! (`CLAUDE.md`). Every other test judges FerroCHART against FerroCHART.
//!
//! The cases are `#[ignore]`d, because they need a server:
//! `scripts/test-cdr.sh` starts the compose demo profile, points
//! `FERROCHART_TEST_CDR_URL` at it, and runs them. An ignored case reports as
//! ignored rather than as a pass, so a run without a CDR cannot be mistaken
//! for one with it.

use std::env;

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::error::CdrError;
use ferrochart_cdr::ids::{CompositionId, VersionedObjectUid};
use ferrochart_cdr::template::Generation;
use ferrochart_form::ids::TemplateId;
use ferrochart_server::commit::{Commit, CommitError};
use url::Url;

use crate::support;

/// The base URL of the CDR under test, as `scripts/test-cdr.sh` sets it.
const CDR_URL: &str = "FERROCHART_TEST_CDR_URL";

/// The `Authorization` header value, where the CDR under test wants one.
const CDR_AUTHORIZATION: &str = "FERROCHART_TEST_CDR_AUTHORIZATION";

/// One committed template per class the pack roots at below COMPOSITION.
///
/// 113 of the 123 committed templates root at an ENTRY or a SECTION, so this
/// is the shape most of the pack commits in. Each name here compiles, fills,
/// builds and passes FerroCHART's own gate, so the only judgement left is the
/// CDR's.
const ROOTED_BELOW_COMPOSITION: [&str; 5] = [
    "aedes-indices-jm.opt",
    "alcohol-consumption-summary-item-r1.opt",
    "case-demographic-information-jm.opt",
    "medication-order-item-r1.opt",
    "travel-event-jm.opt",
];

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

/// Puts the committed template on the CDR, tolerating one an earlier run left.
async fn upload(client: &CdrClient) {
    match client
        .upload_template(Generation::Adl14, &support::template_xml())
        .await
    {
        Ok(_) | Err(CdrError::Conflict { .. }) => {}
        Err(other) => panic!("the CDR would not take {}: {other:?}", support::TEMPLATE),
    }
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_accepts_a_composition_ferrochart_built_and_validated() {
    // The product's promise, against a real server. A refusal here is a
    // FerroCHART defect and is reported as one; it is never worked around by
    // relaxing what this case asserts (`.claude/rules/testing.md`).
    let client = client();
    upload(&client).await;

    let (definition, validator, values) = support::case();
    let envelope = support::envelope();
    let gate = Commit {
        definition: &definition,
        validator: &validator,
        envelope: &envelope,
    };

    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    let written = match gate.create(&client, &ehr, &values).await {
        Ok(written) => written,
        Err(CommitError::Rejected { source, diagnosis }) => panic!(
            "the CDR refused a COMPOSITION FerroCHART validated, which is a \
             FerroCHART defect: {source}\nthe CDR's own words, read onto the \
             form where they could be: {:#?}",
            diagnosis.failures
        ),
        Err(other) => panic!("the commit failed: {other:?}"),
    };

    assert!(
        !written.version.as_str().is_empty(),
        "the CDR named the version it stored"
    );

    // The commit is only proven by the read-back: a CDR that answered 201 and
    // stored nothing would pass every assertion above.
    let stored = client
        .read_composition(
            &ehr,
            &CompositionId::LatestVersionOf(VersionedObjectUid::new(
                written.version.object().as_str(),
            )),
        )
        .await
        .expect("the CDR gives the composition back")
        .expect("the composition is not deleted");
    assert_eq!(
        stored.archetype_details.map(|details| details.template_id),
        Some(Some(
            openehr_base::v1_3::base_types::identification::template_id::TemplateId {
                value: TemplateId::new(support::TEMPLATE_ID).as_str().to_owned(),
            }
        )),
        "the stored document names the template it was built from"
    );
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_never_sees_a_composition_the_gate_refused() {
    // The gate's ordering, against a real server this time: the EHR is real,
    // the client works, and the document still does not leave the process.
    let client = client();
    upload(&client).await;

    let (definition, validator, _) = support::case();
    let envelope = support::envelope();

    // No entries at all. The template makes nodes mandatory, so the gate has
    // something to refuse, and a CDR would refuse the same document.
    let values = ferrochart_form::values::FormValues::new();

    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    let gate = Commit {
        definition: &definition,
        validator: &validator,
        envelope: &envelope,
    };
    match gate.create(&client, &ehr, &values).await {
        Err(CommitError::Refused { .. } | CommitError::Build { .. }) => {}
        Ok(_) => panic!("an empty form committed a document"),
        Err(other) => panic!("the outcome is {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a running CDR: scripts/test-cdr.sh"]
async fn a_real_cdr_accepts_a_composition_around_a_template_rooted_below_it() {
    let client = client();
    let ehr = client.create_ehr().await.expect("the CDR creates an EHR");
    let envelope = support::envelope();
    let mut refused: Vec<String> = Vec::new();

    for name in ROOTED_BELOW_COMPOSITION {
        let xml = support::xml_at(&support::corpus().join(name));
        let (definition, validator, values) = support::case_at(&xml);
        let rooted_at = definition.root.rm_type.as_str().to_owned();
        assert_ne!(
            rooted_at, "COMPOSITION",
            "{name} is in this sample because it roots below COMPOSITION"
        );
        match client.upload_template(Generation::Adl14, &xml).await {
            Ok(_) | Err(CdrError::Conflict { .. }) => {}
            Err(other) => panic!("the CDR would not take {name}: {other:?}"),
        }

        let gate = Commit {
            definition: &definition,
            validator: &validator,
            envelope: &envelope,
        };
        match gate.create(&client, &ehr, &values).await {
            Ok(_) => {}
            Err(CommitError::Rejected { source, .. }) => {
                refused.push(format!("{name} ({rooted_at}): {source}"));
            }
            Err(other) => panic!("the commit failed for {name}: {other:?}"),
        }
    }

    assert_eq!(
        refused,
        Vec::<String>::new(),
        "the CDR refused a COMPOSITION FerroCHART validated, which is a \
         FerroCHART defect"
    );
}
