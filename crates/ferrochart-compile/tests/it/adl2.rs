// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the ADL 2 reader does with the constraint kinds AOM 2 adds, and what
//! it refuses.

use ferrochart_compile::adl2;
use ferrochart_compile::error::ReadError;
use ferrochart_compile::model::ids::{CodeKind, LocalCode, TerminologyName};
use ferrochart_compile::model::node::{ConstraintNode, ConstraintTemplate};
use ferrochart_compile::model::payload::{
    BindingStatus, CodeSource, ConstraintPayload, ExternalSet, SlotAssertion,
};

const OBSERVATION: &str = include_str!("../fixtures/ferro_test_observation.v1.0.0.adls");
const EXTRAS: &str = include_str!("../fixtures/ferro_adl2_extras.v1.0.0.adls");
const REQUIRED_SLOT: &str = include_str!("../fixtures/ferro_adl2_required_slot.v1.0.0.adls");

fn read(source: &str) -> ConstraintTemplate {
    adl2::from_source(source, &[]).expect("the fixture reads")
}

fn node_with_id<'a>(template: &'a ConstraintTemplate, node_id: &str) -> &'a ConstraintNode {
    template
        .walk()
        .find(|node| node.identity().node_id().map(LocalCode::as_str) == Some(node_id))
        .unwrap_or_else(|| panic!("no node {node_id}"))
}

#[test]
fn a_node_the_template_removed_is_absent() {
    // openEHR AM Release-2.3.0 OPT2.html section 3.3: an object node with
    // `occurrences matches {0}` is removed from the operational template.
    let template = read(OBSERVATION);
    assert!(
        template
            .walk()
            .all(|node| node.identity().node_id().map(LocalCode::as_str) != Some("id9")),
        "the node the template removed is still in the model"
    );
}

#[test]
fn an_adl2_node_identity_is_an_id_code() {
    // openEHR AM Release-2.3.0 ADL2.html section 7.13.5.1 splits the code
    // space: `id`-codes identify nodes, `at`-codes identify values, and
    // `ac`-codes name value sets.
    let template = read(OBSERVATION);
    assert_eq!(
        template.root().identity().node_id().map(LocalCode::kind),
        Some(CodeKind::Node)
    );
}

#[test]
fn an_ac_code_whose_members_the_terminology_lists_reads_as_that_enumeration() {
    // A value set the archetype enumerates needs no server, so it becomes the
    // same enumeration an ADL 1.4 `C_CODE_PHRASE` code list becomes.
    let template = read(OBSERVATION);
    let coded = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Coded(_)))
        .expect("the fixture constrains a coded value");
    let ConstraintPayload::Coded(ref constraint) = *coded.payload() else {
        panic!("expected a coded constraint");
    };
    assert_eq!(
        constraint.source,
        CodeSource::Enumerated {
            terminology: TerminologyName::new("local"),
            codes: vec!["at1".to_owned(), "at2".to_owned()],
        }
    );
}

#[test]
fn an_ac_code_with_no_members_is_a_binding_a_server_expands() {
    let template = read(EXTRAS);
    let coded = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Coded(_)))
        .expect("the fixture constrains a coded value");
    let ConstraintPayload::Coded(ref constraint) = *coded.payload() else {
        panic!("expected a coded constraint");
    };
    let CodeSource::External(ExternalSet::ConstraintCode {
        ref code,
        ref bindings,
        ..
    }) = constraint.source
    else {
        panic!("expected a constraint-code binding");
    };
    assert_eq!(code.as_str(), "ac2");
    assert_eq!(
        bindings
            .get(&TerminologyName::new("ferro_test"))
            .map(String::as_str),
        Some("terminology://ferro.example/valueset/synthetic-findings")
    );
}

#[test]
fn a_constraint_status_is_represented_rather_than_dropped() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.5.10 gives
    // `C_PRIMITIVE_OBJECT.constraint_status`; ADL 1.4 has no way to say this.
    let template = read(EXTRAS);
    let coded = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Coded(_)))
        .expect("the fixture constrains a coded value");
    let ConstraintPayload::Coded(ref constraint) = *coded.payload() else {
        panic!("expected a coded constraint");
    };
    assert_eq!(constraint.status, Some(BindingStatus::Preferred));
}

