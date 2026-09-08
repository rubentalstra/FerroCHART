// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Sibling constraints an instance cannot be told apart.
//!
//! An operational template can carry two sibling constraints that share the
//! Reference Model attribute, the node id, the archetype id, the Reference
//! Model type and the pinned name. openEHR AM Release-2.3.0 `AOM1.4.html`
//! section 4.2.3.1 says a node id "guarantees sibling node unique
//! identification" and section 4.3.6 says it is "used to dis-tinguish sibling
//! nodes", but AOM 1.4 states no invariant and no validity condition that
//! requires it: `ARCHETYPE.node_ids_valid()` (section 3.2.1) checks
//! terminology membership only, and neither `C_OBJECT` (section 4.3.6) nor
//! `C_MULTIPLE_ATTRIBUTE` (section 4.3.4) carries an invariant block at all.
//! The ITS-XML schemas carry no identity constraint either: there is no
//! `xs:unique`, `xs:key` or `xs:keyref` in `Template.xsd`,
//! `OpenehrProfile.xsd` or `Archetype.xsd`, in either generation. ADL 2 does
//! forbid it, in `AOM2.html` section 4.5.4.3 rule VCOSU: "object node
//! identifier validity: every object node must be unique within the
//! archetype."
//!
//! A COMPOSITION cannot separate such siblings. openEHR RM Release-1.1.0
//! `common.html` section 3.2.2 gives a non-root node only its
//! `archetype_node_id`; `LOCATABLE.name` is the only other identity attribute
//! and the tied members share it; and `uid` is 0..1 with section 3.1.2.1
//! saying it "will usually be empty". So a form that offered one field per
//! member could build a document whose nodes no reader could attribute to the
//! constraint they satisfy.
//!
//! Members that state the same constraint fold into one wherever they tie,
//! which is lossless by construction. Members that differ are refused only
//! where the identity argument above reaches them: under a container
//! attribute, on a node the template gives an `archetype_node_id`. Under a
//! single-valued attribute they are alternatives (`AOM1.4.html` sections
//! 4.3.2 and 4.3.3), and a node with no node id carries no
//! `archetype_node_id` to tie on.
//!
//! This pass runs before any key is computed, which is what keeps the overlay
//! key stable against the very defect that creates the collision: the sibling
//! ordinal of `docs/architecture.md` section 6.2 is the last-resort
//! discriminator, and a duplicated group shifts the ordinal of every sibling
//! after it. Collapsing first, then renumbering, gives the key the template
//! would have produced without the duplicate.

use std::collections::BTreeMap;

use ferrochart_form::key::{KeyStep, NodeKey};

use crate::derive::error::DeriveError;
use crate::model::multiplicity::{Cardinality, Multiplicity};
use crate::model::node::{ConstraintNode, ConstraintTemplate, NodeIdentity};

/// Collapses every tied sibling group of `template`.
///
/// # Errors
/// [`DeriveError::UnattributableSiblings`] where tied members under a
/// container attribute state different constraints, which is the case no
/// reading of the specifications settles and no instance could resolve.
pub(crate) fn tied_siblings(
    template: &ConstraintTemplate,
) -> Result<ConstraintTemplate, DeriveError> {
    let mut collapsed = template.clone();
    let id = template.id().as_str().to_owned();
    let root_path = NodeKey::root().child(step(collapsed.root().identity()));
    node(collapsed.root_mut(), &id, &root_path)?;
    Ok(collapsed)
}

/// One key step, without the ordinal, so a diagnostic path reads the same
/// before and after the collapse renumbers the siblings.
fn step(identity: &NodeIdentity) -> KeyStep {
    let mut step = super::key_step(identity);
    step.sibling_ordinal = 0;
    step
}

/// Collapses the children of one node, deepest first.
fn node(node: &mut ConstraintNode, template: &str, path: &NodeKey) -> Result<(), DeriveError> {
    let children = std::mem::take(node.children_mut());
    let mut deepened = Vec::with_capacity(children.len());
    for mut child in children {
        let child_path = path.child(step(child.identity()));
        self::node(&mut child, template, &child_path)?;
        deepened.push(child);
    }
    *node.children_mut() = fold(deepened, template, path)?;
    Ok(())
}

