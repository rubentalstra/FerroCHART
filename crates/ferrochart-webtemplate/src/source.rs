// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Where a member the form definition does not model lives between a read and
//! a write.
//!
//! # Why the whole source object is kept, and not only the leftovers
//!
//! The obvious design keeps a map of the members the reader did not consume.
//! It loses two things. A member the form definition models is not only a
//! value but a spelling, and a web template omits a member rather than
//! stating it empty, so "absent" and "stated as zero" are different documents
//! that a leftovers map cannot tell apart. And a member nested inside one the
//! form definition models, a per-option terminology binding inside `inputs`
//! for instance, is not a leftover of the node and would be dropped.
//!
//! Keeping the source object answers both. The writer starts from it, so a
//! node the form definition did not change goes back out exactly as it came
//! in, and a node it did change keeps every member outside
//! [`crate::member::MODELLED_NODE_MEMBERS`].
//!
//! The `EHRbase` SDK preserves arbitrary template annotations, and a consumer
//! who put annotations in a web template and got them stripped would have lost
//! work, so this is the obligation `docs/architecture.md` section 4 records
//! for the compatibility surface. No specification governs it: our own design.

use std::collections::BTreeMap;

use ferrochart_form::key::NodeKey;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One node's source object, as the document spelled it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceNode {
    /// What names the node in the form definition.
    pub key: NodeKey,
    /// The node object as the source document spelled it, without `children`.
    ///
    /// `children` is left out because the tree the writer emits comes from the
    /// form definition: a child the form definition dropped must not come back
    /// from here.
    pub members: Map<String, Value>,
}

/// A web template document as its author spelled it, beside the form
/// definition read out of it.
///
/// Empty for a form definition that never came from a web template, in which
/// case the writer states every member itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SourceWire", into = "SourceWire")]
pub struct SourceSpelling {
    document: Map<String, Value>,
    nodes: BTreeMap<NodeKey, Map<String, Value>>,
}

/// How a [`SourceSpelling`] is stored, since a node key is not a JSON member
/// name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SourceWire {
    document: Map<String, Value>,
    nodes: Vec<SourceNode>,
}

impl From<SourceWire> for SourceSpelling {
    fn from(wire: SourceWire) -> Self {
        Self {
            document: wire.document,
            nodes: wire
                .nodes
                .into_iter()
                .map(|node| (node.key, node.members))
                .collect(),
        }
    }
}

impl From<SourceSpelling> for SourceWire {
    fn from(spelling: SourceSpelling) -> Self {
        Self {
            document: spelling.document,
            nodes: spelling
                .nodes
                .into_iter()
                .map(|(key, members)| SourceNode { key, members })
                .collect(),
        }
    }
}

impl SourceSpelling {
    /// The spelling of a form definition that never came from a web template.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Whether the form definition this sits beside came from a web template.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.document.is_empty() && self.nodes.is_empty()
    }

    /// The document root as the source spelled it, without `tree`.
    #[must_use]
    pub fn document(&self) -> &Map<String, Value> {
        &self.document
    }

    /// Records the document root, which the caller has already stripped of
    /// `tree`.
    pub fn set_document(&mut self, members: Map<String, Value>) {
        self.document = members;
    }

    /// The node at `key` as the source spelled it, without `children`.
    #[must_use]
    pub fn node(&self, key: &NodeKey) -> Option<&Map<String, Value>> {
        self.nodes.get(key)
    }

    /// Records the node at `key`, which the caller has already stripped of
    /// `children`.
    pub fn set_node(&mut self, key: NodeKey, members: Map<String, Value>) {
        self.nodes.insert(key, members);
    }

    /// Every node the source stated, in key order.
    pub fn nodes(&self) -> impl Iterator<Item = (&NodeKey, &Map<String, Value>)> {
        self.nodes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::SourceSpelling;
    use ferrochart_form::ids::{RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};
    use serde_json::json;

    fn key() -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new("items"),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    #[test]
    fn a_spelling_round_trips_through_json_with_its_node_keys_intact() {
        let mut spelling = SourceSpelling::none();
        assert!(spelling.is_empty());
        spelling.set_document(
            json!({"templateId": "t"})
                .as_object()
                .cloned()
                .expect("an object"),
        );
        spelling.set_node(
            key(),
            json!({"annotations": {"ui": "slider"}})
                .as_object()
                .cloned()
                .expect("an object"),
        );

        let text = serde_json::to_string(&spelling).expect("a spelling serializes");
        let back: SourceSpelling = serde_json::from_str(&text).expect("a spelling deserializes");
        assert_eq!(back, spelling);
        assert_eq!(
            back.node(&key()).and_then(|node| node.get("annotations")),
            Some(&json!({"ui": "slider"}))
        );
    }
}
