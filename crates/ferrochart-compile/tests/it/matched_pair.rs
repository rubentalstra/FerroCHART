// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One clinical constraint, written in both generations, read into one model.
//!
//! `docs/architecture.md` names this the narrow waist: both readers normalize
//! into the internal constraint model so the field derivation is written once.
//! The two source artefacts below state the same synthetic observation, one as
//! an ADL 1.4 operational template and one as ADL 2 source.

use ferrochart_compile::model::ids::{CodeKind, LocalCode};
use ferrochart_compile::model::node::{ConstraintNode, ConstraintTemplate};
use ferrochart_compile::model::payload::{CodeSource, ConstraintPayload};
use ferrochart_compile::{adl2, adl14};

const AS_OPT14: &str = include_str!("../fixtures/ferro_test_observation.opt");
const AS_ADL2: &str = include_str!("../fixtures/ferro_test_observation.v1.0.0.adls");

fn pair() -> (ConstraintTemplate, ConstraintTemplate) {
    (
        adl14::from_xml(AS_OPT14).expect("the ADL 1.4 fixture reads"),
        adl2::from_source(AS_ADL2, &[]).expect("the ADL 2 fixture reads"),
    )
}

/// The rubric a node's own terminology gives its code, which is the name of
/// the clinical concept rather than the code that happens to spell it.
fn rubric<'a>(template: &'a ConstraintTemplate, node: &ConstraintNode) -> Option<&'a str> {
    let code = node.identity().node_id()?;
    template
        .terminology(node.terminology_scope())?
        .rubric(template.language(), code)
        .map(ferrochart_compile::model::terminology::TermDefinition::text)
}

/// Asserts that two subtrees express the same constraint, without requiring
/// the two generations to spell their codes the same way.
fn assert_equivalent(
    left_template: &ConstraintTemplate,
    left: &ConstraintNode,
    right_template: &ConstraintTemplate,
    right: &ConstraintNode,
    path: &str,
) {
    assert_eq!(
        left.identity().rm_attribute(),
        right.identity().rm_attribute(),
        "at {path}"
    );
    assert_eq!(
        left.identity().rm_type(),
        right.identity().rm_type(),
        "at {path}"
    );
    assert_eq!(left.occurrences(), right.occurrences(), "at {path}");
    assert_eq!(
        left.attribute().is_container(),
        right.attribute().is_container(),
        "at {path}"
    );
    assert_eq!(
        left.identity().pinned_name(),
        right.identity().pinned_name(),
        "at {path}"
    );
    assert_equivalent_payload(left_template, left, right_template, right, path);
    assert_eq!(
        left.children().len(),
        right.children().len(),
        "at {path}: {:?} against {:?}",
        left.children()
            .iter()
            .map(|c| c.identity().rm_attribute().as_str())
            .collect::<Vec<_>>(),
        right
            .children()
            .iter()
            .map(|c| c.identity().rm_attribute().as_str())
            .collect::<Vec<_>>(),
    );
    for (index, (left_child, right_child)) in left
        .children()
        .iter()
        .zip(right.children().iter())
        .enumerate()
    {
        let child_path = format!("{path}/{}[{index}]", left_child.identity().rm_attribute());
        assert_equivalent(
            left_template,
            left_child,
            right_template,
            right_child,
            &child_path,
        );
    }
}