/// Folds every tied group among one node's children into what a form may
/// offer.
fn fold(
    children: Vec<ConstraintNode>,
    template: &str,
    container: &NodeKey,
) -> Result<Vec<ConstraintNode>, DeriveError> {
    let mut kept: Vec<ConstraintNode> = Vec::with_capacity(children.len());
    for child in children {
        let mut absorbed = false;
        for held in &mut kept {
            if !held.identity().discriminates_like(child.identity()) {
                continue;
            }
            match difference(held, &child) {
                None => {
                    absorb(held, &child);
                    absorbed = true;
                }
                Some(difference) if unattributable(held) => {
                    return Err(DeriveError::UnattributableSiblings {
                        template: template.to_owned(),
                        path: container.to_string(),
                        node: step(child.identity()).to_string(),
                        difference,
                    });
                }
                // openEHR AM Release-2.3.0 `AOM1.4.html` sections 4.3.2 and
                // 4.3.3 make the children of a single-valued attribute
                // alternatives, so members that differ stay separate and an
                // instance satisfying any one of them is valid.
                Some(_) => {}
            }
            break;
        }
        if !absorbed {
            kept.push(child);
        }
    }
    renumber(&mut kept);
    Ok(kept)
}

/// Whether an instance could be told which of two tied members it satisfies.
///
/// The identity a document carries is `LOCATABLE.archetype_node_id` (openEHR
/// RM Release-1.1.0 `common.html` section 3.2.2), so the question only arises
/// where the template gives the node one and the container may hold both. A
/// `REFERENCE_RANGE` (`data_types.html` section 6.2.3) is not a `LOCATABLE`
/// and carries no node id, so its members are told apart by their own content
/// and stay as the template states them.
fn unattributable(node: &ConstraintNode) -> bool {
    node.attribute().is_container()
        && (node.identity().node_id().is_some() || node.identity().archetype_id().is_some())
}

/// Takes one tied member into the member that already stands for the group.
///
/// The two state the same constraint, so one node with the collective
/// occurrences admits exactly the instances the pair did.
fn absorb(held: &mut ConstraintNode, member: &ConstraintNode) {
    if !held.attribute().is_container() {
        // Under a single-valued attribute the members are alternatives, and
        // `AOM1.4.html` section 4.3.3 `Members_valid` caps each at one
        // occurrence, so there is no count to add up.
        return;
    }
    held.set_occurrences(collective(
        held.occurrences(),
        member.occurrences(),
        held.attribute().cardinality(),
    ));
}

/// The occurrences of a tied group, taken together.
///
/// NOTE: openEHR AM Release-2.3.0 `ADL1.4.html` section 5.3.4.2 rule VCOC
/// reads a sibling set additively, "the sum of all occurrences minimum
/// values .. the sum of all occurrences maximum values", and AOM 2 section
/// 4.5.4.3 rule VSONCO does the same for a specialised node set.
fn collective(
    left: Multiplicity,
    right: Multiplicity,
    cardinality: Option<Cardinality>,
) -> Multiplicity {
    let lower = left.lower().saturating_add(right.lower());
    let summed = match (left.upper(), right.upper()) {
        (Some(left), Some(right)) => Some(left.saturating_add(right)),
        _ => None,
    };
    let container = cardinality.and_then(|cardinality| cardinality.interval().upper());
    match (summed, container) {
        (None, None) => Multiplicity::unbounded_from(lower),
        (Some(upper), None) | (None, Some(upper)) => Multiplicity::bounded(lower, upper),
        (Some(summed), Some(container)) => Multiplicity::bounded(lower, summed.min(container)),
    }
}

/// Numbers each child by its position among the siblings under its own
/// Reference Model attribute, which is what both readers count.
fn renumber(children: &mut [ConstraintNode]) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for child in children.iter_mut() {
        let attribute = child.identity().rm_attribute().as_str().to_owned();
        let next = seen.entry(attribute).or_default();
        child.identity_mut().set_sibling_ordinal(*next);
        *next = next.saturating_add(1);
    }
}

