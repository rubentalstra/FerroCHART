// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The form definition, read as a path index.
//!
//! Every group and field of a form definition carries a
//! [`ferrochart_form::key::NodeKey`], and every step of that key carries the
//! Reference Model attribute and the identifier that would appear in a path
//! predicate. So the index is derivable from the published contract alone,
//! with no second walk of the operational template.
//!
//! No specification governs this: our own design.

use std::collections::BTreeMap;

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::key::{KeyStep, NodeKey};

/// Every item of a form definition, addressable by path.
#[derive(Debug, Default)]
pub struct KeyIndex {
    /// The keys at each path, in definition order.
    ///
    /// A path can name more than one key. The key drops the Reference Model
    /// type, the pinned name and the sibling ordinal, which are three of the
    /// five discriminators `docs/architecture.md` section 6.2 measured as
    /// load-bearing, so two siblings that a `NodeKey` separates can share a
    /// path. An ambiguous path resolves to nothing rather than to a guess.
    at: BTreeMap<String, Vec<NodeKey>>,
}

impl KeyIndex {
    /// Indexes every group and field of `definition`.
    ///
    /// `prefix` is prepended to every path, for a template whose root sits
    /// below the composition root. It is the path from the composition to
    /// that node, and it is empty for a template rooted at COMPOSITION.
    #[must_use]
    pub fn of(definition: &FormDefinition, prefix: &str) -> Self {
        let mut index = Self::default();
        for group in definition.groups() {
            index.insert(prefix, &group.key);
        }
        for field in definition.fields() {
            index.insert(prefix, &field.key);
        }
        for undetermined in definition.undetermined() {
            index.insert(prefix, &undetermined.key);
        }
        index
    }

    fn insert(&mut self, prefix: &str, key: &NodeKey) {
        let path = format!("{prefix}{}", relative(key));
        self.at.entry(path).or_default().push(key.clone());
    }

    /// The key `path` names, or the key of the nearest ancestor that one
    /// names.
    ///
    /// A validation message is keyed by the constraining node's path, and
    /// three shapes of that path name no item of the form: an attribute
    /// (`…/items`, which a cardinality violation is reported on), the
    /// attribute holding a leaf's value (`…/value`), and a Reference Model
    /// node the derivation folded away. Walking up puts each of those on the
    /// nearest item a renderer can show, which is where a clinician would
    /// look for it.
    #[must_use]
    pub fn resolve(&self, path: &str) -> Option<&NodeKey> {
        let mut remaining = path;
        loop {
            if let Some(found) = self.exactly(remaining) {
                return Some(found);
            }
            let cut = remaining.rfind('/')?;
            remaining = remaining.get(..cut)?;
            if remaining.is_empty() {
                return None;
            }
        }
    }

    /// The single key at exactly `path`, where the path names one.
    #[must_use]
    pub fn exactly(&self, path: &str) -> Option<&NodeKey> {
        match self.at.get(path)?.as_slice() {
            [only] => Some(only),
            // Two items at one path is the collision measured over 102
            // templates (`docs/architecture.md` section 6.2). Naming one of
            // them would put a clinician's error on the wrong field.
            _ => None,
        }
    }

    /// How many distinct paths the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.at.len()
    }

    /// Whether the index holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.at.is_empty()
    }
}

/// The path of `key` below the template root.
///
/// The first step of a key names the root itself and carries no attribute, so
/// it contributes no path step: the web template's own paths start below the
/// root too (`openehr_its::flat::webtemplate::builder`, which returns an empty
/// path for the root and appends `/attribute[node]` per step below it).
fn relative(key: &NodeKey) -> String {
    let mut out = String::new();
    for step in key.steps.iter().skip(1) {
        out.push('/');
        out.push_str(step.rm_attribute.as_str());
        if let Some(identifier) = identifier(step) {
            out.push('[');
            out.push_str(identifier);
            out.push(']');
        }
    }
    out
}

/// The identifier a step contributes to a path predicate.
///
/// openEHR RM Release-1.1.0 `common.html` section 3.2.2: at an archetype root
/// a node's `archetype_node_id` is the archetype identifier in string form,
/// and it is the node's own code everywhere else. The web template builds its
/// predicate from that same value, so the archetype identifier wins here too.
fn identifier(step: &KeyStep) -> Option<&str> {
    step.archetype_id
        .as_ref()
        .map(ferrochart_form::ids::ArchetypeId::as_str)
        .or_else(|| {
            step.node_id
                .as_ref()
                .map(ferrochart_form::ids::LocalCode::as_str)
        })
}

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{ArchetypeId, LocalCode, RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};

    use super::{KeyIndex, relative};

    fn step(attribute: &str, node_id: Option<&str>, archetype: Option<&str>) -> KeyStep {
        KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: node_id.map(LocalCode::new),
            archetype_id: archetype.map(ArchetypeId::new),
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: Some("a pinned name".to_owned()),
            sibling_ordinal: 0,
        }
    }

    #[test]
    fn the_root_step_contributes_nothing_and_an_archetype_id_wins_over_a_node_id() {
        let key = NodeKey::root()
            .child(step("", None, Some("openEHR-EHR-OBSERVATION.x.v1")))
            .child(step("data", Some("at0001"), None))
            .child(step(
                "items",
                Some("at0002"),
                Some("openEHR-EHR-CLUSTER.y.v1"),
            ));
        assert_eq!(
            relative(&key),
            "/data[at0001]/items[openEHR-EHR-CLUSTER.y.v1]"
        );
    }

    #[test]
    fn a_path_naming_two_items_resolves_to_neither() {
        let mut index = KeyIndex::default();
        let key = NodeKey::root()
            .child(step("", None, Some("openEHR-EHR-OBSERVATION.x.v1")))
            .child(step("items", Some("at0002"), None));
        index.insert("", &key);
        index.insert("", &key);
        assert!(index.exactly("/items[at0002]").is_none());
    }

    #[test]
    fn a_path_below_an_item_resolves_to_that_item() {
        let mut index = KeyIndex::default();
        let key = NodeKey::root()
            .child(step("", None, Some("openEHR-EHR-OBSERVATION.x.v1")))
            .child(step("items", Some("at0002"), None));
        index.insert("", &key);
        // What the archetype-conformance pass reports on a leaf: the path of
        // the attribute holding the value, one step below the ELEMENT.
        assert_eq!(index.resolve("/items[at0002]/value"), Some(&key));
        assert!(index.resolve("/protocol[at0009]").is_none());
    }
}
