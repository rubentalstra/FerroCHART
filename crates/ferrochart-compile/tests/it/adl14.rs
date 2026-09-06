// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the ADL 1.4 reader does with each constrainer the ITS-XML family
//! declares, and what it refuses.

use ferrochart_compile::adl14;
use ferrochart_compile::error::ReadError;
use ferrochart_compile::model::ids::{CodeKind, LocalCode, TerminologyName};
use ferrochart_compile::model::node::ConstraintTemplate;
use ferrochart_compile::model::payload::{
    CodeSource, ConstraintPayload, DefaultValue, ExternalSet, SlotAssertion,
};

const OBSERVATION: &str = include_str!("../fixtures/ferro_test_observation.opt");
const SLOTS_AND_REFS: &str = include_str!("../fixtures/ferro_slots_and_refs.opt");
const REQUIRED_SLOT: &str = include_str!("../fixtures/ferro_required_slot.opt");
const XSD_ONLY: &str = include_str!("../fixtures/ferro_xsd_only.opt");

fn read(xml: &str) -> ConstraintTemplate {
    adl14::from_xml(xml).expect("the fixture reads")
}

/// The node at `node_id`, wherever it sits.
fn node_with_id<'a>(
    template: &'a ConstraintTemplate,
    node_id: &str,
) -> &'a ferrochart_compile::model::node::ConstraintNode {
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
            .all(|node| node.identity().node_id().map(LocalCode::as_str) != Some("at0007")),
        "the node the template removed is still in the model"
    );
}

#[test]
fn an_adl14_node_identity_is_an_at_code() {
    // openEHR AM Release-2.3.0 ADL1.4.html section 7.2: ADL 1.4 identifies a
    // node with an `at`-code, and section 4.2.3.1 lets a leaf with no siblings
    // carry none at all.
    let template = read(OBSERVATION);
    let root = template.root();
    assert_eq!(
        root.identity().node_id().map(LocalCode::kind),
        Some(CodeKind::Term)
    );
    let quantity = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Quantity(_)))
        .expect("the fixture constrains a quantity");
    assert!(quantity.identity().node_id().is_none());
}

#[test]
fn a_quantity_carries_its_units_with_their_own_range() {
    // openEHR RM Release-1.1.0 data_types.html section 6.2.8: each permitted
    // unit carries its own magnitude interval.
    let template = read(OBSERVATION);
    let ConstraintPayload::Quantity(ref quantity) = *node_with_id(&template, "at0002")
        .children()
        .first()
        .expect("the element constrains a value")
        .payload()
    else {
        panic!("expected a quantity");
    };
    assert_eq!(quantity.units.len(), 1);
    assert_eq!(quantity.units[0].units, "mm[Hg]");
    let magnitude = quantity.units[0]
        .magnitude
        .as_ref()
        .expect("the unit states a range");
    assert_eq!(magnitude.lower(), Some(&0.0));
    assert!(magnitude.lower_included());
    assert_eq!(magnitude.upper(), None);
    assert!(
        !magnitude.upper_included(),
        "an absent end is never included"
    );
}

#[test]
fn a_local_code_list_is_an_enumeration_over_the_archetype_terminology() {
    // openEHR RM Release-1.1.0 data_types.html section 5.2.3: the terminology
    // `local` names the archetype's own codes.
    let template = read(OBSERVATION);
    let coded = template
        .walk()
        .find(|node| matches!(*node.payload(), ConstraintPayload::Coded(_)))
        .expect("the fixture constrains a coded value");
    let ConstraintPayload::Coded(ref constraint) = *coded.payload() else {
        panic!("expected a coded constraint");
    };
    let CodeSource::Enumerated {
        ref terminology,
        ref codes,
    } = constraint.source
    else {
        panic!("expected an enumeration");
    };
    assert_eq!(terminology, &TerminologyName::new("local"));
    assert_eq!(codes, &["at0004".to_owned(), "at0005".to_owned()]);
    // The rubric of each code comes from the archetype's own terminology.
    let scope = template
        .terminology(coded.terminology_scope())
        .expect("the archetype carries a terminology");
    assert_eq!(
        scope
            .rubric(template.language(), &LocalCode::new("at0004"))
            .map(ferrochart_compile::model::terminology::TermDefinition::text),
        Some("Sitting")
    );
}