fn assert_equivalent_payload(
    left_template: &ConstraintTemplate,
    left: &ConstraintNode,
    right_template: &ConstraintTemplate,
    right: &ConstraintNode,
    path: &str,
) {
    assert_eq!(left.payload().kind(), right.payload().kind(), "at {path}");
    match (left.payload(), right.payload()) {
        (ConstraintPayload::Coded(from_opt14), ConstraintPayload::Coded(from_adl2)) => {
            let (
                CodeSource::Enumerated {
                    terminology: left_terminology,
                    codes: left_codes,
                },
                CodeSource::Enumerated {
                    terminology: right_terminology,
                    codes: right_codes,
                },
            ) = (&from_opt14.source, &from_adl2.source)
            else {
                panic!("at {path}: both sides state an enumeration");
            };
            assert_eq!(left_terminology, right_terminology, "at {path}");
            // The codes differ between generations; the concepts do not.
            let text = |template: &ConstraintTemplate, node: &ConstraintNode, codes: &[String]| {
                codes
                    .iter()
                    .map(|code| {
                        template
                            .terminology(node.terminology_scope())
                            .and_then(|terminology| {
                                terminology
                                    .rubric(template.language(), &LocalCode::new(code.clone()))
                            })
                            .map_or_else(
                                || panic!("at {path}: {code} has no rubric"),
                                |term| term.text().to_owned(),
                            )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                text(left_template, left, left_codes),
                text(right_template, right, right_codes),
                "at {path}"
            );
        }
        (left_payload, right_payload) => {
            assert_eq!(left_payload, right_payload, "at {path}");
        }
    }
}

#[test]
fn both_generations_produce_the_same_internal_model() {
    let (opt14, adl2) = pair();
    assert_equivalent(&opt14, opt14.root(), &adl2, adl2.root(), "");
}

#[test]
fn the_node_identity_of_each_generation_carries_its_own_code_kind() {
    // openEHR AM Release-2.3.0 ADL1.4.html section 7.2 identifies a node with
    // an `at`-code; `ADL2.html` section 7.13.5.1 identifies it with an
    // `id`-code and leaves `at`-codes for values. Both occupy the same slot of
    // the internal model, which is the whole mapping.
    let (opt14, adl2) = pair();
    for node in opt14.walk() {
        if let Some(code) = node.identity().node_id() {
            assert_eq!(code.kind(), CodeKind::Term, "{code} in the ADL 1.4 model");
        }
    }
    for node in adl2.walk() {
        if let Some(code) = node.identity().node_id() {
            assert_eq!(code.kind(), CodeKind::Node, "{code} in the ADL 2 model");
        }
    }
}

#[test]
fn the_same_concept_carries_the_same_rubric_in_both_generations() {
    let (opt14, adl2) = pair();
    assert_eq!(
        rubric(&opt14, opt14.root()),
        rubric(&adl2, adl2.root()),
        "the two roots name different concepts"
    );
}

#[test]
fn neither_model_records_which_reader_produced_it() {
    // The model carries no generation marker, so the only honest test is that
    // the two trees compare equal wherever they state the same constraint.
    let (opt14, adl2) = pair();
    assert_eq!(opt14.walk().count(), adl2.walk().count());
    for (left, right) in opt14.walk().zip(adl2.walk()) {
        assert_eq!(left.payload().kind(), right.payload().kind());
        assert_eq!(left.is_deprecated(), right.is_deprecated());
    }
}

#[test]
fn a_slot_assertion_reads_the_same_in_both_generations() {
    // ADL 1.4 states the bare archetype-id expression in the assertion's
    // `C_STRING.pattern`; ADL 2 writes a contained regexp with delimiters.
    // Both readers record the bare expression, so a slot compares.
    let from_opt14 = adl14::from_xml(include_str!("../fixtures/ferro_slots_and_refs.opt"))
        .expect("the ADL 1.4 fixture reads");
    let from_adl2 = adl2::from_source(
        include_str!("../fixtures/ferro_adl2_extras.v1.0.0.adls"),
        &[],
    )
    .expect("the ADL 2 fixture reads");
    let includes = |template: &ConstraintTemplate| {
        template
            .walk()
            .find_map(|node| match *node.payload() {
                ConstraintPayload::OpenSlot(ref slot) => Some(slot.includes.clone()),
                _ => None,
            })
            .expect("the fixture carries an open slot")
    };
    assert_eq!(includes(&from_opt14), includes(&from_adl2));
}
