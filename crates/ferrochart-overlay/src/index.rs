// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The nodes of one form definition, indexed by the key that names them.
//!
//! The overlay resolves a stored key against a recompiled definition, and this
//! is what it resolves against. The index is built once per definition and
//! iterates ordered structures only, so a resolution is deterministic.
//!
//! NOTE: no specification governs this: our own design. [`FormItem`] is
//! non-exhaustive, so an item kind this crate does not know is not indexed
//! and no overlay entry can decorate it.

use std::collections::BTreeMap;

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::FormField;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::{ArchetypeId, LocalCode, RmAttributeName, RmTypeName};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::text::Localized;

/// One step of a key, with the sibling ordinal left out.
///
/// The five parts are what openEHR leaves as the node's identity once an
/// id-only path is known not to be unique inside an operational template; the
/// ordinal is the tie-break the overlay records separately, so it is not part
/// of matching.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MatchStep {
    rm_attribute: RmAttributeName,
    node_id: Option<LocalCode>,
    archetype_id: Option<ArchetypeId>,
    rm_type: Option<RmTypeName>,
    pinned_name: Option<String>,
}

impl MatchStep {
    fn of(step: &KeyStep) -> Self {
        Self {
            rm_attribute: step.rm_attribute.clone(),
            node_id: step.node_id.clone(),
            archetype_id: step.archetype_id.clone(),
            rm_type: Some(step.rm_type.clone()),
            pinned_name: step.pinned_name.clone(),
        }
    }
}

/// A key reduced to what a resolution matches on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MatchKey(Vec<MatchStep>);

impl MatchKey {
    /// The match key of `key`.
    pub(crate) fn of(key: &NodeKey) -> Self {
        Self(key.steps.iter().map(MatchStep::of).collect())
    }

    /// This key with the Reference Model type of its last step left out.
    ///
    /// A node whose type changed cannot match a key that carries the old
    /// type, so the type of the terminal step is dropped to find it. Every
    /// other part of every step still has to agree.
    pub(crate) fn without_terminal_type(&self) -> Self {
        let mut steps = self.0.clone();
        if let Some(last) = steps.last_mut() {
            last.rm_type = None;
        }
        Self(steps)
    }
}

/// What one entry of the index decorates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    /// A group of the form.
    Group,
    /// A field a clinician fills.
    Field,
}

/// One node of a form definition an overlay entry can decorate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Target<'a> {
    key: &'a NodeKey,
    kind: TargetKind,
    label: &'a Localized,
    rm_type: &'a RmTypeName,
    group: Option<&'a FormGroup>,
}

impl<'a> Target<'a> {
    /// What names the node in the definition it was read from.
    pub(crate) fn key(&self) -> &'a NodeKey {
        self.key
    }

    /// Whether the node is a group or a field.
    pub(crate) fn kind(&self) -> TargetKind {
        self.kind
    }

    /// The text the definition labels the node with.
    pub(crate) fn label(&self) -> &'a Localized {
        self.label
    }

    /// The Reference Model class the node carries.
    pub(crate) fn rm_type(&self) -> &'a RmTypeName {
        self.rm_type
    }

    /// The node and everything under it, as one deterministic line.
    ///
    /// The depth-annotated pre-order of a subtree determines the subtree, so
    /// two siblings a key cannot tell apart have the same signature only when
    /// they carry the same content.
    pub(crate) fn signature(&self) -> String {
        let Some(group) = self.group else {
            return line(0, self.key, self.rm_type);
        };
        let mut written = String::new();
        let mut pending = vec![(0_usize, Subtree::Group(group))];
        while let Some((depth, node)) = pending.pop() {
            match node {
                Subtree::Group(group) => {
                    written.push_str(&line(depth, &group.key, &group.rm_type));
                    let next = depth.saturating_add(1);
                    for item in group.items.iter().rev() {
                        pending.extend(Subtree::of(item).map(|node| (next, node)));
                    }
                }
                Subtree::Field(field) => {
                    written.push_str(&line(depth, &field.key, &field.rm_type));
                }
            }
        }
        written
    }
}

