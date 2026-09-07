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
    /// The Reference Model type the node constrains.
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.rm_attribute)?;
        match (self.archetype_id.as_ref(), self.node_id.as_ref()) {
            (Some(archetype), _) => write!(f, "[{archetype}]"),
            (None, Some(node)) => write!(f, "[{node}]"),
            (None, None) => Ok(()),
        }
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
        assert_eq!(key.to_string(), "/data[at0001]/items[at0002]");
    }

    #[test]
    fn an_archetype_root_step_carries_the_archetype_id_as_its_predicate() {
        let mut root = step("content", Some("at0000"), "OBSERVATION");
        root.archetype_id = Some(ArchetypeId::new("openEHR-EHR-OBSERVATION.ferro_test.v1"));
        assert_eq!(
            NodeKey::root().child(root).to_string(),
            "/content[openEHR-EHR-OBSERVATION.ferro_test.v1]"
        );
    }

    #[test]
    fn the_root_key_is_the_empty_chain() {
        assert_eq!(NodeKey::root().to_string(), "/");
        assert!(NodeKey::root().terminal().is_none());
    }
}