#[test]
fn a_use_node_reference_is_resolved_against_the_same_template() {
    // openEHR AM Release-2.3.0 OPT2.html section 3.3 expands every `use_node`
    // reference inline; the ADL 1.4 flattener is a vendor tool and does not
    // always do so.
    let template = read(SLOTS_AND_REFS);
    let target = node_with_id(&template, "at0002");
    let reference = node_with_id(&template, "at0004");
    assert_eq!(
        reference.children().len(),
        target.children().len(),
        "the reference did not expand into a copy of its target"
    );
    assert_eq!(reference.identity().rm_type(), target.identity().rm_type());
    assert_eq!(
        reference.identity().pinned_name(),
        target.identity().pinned_name()
    );
    // The reference's own occurrences win at the point it stands.
    assert_eq!(reference.occurrences().to_string(), "0..*");
    assert_eq!(target.occurrences().to_string(), "0..1");
}

#[test]
fn an_unresolvable_use_node_reference_is_refused() {
    let broken = SLOTS_AND_REFS.replace(
        "<target_path>/data[at0001]/items[at0002]</target_path>",
        "<target_path>/data[at0001]/items[at9999]</target_path>",
    );
    let error = adl14::from_xml(&broken).expect_err("a missing target is refused");
    assert!(
        matches!(error, ReadError::UnresolvedInternalReference { ref target, .. }
            if target == "/data[at0001]/items[at9999]"),
        "expected an unresolved-reference refusal, got {error}"
    );
}

#[test]
fn the_pinned_name_is_lifted_into_the_node_identity() {
    let template = read(SLOTS_AND_REFS);
    assert_eq!(
        node_with_id(&template, "at0002").identity().pinned_name(),
        Some("Left arm")
    );
}

#[test]
fn an_open_slot_is_recorded_with_the_archetypes_it_admits() {
    let template = read(SLOTS_AND_REFS);
    let ConstraintPayload::OpenSlot(ref slot) = *node_with_id(&template, "at0005").payload() else {
        panic!("expected an open slot");
    };
    assert_eq!(
        slot.includes,
        vec![SlotAssertion::ArchetypeIdPattern(
            r"openEHR-EHR-CLUSTER\.ferro_.*\.v1".to_owned()
        )]
    );
    assert!(slot.excludes.is_empty());
}

#[test]
fn a_slot_the_template_requires_and_never_filled_is_refused() {
    // openEHR AM Release-2.3.0 OPT2.html section 3.3 fills a slot by inlining
    // the archetype that fills it. Content this compiler cannot determine and
    // the template will not do without is a template it cannot compile.
    let error = adl14::from_xml(REQUIRED_SLOT).expect_err("a required unfilled slot is refused");
    let ReadError::RequiredSlotUnfilled {
        ref node_id,
        ref rm_type,
        occurrences,
        ..
    } = error
    else {
        panic!("expected a required-slot refusal, got {error}");
    };
    assert_eq!(node_id, "at0001");
    assert_eq!(rm_type, "CLUSTER");
    assert_eq!(occurrences.to_string(), "1");
}

#[test]
fn a_reference_set_uri_is_a_binding_a_server_expands() {
    // `C_CODE_REFERENCE` is declared only in the ITS-XML `Template.xsd`;
    // neither AOM 1.4 nor AOM 2 defines the class.
    let template = read(XSD_ONLY);
    let ConstraintPayload::Coded(ref constraint) = *node_with_id(&template, "at0001")
        .children()
        .first()
        .expect("the element constrains a value")
        .payload()
    else {
        panic!("expected a coded constraint");
    };
    assert_eq!(
        constraint.source,
        CodeSource::External(ExternalSet::ReferenceSet {
            uri: "terminology://ferro.example/valueset/synthetic-positions".to_owned()
        })
    );
}

