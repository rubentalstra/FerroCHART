// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The FHIR model this crate speaks comes from `fhir-types`
//! (`docs/architecture.md` sections 2 and 7.3). These tests pin that the
//! model links, that a value set FerroCHART mints for an archetype survives
//! the FHIR JSON codec, and that an `$expand` request reaches the wire as
//! the `Parameters` a terminology server reads.
//!
//! Every code and URL below is invented for the test.

use fhir_types::codec::{Json, Path};
use fhir_types::r4::operations::value_set_expand::ValueSetExpandRequest;
use fhir_types::r4::parameters::ParametersParameterValue;
use fhir_types::r4::primitives::{Code, Integer, String as FhirString, Uri};
use fhir_types::r4::value_set::{
    ValueSet, ValueSetCompose, ValueSetComposeInclude, ValueSetComposeIncludeConcept,
};

/// The canonical URL of the minted value set, in the shape of
/// `docs/architecture.md` section 7.2 under a documentation domain.
const VALUE_SET_URL: &str =
    "https://fhir.example.org/ValueSet/openEHR-EHR-OBSERVATION.pulse_oximetry.v1--ac0.1";

/// The code system the minted value set composes, one per archetype.
const CODE_SYSTEM_URL: &str =
    "https://fhir.example.org/CodeSystem/openEHR-EHR-OBSERVATION.pulse_oximetry.v1";

/// A value set of the shape section 7.2 mints for an archetype's `ac`-code:
/// two local `at`-codes with their rubrics, composed from the archetype's own
/// code system.
fn minted_value_set() -> ValueSet {
    ValueSet {
        url: Some(Uri::from(VALUE_SET_URL)),
        version: Some(FhirString::from("v1")),
        name: Some(FhirString::from("PulseOximetryMethod")),
        status: Code::from("active"),
        compose: Some(ValueSetCompose {
            include: vec![ValueSetComposeInclude {
                system: Some(Uri::from(CODE_SYSTEM_URL)),
                concept: vec![
                    ValueSetComposeIncludeConcept {
                        code: Code::from("at0011"),
                        display: Some(FhirString::from("Fingertip")),
                        ..ValueSetComposeIncludeConcept::default()
                    },
                    ValueSetComposeIncludeConcept {
                        code: Code::from("at0012"),
                        display: Some(FhirString::from("Earlobe")),
                        ..ValueSetComposeIncludeConcept::default()
                    },
                ],
                ..ValueSetComposeInclude::default()
            }],
            ..ValueSetCompose::default()
        }),
        ..ValueSet::default()
    }
}

#[test]
fn a_minted_value_set_survives_the_fhir_json_codec() {
    let value_set = minted_value_set();

    let object = value_set.to_json().expect("the value set has a JSON form");

    assert_eq!(
        object
            .get("resourceType")
            .and_then(serde_json::Value::as_str),
        Some("ValueSet"),
        "a FHIR resource carries its type in the JSON it is sent as"
    );
    assert_eq!(
        object.get("url").and_then(serde_json::Value::as_str),
        Some(VALUE_SET_URL)
    );

    let mut path = Path::root("ValueSet");
    let decoded = ValueSet::from_json(&object, &mut path).expect("the encoding decodes back");

    assert_eq!(decoded, value_set, "the codec round-trips without loss");
}

#[test]
fn an_expand_request_reaches_the_wire_as_parameters() {
    let request = ValueSetExpandRequest {
        url: Some(Uri::from(VALUE_SET_URL)),
        count: Some(Integer::from(200)),
        ..ValueSetExpandRequest::default()
    };

    let parameters = request.to_parameters();

    let names: Vec<&str> = parameters
        .parameter
        .iter()
        .filter_map(|parameter| parameter.name.value.as_deref())
        .collect();
    assert_eq!(names, ["url", "count"], "only what was set is sent");

    let url = parameters
        .parameter
        .first()
        .and_then(|parameter| parameter.value.as_ref());
    match url {
        Some(ParametersParameterValue::Uri(uri)) => {
            assert_eq!(uri.value.as_deref(), Some(VALUE_SET_URL));
        }
        other => panic!("url is sent as a uri parameter, found {other:?}"),
    }

    let read_back =
        ValueSetExpandRequest::from_parameters(&parameters).expect("a server reads it back");
    assert_eq!(read_back, request);
}
