// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Every datum a form can hold survives becoming a `DATA_VALUE` and coming
//! back, and every Reference Model invariant the builder checks is refused.
//!
//! Synthetic content invented for these tests. No patient data. All citations
//! are openEHR RM Release-1.1.0 `data_types.html`.

use ferrochart_compose::datum;
use ferrochart_compose::error::BuildError;
use ferrochart_form::ids::{LocalCode, RmAttributeName, RmTypeName};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::values::Datum;

/// A key deep enough to read in a failure message.
fn key() -> NodeKey {
    NodeKey::root().child(KeyStep {
        rm_attribute: RmAttributeName::new("value"),
        node_id: Some(LocalCode::new("at0001")),
        archetype_id: None,
        rm_type: RmTypeName::new("DV_TEXT"),
        pinned_name: None,
        sibling_ordinal: 0,
    })
}

/// Builds `datum` as `rm_type` and reads it straight back.
fn round_trip(rm_type: &str, datum: &Datum) -> Datum {
    let built = datum::build(&key(), rm_type, datum).expect("the datum builds");
    datum::read("/value", &built).expect("the value reads back")
}

/// Every datum shape a field derivation produces, one per kind.
fn every_kind() -> Vec<(&'static str, Datum)> {
    vec![
        ("DV_BOOLEAN", Datum::Boolean(true)),
        ("DV_TEXT", Datum::Text("a synthetic note".to_owned())),
        (
            "DV_CODED_TEXT",
            Datum::Coded {
                terminology: "local".to_owned(),
                code: "at0004".to_owned(),
                rubric: "Mild".to_owned(),
            },
        ),
        (
            "DV_ORDINAL",
            Datum::Ordinal {
                terminology: "local".to_owned(),
                code: "at0005".to_owned(),
                rubric: "Moderate".to_owned(),
                value: 2,
            },
        ),
        (
            "DV_SCALE",
            Datum::Scale {
                terminology: "local".to_owned(),
                code: "at0006".to_owned(),
                rubric: "Half".to_owned(),
                value: 0.5,
            },
        ),
        ("DV_COUNT", Datum::Count(7)),
        (
            "DV_QUANTITY",
            Datum::Quantity {
                magnitude: 37.2,
                units: "Cel".to_owned(),
                precision: Some(1),
            },
        ),
        (
            "DV_PROPORTION",
            Datum::Proportion {
                numerator: 45.0,
                denominator: 100.0,
                kind: 2,
                precision: None,
            },
        ),
        ("DV_DATE", Datum::Date("2026-09-07".to_owned())),
        ("DV_TIME", Datum::Time("14:30:00".to_owned())),
        (
            "DV_DATE_TIME",
            Datum::DateTime("2026-09-07T14:30:00Z".to_owned()),
        ),
        ("DV_DURATION", Datum::Duration("PT30M".to_owned())),
        (
            "DV_IDENTIFIER",
            Datum::Identifier {
                id: "SYN-0001".to_owned(),
                issuer: Some("Ferro synthetic issuer".to_owned()),
                assigner: None,
                identifier_type: Some("local".to_owned()),
            },
        ),
        (
            "DV_URI",
            Datum::Uri("https://ferro.example/synthetic".to_owned()),
        ),
        (
            "DV_EHR_URI",
            Datum::Uri("ehr:/synthetic/composition".to_owned()),
        ),
        (
            "DV_PARSABLE",
            Datum::Parsable {
                value: "a synthetic expression".to_owned(),
                formalism: "text/plain".to_owned(),
            },
        ),
        (
            "DV_INTERVAL<DV_QUANTITY>",
            Datum::Interval {
                lower: Some(Box::new(Datum::Quantity {
                    magnitude: 60.0,
                    units: "mm[Hg]".to_owned(),
                    precision: None,
                })),
                upper: Some(Box::new(Datum::Quantity {
                    magnitude: 90.0,
                    units: "mm[Hg]".to_owned(),
                    precision: None,
                })),
                lower_included: true,
                upper_included: true,
            },
        ),
    ]
}

#[test]
fn every_datum_a_form_holds_survives_the_round_trip() {
    // The whole point of the pair: a value that goes in comes out unchanged.
    // A datum that builds but reads back differently is the silent-wrongness
    // class this project exists to prevent.
    for (rm_type, datum) in every_kind() {
        assert_eq!(
            round_trip(rm_type, &datum),
            datum,
            "a {rm_type} did not survive the round trip"
        );
    }
}

#[test]
fn a_coded_text_carries_its_rubric_as_its_value() {
    // Section 5.2.4: a `DV_CODED_TEXT` is "a combination of a CODE_PHRASE and
    // the rubric of that term, from a terminology service, in the language in
    // which the data were authored".
    let datum = Datum::Coded {
        terminology: "openehr".to_owned(),
        code: "433".to_owned(),
        rubric: "event".to_owned(),
    };
    let built = datum::build(&key(), "DV_CODED_TEXT", &datum).expect("it builds");
    let json = serde_json::to_value(&built).expect("it serialises");
    assert_eq!(json["value"], "event");
    assert_eq!(json["defining_code"]["code_string"], "433");
    assert_eq!(json["defining_code"]["terminology_id"]["value"], "openehr");
}