#[test]
fn a_constraint_ref_carries_the_bindings_the_template_states() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.3.14: a `CONSTRAINT_REF`
    // names an `ac`-code resolved through the archetype's constraint bindings.
    let template = read(XSD_ONLY);
    let ConstraintPayload::Coded(ref constraint) = *node_with_id(&template, "at0002")
        .children()
        .first()
        .expect("the element constrains a value")
        .payload()
    else {
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
    assert_eq!(code.as_str(), "ac0001");
    assert_eq!(code.kind(), CodeKind::ValueSet);
    assert_eq!(
        bindings
            .get(&TerminologyName::new("SNOMED-CT"))
            .map(String::as_str),
        Some("terminology://ferro.example/valueset/synthetic-positions")
    );
}

#[test]
fn a_state_machine_is_read_from_the_only_definition_there_is() {
    // `C_DV_STATE`, `STATE_MACHINE`, `NON_TERMINAL_STATE`, `TERMINAL_STATE`
    // and `TRANSITION` are declared in the ITS-XML `OpenehrProfile.xsd` and by
    // no AOM prose in either generation, so the XSD is the whole definition.
    let template = read(XSD_ONLY);
    let ConstraintPayload::State(ref state) = *node_with_id(&template, "at0003")
        .children()
        .first()
        .expect("the element constrains a value")
        .payload()
    else {
        panic!("expected a state constraint");
    };
    assert_eq!(state.states.len(), 2);
    assert_eq!(state.states[0].name, "planned");
    assert!(!state.states[0].is_terminal);
    assert_eq!(state.states[0].transitions[0].event, "start");
    assert_eq!(
        state.states[0].transitions[0].next_state.as_deref(),
        Some("completed")
    );
    assert!(state.states[1].is_terminal);
}

#[test]
fn a_default_value_reaches_the_node_its_differential_path_names() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.2.3.2: a default value
    // does appear in data, unlike an assumed value.
    let template = read(XSD_ONLY);
    let value = node_with_id(&template, "at0004")
        .children()
        .first()
        .expect("the element constrains a value");
    assert_eq!(
        value.default_value(),
        Some(&DefaultValue::Text("A synthetic default".to_owned()))
    );
}

#[test]
fn a_default_value_naming_a_node_the_template_does_not_define_is_refused() {
    let broken = XSD_ONLY.replace(
        "/data[at0004]</differential_path>",
        "/data[at9999]</differential_path>",
    );
    let error = adl14::from_xml(&broken).expect_err("a dangling default path is refused");
    assert!(
        matches!(error, ReadError::UnresolvedDefaultPath { .. }),
        "expected an unresolved-default refusal, got {error}"
    );
}

#[test]
fn a_primitive_rm_type_is_recorded_under_its_openehr_base_name() {
    // openEHR BASE Release-1.2.0 foundation_types.html names the classes
    // `String`, `Integer`, `Real`, `Boolean` and the `Iso8601_*` family; a
    // real `.opt` spells them in upper case.
    let template = read(SLOTS_AND_REFS);
    let names: Vec<&str> = template
        .walk()
        .map(|node| node.identity().rm_type().as_str())
        .collect();
    assert!(names.contains(&"String"), "{names:?}");
    assert!(names.contains(&"Integer"), "{names:?}");
    assert!(!names.contains(&"STRING"), "{names:?}");
    assert!(!names.contains(&"INTEGER"), "{names:?}");
}

#[test]
fn a_container_attribute_carries_its_cardinality() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.2.2 keeps existence,
    // cardinality and occurrences apart.
    let template = read(OBSERVATION);
    let element = node_with_id(&template, "at0002");
    let attribute = element.attribute();
    assert!(attribute.is_container());
    let cardinality = attribute.cardinality().expect("a container states one");
    assert!(cardinality.is_ordered());
    assert!(!cardinality.is_unique());
    assert_eq!(cardinality.interval().to_string(), "0..*");
    assert_eq!(
        attribute.existence().map(|e| e.to_string()).as_deref(),
        Some("0..1")
    );
}
