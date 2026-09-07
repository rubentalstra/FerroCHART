// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a person authored, which no specification governs.
//!
//! openEHR publishes no form artefact. openEHR AM Release-2.3.0 `AOM2.html`
//! section 3.5.1 scopes archetype annotations to documentation, and
//! `OPT2.html` section 3.3 permits a generator to delete the annotations
//! section outright, so none of the places a template could carry layout is
//! governed by specification prose. Everything in this module is therefore
//! FerroCHART's own design.
//!
//! A layout decorates one node of a form definition and never replaces it.
//! The definition says what the template admits; the layout says how a person
//! wants it presented, so nothing here can make a form admit what the
//! template refuses.

use std::fmt;

use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;
use ferrochart_form::value::Prefill;
use serde::{Deserialize, Serialize};

/// What names one authored section.
///
/// No specification governs this: our own design. A section is the person's
/// own grouping, so its identifier is theirs too and FerroCHART never mints
/// or rewrites one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SectionId(String);

impl SectionId {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The identifier as the person stated it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name a renderer knows one control by.
///
/// No specification governs this: our own design. FerroCHART never interprets
/// the name, because the set of controls belongs to the renderer rather than
/// to the form definition, and a closed list here would refuse a control a
/// third-party renderer has.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WidgetName(String);

impl WidgetName {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The name as the person stated it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WidgetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One grouping a person authored.
///
/// No specification governs this: our own design. The Reference Model tree is
/// the only grouping a template states, and a form author routinely wants
/// another one, so a section lives in the overlay and the items that belong
/// to it point at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// What names the section.
    pub id: SectionId,
    /// The section this one sits inside, where a person nested it.
    pub parent: Option<SectionId>,
    /// The heading the section is shown under.
    pub label: Localized,
    /// The longer text shown beneath the heading.
    pub help: Localized,
    /// Where the section sits among its siblings, smallest first.
    pub order: Option<u32>,
}

impl Section {
    /// A top-level section with a heading and nothing else decided.
    #[must_use]
    pub fn new(id: SectionId, label: Localized) -> Self {
        Self {
            id,
            parent: None,
            label,
            help: Localized::empty(),
            order: None,
        }
    }
}

/// When an item is shown.
///
/// No specification governs this: our own design. ADL 1.4 section 8.5 and the
/// AOM 2 Rules package evaluate an assertion after entry rather than deciding
/// what a person sees, so a visibility rule is authored rather than derived.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "visibility", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Visibility {
    /// Always, which is what an item with no rule does.
    #[default]
    Always,
    /// Never: the item is entered by nobody and stays in the form so a
    /// composition builder still knows the node is there.
    Never,
    /// Only while the condition holds.
    When {
        /// The condition that decides it.
        condition: Condition,
    },
}

/// What decides whether an item is shown.
///
/// No specification governs this: our own design. The set is deliberately
/// small and closed, because a condition FerroCHART cannot evaluate is a
/// condition a renderer cannot honour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "test", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Condition {
    /// The node carries a value.
    Answered {
        /// The node whose answer decides it.
        node: NodeKey,
    },
    /// The node carries no value.
    NotAnswered {
        /// The node whose answer decides it.
        node: NodeKey,
    },
    /// The node carries exactly this value.
    Equals {
        /// The node whose answer decides it.
        node: NodeKey,
        /// The value the answer is compared against.
        value: Prefill,
    },
    /// The node carries anything but this value.
    NotEquals {
        /// The node whose answer decides it.
        node: NodeKey,
        /// The value the answer is compared against.
        value: Prefill,
    },
    /// Every one of these conditions holds.
    All {
        /// The conditions, in authored order.
        conditions: Vec<Condition>,
    },
    /// At least one of these conditions holds.
    Any {
        /// The conditions, in authored order.
        conditions: Vec<Condition>,
    },
}

impl Condition {
    /// Every node this condition reads, in the order the condition names them.
    ///
    /// A replay classifies these beside the entry itself, because a rule that
    /// reads a node the revision removed cannot be evaluated and the person
    /// has to be told.
    #[must_use]
    pub fn nodes(&self) -> Vec<&NodeKey> {
        let mut found = Vec::new();
        let mut pending = vec![self];
        while let Some(condition) = pending.pop() {
            match *condition {
                Self::Answered { ref node }
                | Self::NotAnswered { ref node }
                | Self::Equals { ref node, .. }
                | Self::NotEquals { ref node, .. } => found.push(node),
                Self::All { ref conditions } | Self::Any { ref conditions } => {
                    pending.extend(conditions.iter().rev());
                }
            }
        }
        found
    }
}

/// What a person authored about one node of a form.
///
/// No specification governs any member of this record: it is the half of a
/// form the openEHR specifications leave open, and it is the work a template
/// revision would otherwise destroy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    /// Where the item sits among its siblings, smallest first.
    ///
    /// An item with no order keeps template order, which is the only order
    /// the definition has.
    pub order: Option<u32>,
    /// The authored section the item belongs to.
    pub section: Option<SectionId>,
    /// The label shown instead of the archetype rubric.
    pub label: Localized,
    /// The help text shown instead of the archetype description.
    pub help: Localized,
    /// The value the form starts with.
    ///
    /// The shape is the definition's own [`Prefill`], so an authored default
    /// is comparable with the one the template states rather than being a
    /// second value model.
    pub default: Option<Prefill>,
    /// When the item is shown.
    pub visibility: Visibility,
    /// The control the renderer is asked for.
    pub widget: Option<WidgetName>,
}

impl Layout {
    /// A layout that decides nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this layout decides nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Every node this layout reads, in the order it names them.
    ///
    /// The layout of one item can depend on the answer to another, and that
    /// dependency is a key like any other, so a replay resolves it too.
    #[must_use]
    pub fn referenced_nodes(&self) -> Vec<&NodeKey> {
        match self.visibility {
            Visibility::Always | Visibility::Never => Vec::new(),
            Visibility::When { ref condition } => condition.nodes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};

    use super::{Condition, Layout, Visibility};

    fn key(attribute: &str) -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    #[test]
    fn a_layout_that_decides_nothing_is_empty() {
        assert!(Layout::new().is_empty());
        let mut layout = Layout::new();
        layout.order = Some(3);
        assert!(!layout.is_empty());
    }

    #[test]
    fn a_nested_condition_names_its_nodes_in_the_order_it_reads_them() {
        let condition = Condition::All {
            conditions: vec![
                Condition::Answered { node: key("a") },
                Condition::Any {
                    conditions: vec![
                        Condition::NotAnswered { node: key("b") },
                        Condition::Answered { node: key("c") },
                    ],
                },
            ],
        };
        let mut layout = Layout::new();
        layout.visibility = Visibility::When { condition };
        let read: Vec<String> = layout
            .referenced_nodes()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(read, ["/a", "/b", "/c"]);
    }
}
