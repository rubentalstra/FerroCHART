// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The node tree both readers fill, and the template that owns it.

use std::collections::BTreeMap;

use crate::model::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId,
};
use crate::model::multiplicity::{AttributeContext, Multiplicity};
use crate::model::payload::{ConstraintPayload, DefaultValue};
use crate::model::terminology::Terminology;

/// What tells one node apart from its siblings.
///
/// No specification governs this shape; it is FerroCHART's own design, and it
/// was measured rather than assumed. An id-only path is not unique inside an
/// operational template, because openEHR BASE Release-1.2.0
/// `architecture_overview.html` section 11.2.2.2 guarantees uniqueness of an
/// archetype path only *in an archetype*, and an operational template composes
/// many. The five parts below are what separates a colliding sibling group;
/// the ordinal is the last resort where the whole tuple still ties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    rm_attribute: RmAttributeName,
    node_id: Option<LocalCode>,
    archetype_id: Option<ArchetypeId>,
    rm_type: RmTypeName,
    pinned_name: Option<String>,
    sibling_ordinal: usize,
}

impl NodeIdentity {
    /// Builds an identity from its six parts.
    #[must_use]
    pub fn new(
        rm_attribute: RmAttributeName,
        node_id: Option<LocalCode>,
        archetype_id: Option<ArchetypeId>,
        rm_type: RmTypeName,
        pinned_name: Option<String>,
        sibling_ordinal: usize,
    ) -> Self {
        Self {
            rm_attribute,
            node_id,
            archetype_id,
            rm_type,
            pinned_name,
            sibling_ordinal,
        }
    }

    /// The Reference Model attribute the node sits under.
    #[must_use]
    pub fn rm_attribute(&self) -> &RmAttributeName {
        &self.rm_attribute
    }

    /// The node's local code, where it has one.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.1 lets a leaf with
    /// no siblings carry no node id at all, so this is genuinely optional.
    #[must_use]
    pub fn node_id(&self) -> Option<&LocalCode> {
        self.node_id.as_ref()
    }

    /// The archetype this node is the root of, where it is one.
    #[must_use]
    pub fn archetype_id(&self) -> Option<&ArchetypeId> {
        self.archetype_id.as_ref()
    }

    /// The Reference Model type the node constrains.
    #[must_use]
    pub fn rm_type(&self) -> &RmTypeName {
        &self.rm_type
    }

    /// The name the template pins on the node, where it states one.
    ///
    /// This is a definition fact the template states, not a predicate matched
    /// against data.
    #[must_use]
    pub fn pinned_name(&self) -> Option<&str> {
        self.pinned_name.as_deref()
    }

    /// The node's position among its siblings under the same attribute,
    /// counting from zero.
    #[must_use]
    pub fn sibling_ordinal(&self) -> usize {
        self.sibling_ordinal
    }

    /// Renumbers the node's position among its siblings.
    pub(crate) fn set_sibling_ordinal(&mut self, ordinal: usize) {
        self.sibling_ordinal = ordinal;
    }

    /// The five parts that separate this node from its siblings, without the
    /// ordinal.
    pub(crate) fn discriminates_like(&self, other: &Self) -> bool {
        self.rm_attribute == other.rm_attribute
            && self.node_id == other.node_id
            && self.archetype_id == other.archetype_id
            && self.rm_type == other.rm_type
            && self.pinned_name == other.pinned_name
    }
}

/// One node of the internal constraint model.
///
/// A node carries everything the field derivation needs and nothing that says
/// which ADL generation produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintNode {
    identity: NodeIdentity,
    occurrences: Multiplicity,
    occurrences_stated: bool,
    attribute: AttributeContext,
    payload: ConstraintPayload,
    terminology_scope: ArchetypeId,
    is_deprecated: bool,
    default_value: Option<DefaultValue>,
    children: Vec<ConstraintNode>,
}

