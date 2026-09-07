// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What names one item of a form definition.
//!
//! No specification governs this shape; it is FerroCHART's own design, and it
//! was measured rather than assumed. openEHR BASE Release-1.2.0
//! `architecture_overview.html` section 11.2.2.2 guarantees uniqueness of an
//! archetype path only *in an archetype*, and an operational template composes
//! many, so an id-only path is not unique inside one. The five parts of a step
//! are what separates a colliding sibling group; the sibling ordinal is the
//! last resort where the whole tuple still ties, and a key that leans on it is
//! marked so a later replay can report it rather than trust it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{ArchetypeId, LocalCode, RmAttributeName, RmTypeName};

/// One step of a [`NodeKey`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyStep {
    /// The Reference Model attribute the node sits under.
    pub rm_attribute: RmAttributeName,
    /// The node's local code, where the template gives it one.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.1 lets a leaf with
    /// no siblings carry no node id at all, so this is genuinely optional.
    pub node_id: Option<LocalCode>,
    /// The archetype this node is the root of, where it is one.
    pub archetype_id: Option<ArchetypeId>,
    /// The Reference Model class of the node this step names.
    ///
    /// This is the class the node IS, which for a leaf is `ELEMENT` rather
    /// than the data value it holds. A field's own `rm_type` is the class it
    /// COLLECTS, so the two differ on every field derived from an `ELEMENT`
    /// (openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3).
    pub rm_type: RmTypeName,
    /// The name the template pins on the node, where it states one.
    ///
    /// This is a definition fact the template states, and not a predicate
    /// matched against data.
    pub pinned_name: Option<String>,
    /// The node's position among its siblings under the same attribute,
    /// counting from zero.
    pub sibling_ordinal: usize,
}

impl fmt::Display for KeyStep {
    /// Writes the step so that two steps which do not match never read alike.
    ///
    /// The shape follows the AQL node predicate, which spells an identifier
    /// and a name together as `[at0001, 'Systolic']` (openEHR QUERY
    /// Release-1.1.0 section 3.6.3). Every part that decides a match is
    /// printed, because a person reading a replay report is being asked to act
    /// on the difference between two keys: the identifier, the Reference Model
    /// class, the pinned name where the template states one, and the ordinal
    /// where the step is not the first of its tied siblings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.rm_attribute)?;
        f.write_str("[")?;
        match (self.archetype_id.as_ref(), self.node_id.as_ref()) {
            (Some(archetype), _) => write!(f, "{archetype}, ")?,
            (None, Some(node)) => write!(f, "{node}, ")?,
            (None, None) => {}
        }
        write!(f, "{}", self.rm_type)?;
        if let Some(name) = self.pinned_name.as_deref() {
            write!(f, ", '{name}'")?;
        }
        if self.sibling_ordinal > 0 {
            write!(f, ", #{}", self.sibling_ordinal)?;
        }
        f.write_str("]")
    }
}

/// What names one group, field or piece of undetermined content.
///
/// The key is the chain of steps from the template root to the item. It is
/// what a layout overlay attaches to, and what a validation result is keyed
/// by.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeKey {
    /// The steps, from the template root to the item.
    pub steps: Vec<KeyStep>,
    /// Whether any step of the key is told from its siblings only by its
    /// position.
    ///
    /// A positional key is the one a reordering silently breaks, so it is
    /// marked here rather than trusted.
    pub is_positional: bool,
}

impl NodeKey {
    /// The key of the template root, which is the empty chain.
    #[must_use]
    pub fn root() -> Self {
        Self {
            steps: Vec::new(),
            is_positional: false,
        }
    }

    /// This key with `step` appended.
    #[must_use]
    pub fn child(&self, step: KeyStep) -> Self {
        let mut steps = self.steps.clone();
        steps.push(step);
        Self {
            steps,
            is_positional: self.is_positional,
        }
    }

    /// The last step, which names the item itself.
    #[must_use]
    pub fn terminal(&self) -> Option<&KeyStep> {
        self.steps.last()
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.steps.is_empty() {
            return f.write_str("/");
        }
        for step in &self.steps {
            write!(f, "{step}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyStep, NodeKey};
    use crate::ids::{ArchetypeId, LocalCode, RmAttributeName, RmTypeName};

    fn step(attribute: &str, node_id: Option<&str>, rm_type: &str) -> KeyStep {
        KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: node_id.map(LocalCode::new),
            archetype_id: None,
            rm_type: RmTypeName::new(rm_type),
            pinned_name: None,
            sibling_ordinal: 0,
        }
    }

    #[test]
    fn a_key_reads_as_the_path_a_person_can_find_in_the_template() {
        let key = NodeKey::root()
            .child(step("data", Some("at0001"), "ITEM_TREE"))
            .child(step("items", Some("at0002"), "ELEMENT"));
        // Every part that decides a match is printed (#83), so a person
        // acting on a replay report sees the difference they are asked about.
        assert_eq!(
            key.to_string(),
            "/data[at0001, ITEM_TREE]/items[at0002, ELEMENT]"
        );
    }

    #[test]
    fn an_archetype_root_step_carries_the_archetype_id_as_its_predicate() {
        let mut root = step("content", Some("at0000"), "OBSERVATION");
        root.archetype_id = Some(ArchetypeId::new("openEHR-EHR-OBSERVATION.ferro_test.v1"));
        assert_eq!(
            NodeKey::root().child(root).to_string(),
            "/content[openEHR-EHR-OBSERVATION.ferro_test.v1, OBSERVATION]"
        );
    }

    #[test]
    fn a_pinned_name_and_an_ordinal_reach_the_printed_key() {
        // The two parts the old format dropped. A name separates 41.0% of
        // colliding siblings and an ordinal is all that separates the 7.0%
        // nothing else can (docs/architecture.md section 6.2).
        let mut named = step("items", Some("at0001"), "CLUSTER");
        named.pinned_name = Some("Systolic".to_owned());
        assert_eq!(
            NodeKey::root().child(named.clone()).to_string(),
            "/items[at0001, CLUSTER, 'Systolic']"
        );

        let mut second = named;
        second.sibling_ordinal = 1;
        assert_eq!(
            NodeKey::root().child(second).to_string(),
            "/items[at0001, CLUSTER, 'Systolic', #1]"
        );
    }

    #[test]
    fn the_root_key_is_the_empty_chain() {
        assert_eq!(NodeKey::root().to_string(), "/");
        assert!(NodeKey::root().terminal().is_none());
    }
}
