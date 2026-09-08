// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The tree a form is: groups of items, and the content the template left
//! undetermined.

use serde::{Deserialize, Serialize};

use crate::field::{FormField, NameConstraint};
use crate::ids::{ArchetypeId, RmTypeName};
use crate::key::NodeKey;
use crate::occurrences::Occurrences;
use crate::text::Localized;

/// One group of a form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormGroup {
    /// What names the group.
    pub key: NodeKey,
    /// The Reference Model class the group was derived from.
    pub rm_type: RmTypeName,
    /// The archetype this group is the root of, where it is one.
    pub archetype_id: Option<ArchetypeId>,
    /// The text the group is labelled with.
    pub label: Localized,
    /// The longer text shown beside the group.
    pub help: Localized,
    /// How many times the group may appear.
    pub occurrences: Occurrences,
    /// What shape the Reference Model gives the group's content.
    pub shape: GroupShape,
    /// The constraint on the node's Reference Model name, where the template
    /// leaves the name open.
    pub name_constraint: Option<NameConstraint>,
    /// Whether the order of the group's members carries meaning.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.5,
    /// `CARDINALITY.is_ordered`, of the attribute the group sits under.
    pub is_ordered: bool,
    /// Whether the group's members must differ from one another.
    pub is_unique: bool,
    /// Whether the archetype marks the node deprecated.
    pub is_deprecated: bool,
    /// The group's items, in template order.
    pub items: Vec<FormItem>,
    /// The content the template left undetermined, which is recorded and
    /// never rendered.
    pub undetermined: Vec<UndeterminedContent>,
}

/// One member of a group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FormItem {
    /// A nested group.
    Group(Box<FormGroup>),
    /// A field a clinician fills.
    Field(Box<FormField>),
}

/// What shape the Reference Model gives a group's content.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 4.3 defines the
/// `ITEM_STRUCTURE` subtypes, and section 4.2 gives them their shape through
/// the ISO 13606 encoding rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GroupShape {
    /// `ITEM_SINGLE` (section 4.3.2): one field.
    Single,
    /// `ITEM_LIST` (section 4.3.3): a flat list of fields.
    List,
    /// `ITEM_TABLE` (section 4.3.4): a grid whose rows are clusters, whose
    /// column names are the element names, and whose empty cell is an
    /// `ELEMENT` with a null flavour rather than an absent one.
    Table,
    /// `ITEM_TREE` (section 4.3.5): nested groups.
    Tree,
    /// `CLUSTER` (section 5.2.2): an ordered list of items.
    Cluster,
    /// Any other Reference Model class that holds content.
    Plain,
}

/// Content the operational template did not determine.
///
/// A form renders nothing for it and never guesses, and the entry survives so
/// a person can see that the template left a hole rather than that the
/// compiler lost one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndeterminedContent {
    /// What names the content.
    pub key: NodeKey,
    /// The Reference Model class the template names for it.
    pub rm_type: RmTypeName,
    /// The text the node is labelled with, where the template carries one.
    pub label: Localized,
    /// How many times the content may appear.
    pub occurrences: Occurrences,
    /// Why the content is undetermined.
    pub reason: UndeterminedReason,
}

/// Why a piece of content is undetermined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UndeterminedReason {
    /// An archetype slot the template leaves open.
    ///
    /// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 removes only closed
    /// slots and inlines every filler, so an open slot is a legitimate runtime
    /// extension point that survives into an operational template. A slot
    /// whose occurrences make its content mandatory is refused at read time
    /// instead.
    OpenSlot {
        /// The archetypes the slot admits.
        includes: Vec<SlotAssertion>,
        /// The archetypes the slot refuses.
        excludes: Vec<SlotAssertion>,
    },
    /// An `ELEMENT` whose `value` the template constrains not at all.
    ///
    /// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 types
    /// `ELEMENT.value` as a `DATA_VALUE`, so an unconstrained one admits every
    /// data type there is. Rendering one of them would be a guess at which,
    /// and the same section's null mechanism still lets the `ELEMENT` be
    /// satisfied, so the field is recorded rather than invented.
    UnconstrainedValue,
    /// A `DV_INTERVAL` whose element type the template never states.
    ///
    /// openEHR RM Release-1.1.0 `data_types.html` section 6.2.2 types both
    /// ends as the same `DV_ORDERED` descendant, and a template that names
    /// neither the type parameter nor either end has not said which.
    UntypedInterval,
}

/// One assertion on an archetype slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SlotAssertion {
    /// A regular expression over archetype identifiers.
    ArchetypeIdPattern(String),
    /// An archetype identifier the template names outright.
    ArchetypeId(String),
    /// An assertion FerroCHART does not interpret, kept as text so nothing is
    /// dropped.
    Opaque(String),
}

impl FormGroup {
    /// Every group of this subtree, this one first, in template order.
    pub fn walk_groups(&self) -> impl Iterator<Item = &Self> {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            let group = stack.pop()?;
            stack.extend(group.items.iter().rev().filter_map(|item| match *item {
                FormItem::Group(ref nested) => Some(nested.as_ref()),
                FormItem::Field(_) => None,
            }));
            Some(group)
        })
    }

    /// Every field of this subtree, in template order.
    pub fn walk_fields(&self) -> impl Iterator<Item = &FormField> {
        self.walk_groups()
            .flat_map(|group| group.items.iter())
            .filter_map(|item| match *item {
                FormItem::Field(ref field) => Some(field.as_ref()),
                FormItem::Group(_) => None,
            })
    }

    /// Every piece of undetermined content of this subtree.
    pub fn walk_undetermined(&self) -> impl Iterator<Item = &UndeterminedContent> {
        self.walk_groups()
            .flat_map(|group| group.undetermined.iter())
    }
}