/// The first fact two tied members state differently, or `None` where they
/// state the same constraint.
///
/// The sibling ordinal is left out at every level: it is the position this
/// pass is about to rewrite, and it says nothing about what a node admits.
fn difference(left: &ConstraintNode, right: &ConstraintNode) -> Option<String> {
    difference_at("", left, right)
}

/// The first difference between two nodes, named with the path below the
/// tied node where the difference is not on the node itself.
fn difference_at(path: &str, left: &ConstraintNode, right: &ConstraintNode) -> Option<String> {
    if !left.identity().discriminates_like(right.identity()) {
        return Some(say(path, "the node identity".to_owned()));
    }
    if left.occurrences() != right.occurrences() {
        return Some(say(
            path,
            format!(
                "the occurrences {} and {}",
                left.occurrences(),
                right.occurrences()
            ),
        ));
    }
    if left.occurrences_stated() != right.occurrences_stated() {
        return Some(say(path, "whether the occurrences are stated".to_owned()));
    }
    if left.attribute() != right.attribute() {
        return Some(say(path, "the containing attribute".to_owned()));
    }
    if left.payload() != right.payload() {
        return Some(say(
            path,
            format!(
                "a {} constraint and a {} constraint",
                left.payload().kind(),
                right.payload().kind()
            ),
        ));
    }
    if left.terminology_scope() != right.terminology_scope() {
        return Some(say(
            path,
            format!(
                "the terminology of {} and of {}",
                left.terminology_scope(),
                right.terminology_scope()
            ),
        ));
    }
    if left.is_deprecated() != right.is_deprecated() {
        return Some(say(path, "whether the node is deprecated".to_owned()));
    }
    if left.default_value() != right.default_value() {
        return Some(say(path, "the default value".to_owned()));
    }
    if left.children().len() != right.children().len() {
        return Some(say(
            path,
            format!(
                "{} children and {} children",
                left.children().len(),
                right.children().len()
            ),
        ));
    }
    for (left, right) in left.children().iter().zip(right.children()) {
        let below = format!("{path}{}", step(left.identity()));
        if let Some(difference) = difference_at(&below, left, right) {
            return Some(difference);
        }
    }
    None
}

/// Names a difference, with the path below the tied node where there is one.
fn say(path: &str, what: String) -> String {
    if path.is_empty() {
        what
    } else {
        format!("{what}, at {path}")
    }
}

#[cfg(test)]
mod tests {
    use super::collective;
    use crate::model::multiplicity::{Cardinality, Multiplicity};

    fn card(interval: Multiplicity) -> Cardinality {
        Cardinality::new(interval, false, false)
    }

    #[test]
    fn two_optional_members_of_an_open_container_become_one_that_admits_two() {
        // The imaging shape: two `0..1` members under a `1..*` container.
        assert_eq!(
            collective(
                Multiplicity::bounded(0, 1),
                Multiplicity::bounded(0, 1),
                Some(card(Multiplicity::unbounded_from(1))),
            ),
            Multiplicity::bounded(0, 2)
        );
    }

    #[test]
    fn the_container_cardinality_caps_the_sum() {
        assert_eq!(
            collective(
                Multiplicity::bounded(0, 3),
                Multiplicity::bounded(0, 3),
                Some(card(Multiplicity::bounded(0, 4))),
            ),
            Multiplicity::bounded(0, 4)
        );
    }

    #[test]
    fn an_unbounded_member_leaves_the_sum_unbounded_until_the_container_caps_it() {
        assert_eq!(
            collective(
                Multiplicity::unbounded_from(1),
                Multiplicity::bounded(1, 1),
                None,
            ),
            Multiplicity::unbounded_from(2)
        );
        assert_eq!(
            collective(
                Multiplicity::unbounded_from(1),
                Multiplicity::bounded(1, 1),
                Some(card(Multiplicity::bounded(0, 5))),
            ),
            Multiplicity::bounded(2, 5)
        );
    }
}