#[test]
fn an_ordinal_tuple_folds_into_the_same_payload_the_domain_type_produces() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.5.26 ties `symbol` and
    // `value` with a `C_ATTRIBUTE_TUPLE`, where ADL 1.4 has a `C_DV_ORDINAL`.
    let template = read(EXTRAS);
    let ConstraintPayload::Ordinal(ref ordinal) = *node_with_id(&template, "id4").payload() else {
        panic!("expected an ordinal");
    };
    assert_eq!(ordinal.options.len(), 2);
    let scores: Vec<String> = ordinal
        .options
        .iter()
        .map(|option| option.value.to_string())
        .collect();
    assert_eq!(scores, vec!["0".to_owned(), "1".to_owned()]);
    assert_eq!(ordinal.options[0].symbol.code(), "at1");
    assert_eq!(ordinal.options[1].symbol.code(), "at2");
    // openEHR AM Release-2.3.0 AOM2.html section 4.3 keeps a tuple's member
    // attributes inside the tuple, so the folded node holds no children.
    assert!(node_with_id(&template, "id4").children().is_empty());
}

#[test]
fn an_open_slot_is_recorded_with_the_archetypes_it_admits() {
    let template = read(EXTRAS);
    let ConstraintPayload::OpenSlot(ref slot) = *node_with_id(&template, "id7").payload() else {
        panic!("expected an open slot");
    };
    assert_eq!(slot.includes.len(), 1);
    assert!(
        matches!(slot.includes[0], SlotAssertion::ArchetypeIdPattern(ref pattern)
            if pattern.contains("ferro_")),
        "{:?}",
        slot.includes
    );
}

#[test]
fn a_slot_the_template_requires_and_never_filled_is_refused() {
    let error = adl2::from_source(REQUIRED_SLOT, &[]).expect_err("a required slot is refused");
    let ReadError::RequiredSlotUnfilled {
        ref node_id,
        ref rm_type,
        ..
    } = error
    else {
        panic!("expected a required-slot refusal, got {error}");
    };
    assert_eq!(node_id, "id2");
    assert_eq!(rm_type, "CLUSTER");
}

#[test]
fn a_source_that_does_not_parse_is_refused_with_every_error() {
    let error = adl2::parse("archetype (adl_version=2.0.5)\n").expect_err("a broken source");
    let ReadError::AdlSyntax { ref all, .. } = error else {
        panic!("expected a syntax refusal, got {error}");
    };
    assert!(!all.is_empty(), "the refusal carries no error");
}

#[test]
fn unstated_occurrences_are_inferred_from_the_containing_attribute() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.5.2 makes
    // `C_OBJECT.occurrences` optional and infers an unstated value from the
    // existence and cardinality of the containing attribute.
    let template = read(OBSERVATION);
    let defining_code = template
        .walk()
        .find(|node| node.identity().rm_attribute().as_str() == "defining_code")
        .expect("the fixture constrains a defining code");
    assert!(!defining_code.occurrences_stated());
    assert_eq!(defining_code.occurrences().to_string(), "0..1");
    let quantity = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Quantity(_)))
        .expect("the fixture constrains a quantity");
    assert!(quantity.occurrences_stated());
}

#[test]
fn a_contained_regexp_is_a_pattern_rather_than_a_literal() {
    // openEHR AM Release-2.3.0 ADL2.html section 4.5 writes a regular
    // expression as a contained regexp, delimited by `/` or `^`, in the same
    // list as the literal alternatives. ADL 1.4 states the bare expression in
    // `C_STRING.pattern`, so both readers record the bare one.
    let template = read(EXTRAS);
    let ConstraintPayload::Text(ref text) = *node_with_id(&template, "id9")
        .children()
        .first()
        .expect("the text constrains a value")
        .payload()
    else {
        panic!("expected a text constraint");
    };
    assert_eq!(text.patterns, vec!["[A-Z]{2}".to_owned()]);
    assert!(text.list.is_empty());
}

#[test]
fn a_tuple_this_reader_does_not_fold_is_carried_whole() {
    // openEHR AM Release-2.3.0 AOM2.html section 4.5.25 ties any sibling
    // attributes with a tuple. A pairing that is neither a quantity nor an
    // ordinal has no ADL 1.4 equivalent, and is represented rather than
    // dropped.
    let template = read(EXTRAS);
    let ConstraintPayload::Tuple(ref tuple) = *node_with_id(&template, "id11").payload() else {
        panic!("expected a tuple");
    };
    let members: Vec<&str> = tuple
        .members
        .iter()
        .map(ferrochart_compile::model::ids::RmAttributeName::as_str)
        .collect();
    assert_eq!(members, vec!["numerator", "denominator"]);
    assert_eq!(tuple.rows.len(), 1);
    assert_eq!(tuple.rows[0].len(), 2);
    // AOM 2 keeps a tuple's member attributes inside the tuple rather than
    // beside them, so the whole pairing lives in this payload.
    assert!(node_with_id(&template, "id11").children().is_empty());
}