impl ConstraintNode {
    /// Builds a node from its parts.
    #[expect(
        clippy::too_many_arguments,
        reason = "the constructor takes exactly the facts a reader owes the model; grouping them into a second struct would only move the arity"
    )]
    #[must_use]
    pub fn new(
        identity: NodeIdentity,
        occurrences: Multiplicity,
        occurrences_stated: bool,
        attribute: AttributeContext,
        payload: ConstraintPayload,
        terminology_scope: ArchetypeId,
        is_deprecated: bool,
        default_value: Option<DefaultValue>,
        children: Vec<ConstraintNode>,
    ) -> Self {
        Self {
            identity,
            occurrences,
            occurrences_stated,
            attribute,
            payload,
            terminology_scope,
            is_deprecated,
            default_value,
            children,
        }
    }

    /// What tells this node apart from its siblings.
    #[must_use]
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    /// How many times this node may repeat under its attribute.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.6,
    /// `C_OBJECT.occurrences`.
    #[must_use]
    pub fn occurrences(&self) -> Multiplicity {
        self.occurrences
    }

    /// Whether the template states the occurrences, or the reader inferred
    /// them.
    ///
    /// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.2 makes
    /// `C_OBJECT.occurrences` optional and says an unstated value is inferred
    /// from the existence and cardinality of the containing attribute, which
    /// is what the reader does. An ADL 1.4 operational template always states
    /// it, because the ITS-XML `Archetype.xsd` `C_OBJECT` declares
    /// `occurrences` without `minOccurs="0"`.
    #[must_use]
    pub fn occurrences_stated(&self) -> bool {
        self.occurrences_stated
    }

    /// What the owning attribute says about this node's slot.
    #[must_use]
    pub fn attribute(&self) -> AttributeContext {
        self.attribute
    }

    /// What this node constrains.
    #[must_use]
    pub fn payload(&self) -> &ConstraintPayload {
        &self.payload
    }

    /// The archetype whose terminology governs this node's codes.
    #[must_use]
    pub fn terminology_scope(&self) -> &ArchetypeId {
        &self.terminology_scope
    }

    /// Whether the archetype marks this node deprecated.
    ///
    /// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.2,
    /// `C_OBJECT.is_deprecated`. ADL 1.4 has no way to say this, so a node
    /// read from an ADL 1.4 template is never deprecated.
    #[must_use]
    pub fn is_deprecated(&self) -> bool {
        self.is_deprecated
    }

    /// The value the template prefills this node with.
    #[must_use]
    pub fn default_value(&self) -> Option<&DefaultValue> {
        self.default_value.as_ref()
    }

    /// The node's children, in template order.
    #[must_use]
    pub fn children(&self) -> &[ConstraintNode] {
        &self.children
    }

    /// Records the value a template prefills this node with.
    pub(crate) fn set_default_value(&mut self, value: DefaultValue) {
        self.default_value = Some(value);
    }

    /// What tells this node apart from its siblings, so a pass that rebuilds
    /// the sibling order can renumber it.
    pub(crate) fn identity_mut(&mut self) -> &mut NodeIdentity {
        &mut self.identity
    }

    /// Restates how many times this node may repeat under its attribute.
    pub(crate) fn set_occurrences(&mut self, occurrences: Multiplicity) {
        self.occurrences = occurrences;
    }

    /// The node's children, for a pass that decorates the built tree.
    pub(crate) fn children_mut(&mut self) -> &mut Vec<ConstraintNode> {
        &mut self.children
    }

    /// Every node of this subtree, this one first, in template order.
    pub fn walk(&self) -> impl Iterator<Item = &ConstraintNode> {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            let node = stack.pop()?;
            stack.extend(node.children.iter().rev());
            Some(node)
        })
    }
}

/// An operational template read into the internal constraint model.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintTemplate {
    id: TemplateId,
    language: LanguageTag,
    root: ConstraintNode,
    terminologies: BTreeMap<ArchetypeId, Terminology>,
}

impl ConstraintTemplate {
    /// Builds a template from its parts.
    #[must_use]
    pub fn new(
        id: TemplateId,
        language: LanguageTag,
        root: ConstraintNode,
        terminologies: BTreeMap<ArchetypeId, Terminology>,
    ) -> Self {
        Self {
            id,
            language,
            root,
            terminologies,
        }
    }

    /// The identifier the template states for itself.
    #[must_use]
    pub fn id(&self) -> &TemplateId {
        &self.id
    }

    /// The language the template is authored in.
    #[must_use]
    pub fn language(&self) -> &LanguageTag {
        &self.language
    }

    /// The root node, which is always an archetype root.
    #[must_use]
    pub fn root(&self) -> &ConstraintNode {
        &self.root
    }

    /// The terminology of `archetype`, where the template carries one.
    #[must_use]
    pub fn terminology(&self, archetype: &ArchetypeId) -> Option<&Terminology> {
        self.terminologies.get(archetype)
    }

    /// Every terminology the template carries, ordered by archetype.
    #[must_use]
    pub fn terminologies(&self) -> &BTreeMap<ArchetypeId, Terminology> {
        &self.terminologies
    }

    /// Every node in the template, the root first, in template order.
    pub fn walk(&self) -> impl Iterator<Item = &ConstraintNode> {
        self.root.walk()
    }

    /// The root node, for a pass that rewrites the tree before it is derived.
    pub(crate) fn root_mut(&mut self) -> &mut ConstraintNode {
        &mut self.root
    }
}