#[test]
fn an_unbounded_interval_end_is_never_included() {
    // openEHR BASE Release-1.2.0 `foundation_types.html` section 5.2.1:
    // `Lower_included_valid: lower_unbounded implies not lower_included`. A
    // caller that asks for both gets the specification's answer, not its own.
    let datum = Datum::Interval {
        lower: None,
        upper: Some(Box::new(Datum::Count(10))),
        lower_included: true,
        upper_included: true,
    };
    let built = datum::build(&key(), "DV_INTERVAL<DV_COUNT>", &datum).expect("it builds");
    let json = serde_json::to_value(&built).expect("it serialises");
    assert_eq!(json["lower_unbounded"], true);
    assert_eq!(json["lower_included"], false);
    assert_eq!(json["upper_included"], true);
}

#[test]
fn an_ehr_uri_that_is_not_an_ehr_uri_is_refused() {
    // Section 10.3.2: `Scheme_valid: scheme.is_equal ("ehr")`.
    let datum = Datum::Uri("https://ferro.example/not-an-ehr-uri".to_owned());
    match datum::build(&key(), "DV_EHR_URI", &datum).expect_err("it is refused") {
        BuildError::Invariant { invariant, .. } => {
            assert_eq!(invariant, "DV_EHR_URI.Scheme_valid");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn every_proportion_invariant_is_refused_by_name() {
    // Section 6.2.10 states seven invariants, the richest set in the model,
    // and each is a way for a form to build a document a CDR refuses. A
    // refusal names the invariant, because that is what a reader has to go
    // and read.
    let cases: Vec<(&str, Datum)> = vec![
        (
            "DV_PROPORTION.Valid_denominator",
            Datum::Proportion {
                numerator: 1.0,
                denominator: 0.0,
                kind: 0,
                precision: None,
            },
        ),
        (
            "DV_PROPORTION.Type_validity",
            Datum::Proportion {
                numerator: 1.0,
                denominator: 2.0,
                kind: 9,
                precision: None,
            },
        ),
        (
            "DV_PROPORTION.Unitary_validity",
            Datum::Proportion {
                numerator: 1.0,
                denominator: 2.0,
                kind: 1,
                precision: None,
            },
        ),
        (
            "DV_PROPORTION.Percent_validity",
            Datum::Proportion {
                numerator: 1.0,
                denominator: 50.0,
                kind: 2,
                precision: None,
            },
        ),
        (
            "DV_PROPORTION.Fraction_validity",
            Datum::Proportion {
                numerator: 1.5,
                denominator: 2.0,
                kind: 3,
                precision: None,
            },
        ),
        (
            "DV_PROPORTION.Precision_validity",
            Datum::Proportion {
                numerator: 1.5,
                denominator: 2.5,
                kind: 0,
                precision: Some(0),
            },
        ),
    ];
    for (expected, datum) in cases {
        let failure = datum::build(&key(), "DV_PROPORTION", &datum)
            .err()
            .unwrap_or_else(|| panic!("{expected} was not refused"));
        match failure {
            BuildError::Invariant { invariant, .. } => assert_eq!(invariant, expected),
            other => panic!("the refusal for {expected} is {other:?}"),
        }
    }
}

#[test]
fn an_empty_identifier_and_an_empty_formalism_are_refused() {
    // `DV_IDENTIFIER.Id_valid: not id.is_empty` (section 4.2.4) and
    // `DV_PARSABLE.Formalism_valid: not formalism.is_empty` (section 9.2.3).
    let empty_id = Datum::Identifier {
        id: String::new(),
        issuer: None,
        assigner: None,
        identifier_type: None,
    };
    match datum::build(&key(), "DV_IDENTIFIER", &empty_id).expect_err("it is refused") {
        BuildError::Invariant { invariant, .. } => {
            assert_eq!(invariant, "DV_IDENTIFIER.Id_valid");
        }
        other => panic!("the refusal is {other:?}"),
    }

    let empty_formalism = Datum::Parsable {
        value: "anything".to_owned(),
        formalism: String::new(),
    };
    match datum::build(&key(), "DV_PARSABLE", &empty_formalism).expect_err("it is refused") {
        BuildError::Invariant { invariant, .. } => {
            assert_eq!(invariant, "DV_PARSABLE.Formalism_valid");
        }
        other => panic!("the refusal is {other:?}"),
    }
}

#[test]
fn an_interval_end_that_is_not_ordered_is_refused() {
    // Section 6.2.2: "The type parameter, T, must be a descendant of the type
    // DV_ORDERED." A boolean is not one, and a form that offered one would
    // build a document no CDR accepts.
    let datum = Datum::Interval {
        lower: Some(Box::new(Datum::Boolean(true))),
        upper: None,
        lower_included: true,
        upper_included: false,
    };
    match datum::build(&key(), "DV_INTERVAL", &datum).expect_err("it is refused") {
        BuildError::WrongDatum { found, .. } => assert_eq!(found, "a DV_BOOLEAN"),
        other => panic!("the refusal is {other:?}"),
    }
}
