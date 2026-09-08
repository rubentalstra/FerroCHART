// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Sibling constraints that share every part of their identity.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.1 says a node id
//! "guarantees sibling node unique identification" and section 4.3.6 says it
//! is "used to dis-tinguish sibling nodes", but AOM 1.4 carries no invariant
//! and no validity condition that requires it, and the ITS-XML schemas carry
//! no `xs:unique`, `xs:key` or `xs:keyref`. ADL 2 does forbid it, in
//! `AOM2.html` section 4.5.4.3 rule VCOSU. So a real ADL 1.4 operational
//! template can carry the shape, and these are what the derivation does with
//! it.

use std::collections::BTreeMap;
use std::fs;

use ferrochart_compile::derive::error::DeriveError;
use ferrochart_compile::{adl14, derive};
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::FieldKind;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::occurrences::Occurrences;

use crate::support::templates;

const TIED: &str = include_str!("../fixtures/ferro_tied_siblings.opt");
const DIFFER: &str = include_str!("../fixtures/ferro_tied_siblings_differ.opt");
const ALTERNATIVES: &str = include_str!("../fixtures/ferro_tied_alternatives.opt");

fn derived(xml: &str) -> FormDefinition {
    let template = adl14::from_xml(xml).expect("the fixture reads");
    derive::form(&template).expect("the fixture derives")
}

/// The key of one item, where the item is a shape this test knows.
///
/// NOTE: `FormItem` is `#[non_exhaustive]`, so a variant added later reaches
/// the wildcard rather than failing this build; it is skipped, not counted.
fn key_of(item: &FormItem) -> Option<&NodeKey> {
    match *item {
        FormItem::Group(ref group) => Some(&group.key),
        FormItem::Field(ref field) => Some(&field.key),
        _ => None,
    }
}

/// The terminal node id of one item's key.
fn node_id(key: &NodeKey) -> Option<&str> {
    key.terminal()
        .and_then(|step| step.node_id.as_ref())
        .map(ferrochart_form::ids::LocalCode::as_str)
}

/// Every item of `group` whose terminal key step carries `code`.
fn items<'a>(group: &'a FormGroup, code: &str) -> Vec<&'a FormItem> {
    group
        .items
        .iter()
        .filter(|item| key_of(item).is_some_and(|key| node_id(key) == Some(code)))
        .collect()
}

#[test]
fn a_value_stated_twice_under_a_single_valued_attribute_is_one_field() {
    // openEHR AM Release-2.3.0 AOM1.4.html section 4.3.2 calls the children of
    // a singular attribute "alternatives", and section 4.3.3 says "the meaning
    // of the inherited children attribute is that they are alternatives", with
    // `Members_valid` capping each member at one occurrence. Two alternatives
    // that state the same constraint are one alternative, so the field is a
    // text field rather than a choice between two identical texts.
    let form = derived(TIED);
    let held = items(&form.root, "at0002");
    assert_eq!(held.len(), 1, "at0002 derived more than one item");
    let FormItem::Field(ref field) = *held.first().copied().expect("one item") else {
        panic!("at0002 is not a field");
    };
    let FieldKind::Text(ref text) = field.kind else {
        panic!("at0002 is not a text field, it is {}", field.kind.name());
    };
    assert_eq!(text.options, vec!["two".to_owned()]);
    assert_eq!(field.occurrences, Occurrences::bounded(0, 1));
}

#[test]
fn identical_members_under_a_container_become_one_node_with_the_collective_count() {
    // openEHR AM Release-2.3.0 ADL1.4.html section 5.3.4.2 rule VCOC reads a
    // sibling set additively: the interval it represents runs from the sum of
    // the occurrences minima to the sum of the occurrences maxima, and must
    // sit inside the containing attribute's cardinality. The fold is lossless
    // here only because the members state the same constraint.
    let form = derived(TIED);

    // Two `0..1` members under a `0..*` container.
    let held = items(&form.root, "at0001");
    assert_eq!(held.len(), 1, "at0001 derived more than one item");
    let FormItem::Field(ref field) = *held.first().copied().expect("one item") else {
        panic!("at0001 is not a field");
    };
    assert_eq!(field.occurrences, Occurrences::bounded(0, 2));

    // Two `1..1` members, so the lower bounds sum too.
    let held = items(&form.root, "at0003");
    assert_eq!(held.len(), 1, "at0003 derived more than one item");
    let FormItem::Group(ref group) = *held.first().copied().expect("one item") else {
        panic!("at0003 is not a group");
    };
    assert_eq!(group.occurrences, Occurrences::bounded(2, 2));
    assert_eq!(
        group.items.len(),
        1,
        "the group's own pair did not collapse"
    );

    // Two `0..1` members whose container holds at most one, so the container
    // cardinality is what caps the sum.
    let held = items(&form.root, "at0005");
    let FormItem::Group(ref capped) = *held.first().copied().expect("at0005 derives") else {
        panic!("at0005 is not a group");
    };
    let held = items(capped, "at0006");
    assert_eq!(held.len(), 1, "at0006 derived more than one item");
    let FormItem::Field(ref field) = *held.first().copied().expect("one item") else {
        panic!("at0006 is not a field");
    };
    assert_eq!(field.occurrences, Occurrences::bounded(0, 1));
}