/// One node of a subtree being signed.
#[derive(Debug, Clone, Copy)]
enum Subtree<'a> {
    Group(&'a FormGroup),
    Field(&'a FormField),
}

impl<'a> Subtree<'a> {
    fn of(item: &'a FormItem) -> Option<Self> {
        match *item {
            FormItem::Group(ref group) => Some(Self::Group(group.as_ref())),
            FormItem::Field(ref field) => Some(Self::Field(field.as_ref())),
            _ => None,
        }
    }
}

/// One node of a signature: its depth, its last key step and its class.
fn line(depth: usize, key: &NodeKey, rm_type: &RmTypeName) -> String {
    let step = key
        .terminal()
        .map_or_else(|| "/".to_owned(), ToString::to_string);
    format!("{depth}{step}:{rm_type};")
}

/// Every node of one form definition an overlay entry can decorate.
///
/// Content the template left undetermined is not in the index. A form renders
/// nothing for it, so there is nothing to lay out, and a node that becomes
/// undetermined therefore reads as a node that disappeared.
#[derive(Debug)]
pub(crate) struct Index<'a> {
    targets: Vec<Target<'a>>,
    by_key: BTreeMap<MatchKey, Vec<usize>>,
    by_key_without_type: BTreeMap<MatchKey, Vec<usize>>,
    by_terminal: BTreeMap<(LocalCode, RmTypeName), Vec<usize>>,
}

impl<'a> Index<'a> {
    /// Indexes every group and field of `definition`.
    pub(crate) fn new(definition: &'a FormDefinition) -> Self {
        let mut targets = Vec::new();
        collect(&definition.root, &mut targets);
        targets.sort_by(|left, right| left.key.cmp(right.key));

        let mut by_key: BTreeMap<MatchKey, Vec<usize>> = BTreeMap::new();
        let mut by_key_without_type: BTreeMap<MatchKey, Vec<usize>> = BTreeMap::new();
        let mut by_terminal: BTreeMap<(LocalCode, RmTypeName), Vec<usize>> = BTreeMap::new();
        for (position, target) in targets.iter().enumerate() {
            let key = MatchKey::of(target.key);
            by_key_without_type
                .entry(key.without_terminal_type())
                .or_default()
                .push(position);
            by_key.entry(key).or_default().push(position);
            if let Some(step) = target.key.terminal()
                && let Some(node_id) = step.node_id.as_ref()
            {
                by_terminal
                    .entry((node_id.clone(), step.rm_type.clone()))
                    .or_default()
                    .push(position);
            }
        }
        Self {
            targets,
            by_key,
            by_key_without_type,
            by_terminal,
        }
    }

    /// Every node of the definition, in key order.
    pub(crate) fn targets(&self) -> &[Target<'a>] {
        &self.targets
    }

    /// Every node whose five-part step chain is the one `key` states.
    pub(crate) fn resolve(&self, key: &NodeKey) -> Vec<Target<'a>> {
        self.pick(self.by_key.get(&MatchKey::of(key)))
    }

    /// Every node whose step chain is the one `key` states apart from the
    /// Reference Model type of its last step.
    pub(crate) fn resolve_retyped(&self, key: &NodeKey) -> Vec<Target<'a>> {
        let without_type = MatchKey::of(key).without_terminal_type();
        self.pick(self.by_key_without_type.get(&without_type))
    }

    /// Every node carrying the terminal node id and Reference Model type of
    /// `key`, wherever it sits.
    ///
    /// A key whose last step has no node id has nothing to be recognized by,
    /// so it matches nothing here rather than matching by type alone.
    pub(crate) fn resolve_moved(&self, key: &NodeKey) -> Vec<Target<'a>> {
        let Some(step) = key.terminal() else {
            return Vec::new();
        };
        let Some(node_id) = step.node_id.as_ref() else {
            return Vec::new();
        };
        self.pick(
            self.by_terminal
                .get(&(node_id.clone(), step.rm_type.clone())),
        )
    }

    fn pick(&self, positions: Option<&Vec<usize>>) -> Vec<Target<'a>> {
        positions.map_or_else(Vec::new, |found| {
            found
                .iter()
                .filter_map(|position| self.targets.get(*position).copied())
                .collect()
        })
    }
}

/// Every group and field of one subtree, in no particular order.
fn collect<'a>(group: &'a FormGroup, into: &mut Vec<Target<'a>>) {
    into.push(Target {
        key: &group.key,
        kind: TargetKind::Group,
        label: &group.label,
        rm_type: &group.rm_type,
        group: Some(group),
    });
    for item in &group.items {
        match *item {
            FormItem::Group(ref nested) => collect(nested.as_ref(), into),
            FormItem::Field(ref field) => into.push(Target {
                key: &field.key,
                kind: TargetKind::Field,
                label: &field.label,
                rm_type: &field.rm_type,
                group: None,
            }),
            _ => {}
        }
    }
}
