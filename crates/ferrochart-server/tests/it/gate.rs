// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The gate, without a CDR.
//!
//! The cases here point the client at a port nothing listens on, so a request
//! that leaves the process fails loudly. That is what makes them a test of
//! the ordering rather than of the validator: a refusal proves the gate ran
//! first, and a transport failure would prove it did not.

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::ids::EhrId;
use ferrochart_form::validation::FailureKind;
use ferrochart_form::values::{Datum, Entered};
use ferrochart_server::commit::{Commit, CommitError};
use url::Url;

use crate::support;

/// A CDR that cannot answer, because nothing is listening.
///
/// Port 9 is the discard service, which no CDR runs on.
fn unreachable() -> CdrClient {
    let base = Url::parse("http://127.0.0.1:9/openehr").expect("the base is a URL");
    CdrClient::new(&base).expect("the client builds")
}

#[tokio::test]
async fn a_composition_the_template_refuses_never_reaches_the_wire() {
    // Acceptance criterion 1 of #24, tested by construction: the client
    // cannot reach a server, so the only way this can end in a refusal is if
    // the validation ran before the request.
    let (definition, validator, mut values) = support::case();
    let envelope = support::envelope();

    // A quantity outside every range its template states. The form's own
    // field carries those ranges, so this is a value the form refuses, put in
    // behind the form's back.
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
    values.set(
        field.key.clone(),
        Entered::Value(Datum::Quantity {
            magnitude: 1_000_000.0,
            units,
            precision: None,
        }),
    );

    let gate = Commit {
        definition: &definition,
        validator: &validator,
        envelope: &envelope,
    };
    let outcome = gate
        .create(&unreachable(), &EhrId::new("ferro-test"), &values)
        .await
        .expect_err("the document does not conform");

    match outcome {
        CommitError::Refused { report } => {
            assert!(
                report
                    .failures
                    .iter()
                    .any(|failure| failure.kind == FailureKind::RangeError),
                "{:#?}",
                report.failures
            );
            assert!(
                report.at(&field.key).next().is_some(),
                "the refusal reaches the field a renderer would show it on"
            );
        }
        CommitError::Wire { source } => {
            panic!("the request left the process before the gate ran: {source}")
        }
        other => panic!("the outcome is {other:?}"),
    }
}

#[tokio::test]
async fn a_composition_the_template_admits_reaches_the_wire() {
    // The control. Without it the case above would pass on a gate that
    // refuses everything, and nothing would ever be committed.
    let (definition, validator, values) = support::case();
    let envelope = support::envelope();
    let gate = Commit {
        definition: &definition,
        validator: &validator,
        envelope: &envelope,
    };

    assert!(
        gate.validated(&values).is_ok(),
        "the filled form builds a conforming document"
    );

    match gate
        .create(&unreachable(), &EhrId::new("ferro-test"), &values)
        .await
        .expect_err("nothing is listening")
    {
        CommitError::Wire { .. } => {}
        other => panic!("the outcome is {other:?}"),
    }
}