#[test]
fn tied_members_under_a_container_that_differ_are_refused_by_name() {
    // openEHR RM Release-1.1.0 common.html section 3.2.2 gives a non-root node
    // only its `archetype_node_id`; `LOCATABLE.name` is the only other
    // identity attribute and the members share it; and section 3.1.2.1 says
    // `uid` "will usually be empty". So no instance could say which member it
    // satisfies, and a form offering both could build a document whose nodes
    // cannot be attributed to the constraint they answer.
    let template = adl14::from_xml(DIFFER).expect("the fixture reads");
    let error = derive::form(&template).expect_err("the fixture is refused");
    let DeriveError::UnattributableSiblings {
        ref template,
        ref path,
        ref node,
        ref difference,
    } = error
    else {
        panic!("the refusal is {error}");
    };
    assert_eq!(
        template,
        "openEHR-EHR-CLUSTER.ferro_tied_siblings_differ.v1"
    );
    assert_eq!(
        path,
        "/[openEHR-EHR-CLUSTER.ferro_tied_siblings_differ.v1, CLUSTER]"
    );
    assert_eq!(node, "/items[at0001, ELEMENT]");
    assert!(
        difference.contains("text"),
        "the refusal does not name what the members differ on: {difference}"
    );
}

#[test]
fn tied_alternatives_under_a_single_valued_attribute_that_differ_both_survive() {
    // The case the two rules above leave standing. openEHR AM Release-2.3.0
    // AOM1.4.html sections 4.3.2 and 4.3.3 make the children of a
    // single-valued attribute alternatives, so two that state different
    // constraints are two things the template offers rather than one node
    // stated twice, and the fold would lose whichever it dropped. A key can
    // separate them only by position, so both are marked positional and a
    // replay reports them rather than trusting the position
    // (`docs/architecture.md` section 6.2).
    let form = derived(ALTERNATIVES);
    let held = items(&form.root, "at0005");
    assert_eq!(
        held.len(),
        2,
        "the differing alternatives did not both derive"
    );
    for item in held {
        let FormItem::Group(ref group) = *item else {
            panic!("at0005 is not a group");
        };
        assert!(
            group.key.is_positional,
            "a tied alternative is keyed as though its identity separated it"
        );
    }
}

#[test]
fn no_tied_sibling_group_survives_the_derivation_of_the_committed_pack() {
    // The corpus-wide statement of the three branches above. A form is a
    // projection of its template (`CLAUDE.md`), so two sibling items that
    // share every identifying part would be two fields one document cannot
    // fill without writing a node no reader can attribute.
    let mut checked = 0_usize;
    for path in templates() {
        let xml = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        let Ok(template) = adl14::from_xml(&xml) else {
            continue;
        };
        let form = derive::form(&template).expect("every template derives");
        checked = checked.saturating_add(1);
        assert_untied(&form.root, &path.display().to_string());
    }
    assert!(checked >= 100, "only {checked} templates were checked");
}

/// Fails where two of `group`'s items are separated by nothing but position.
fn assert_untied(group: &FormGroup, source: &str) {
    let mut seen: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for item in &group.items {
        let Some(step) = key_of(item).and_then(NodeKey::terminal) else {
            continue;
        };
        *seen.entry(discriminator(step)).or_default() += 1;
    }
    for (tuple, count) in &seen {
        assert_eq!(
            *count, 1,
            "{source}: {} sibling items under {} share {tuple:?}",
            count, group.key
        );
    }
    for item in &group.items {
        if let FormItem::Group(ref child) = *item {
            assert_untied(child, source);
        }
    }
}

/// The five parts of `docs/architecture.md` section 6.2, without the ordinal.
fn discriminator(step: &KeyStep) -> Vec<String> {
    vec![
        step.rm_attribute.as_str().to_owned(),
        step.node_id
            .as_ref()
            .map(|code| code.as_str().to_owned())
            .unwrap_or_default(),
        step.archetype_id
            .as_ref()
            .map(|id| id.as_str().to_owned())
            .unwrap_or_default(),
        step.rm_type.as_str().to_owned(),
        step.pinned_name.clone().unwrap_or_default(),
    ]
}
