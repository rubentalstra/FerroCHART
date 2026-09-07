// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The field derivation: an operational template to a form definition.
//!
//! The mechanical half of the product. Walk the internal constraint model and
//! derive an item from the Reference Model type and the constraint at each
//! node. It is deterministic and it is written once for both ADL generations,
//! because both template readers normalize into [`crate::model`].
//!
//! # What a node becomes
//!
//! An `ELEMENT` becomes one field: openEHR RM Release-1.1.0
//! `data_structures.html` section 5.2.3 makes it the leaf "to which a
//! `DATA_VALUE` instance is attached", so the value it carries is the field
//! and the `ELEMENT` is its label, its occurrences and its null mechanism. A
//! data value under any other attribute becomes a field of its own. Every
//! other Reference Model class that holds content becomes a group. A slot the
//! template leaves open becomes undetermined content that is recorded and
//! never rendered.
//!
//! # What the specifications leave open
//!
//! Everything a renderer needs beyond the list above is ungoverned, and each
//! decision below carries its own note at the point it is made: field order,
//! grouping beyond the Reference Model tree, which rubric becomes the label
//! and which becomes the help text, conditional visibility, widget choice, a
//! node that accepts either a code or free text, validation error
//! presentation, which unit of a quantity is preferred, repeating group
//! presentation, `C_DV_STATE` semantics, and the layout overlay itself. The
//! form definition carries the facts and leaves every one of those to the
//! overlay and the renderer.

mod collapse;
pub mod error;
pub mod field;
mod temporal;
mod terms;

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{ChoiceField, FieldKind, FormField, NameConstraint, NullFlavour};
use ferrochart_form::group::{
    FormGroup, FormItem, GroupShape, SlotAssertion, UndeterminedContent, UndeterminedReason,
};
use ferrochart_form::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId,
};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::occurrences::Occurrences;
use ferrochart_form::text::Localized;
use std::collections::{BTreeMap, BTreeSet};

use crate::derive::error::DeriveError;
use crate::derive::terms::Terms;
use crate::model::multiplicity::{Cardinality, Multiplicity};
use crate::model::node::{ConstraintNode, ConstraintTemplate, NodeIdentity};
use crate::model::payload::{ConstraintPayload, OpenSlot, SlotAssertion as ModelSlotAssertion};

/// The Reference Model attribute that names a node rather than filling it.
pub(crate) const ATTRIBUTE_NAME: &str = "name";

/// The declared type of `ELEMENT.value`.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3. A field that
/// offers several alternatives collects one of them, and this is the class
/// they have in common.
const DATA_VALUE: &str = "DATA_VALUE";

/// Derives the form definition of `template`.
///
/// # Errors
/// [`DeriveError`] when a node collects a Reference Model class this
/// derivation does not model, constrains an attribute it does not model,
/// carries a constraint that cannot sit under its class, or declares sibling
/// constraints no instance could be attributed to. A constraint the
/// derivation does not understand is never absorbed into a permissive field.
pub fn form(template: &ConstraintTemplate) -> Result<FormDefinition, DeriveError> {
    // NOTE: no specification governs this: our own design. The collapse runs
    // before any key exists, so a duplicated group cannot shift the sibling
    // ordinal that section 6.2 of the architecture keys the overlay by.
    let collapsed = collapse::tied_siblings(template)?;
    let template = &collapsed;
    let deriver = Deriver {
        terms: Terms::new(template),
        default_language: LanguageTag::new(template.language().as_str()),
    };
    let root_key = NodeKey::root().child(key_step(template.root().identity()));
    // NOTE: no specification governs this: our own design. ADL 1.4 section
    // 8.5 and the AOM 2 Rules package evaluate a predicate after entry, and
    // nothing maps one onto hiding a field, so no visibility rule is derived.
    let (outcome, extra) = deriver.node(template.root(), root_key)?;
    let Derived::Group(mut root) = outcome else {
        return Err(DeriveError::RootIsNotAGroup {
            path: "/".to_owned(),
            rm_type: template.root().identity().rm_type().as_str().to_owned(),
        });
    };
    root.undetermined.extend(extra);
    let root = *root;
    // NOTE: no specification governs this: our own design. The definition
    // carries no layout at all, so a recompile replays the overlay rather
    // than discarding hand-authored work.
    Ok(FormDefinition {
        format_version: FORMAT_VERSION,
        template_id: TemplateId::new(template.id().as_str()),
        default_language: LanguageTag::new(template.language().as_str()),
        languages: deriver.terms.languages(),
        root,
    })
}

/// How many times an item may appear.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.6: an upper bound above
/// one makes the item repeatable, a lower bound of zero makes it optional, and
/// a lower bound of one makes it mandatory.
pub(crate) fn occurrences_of(occurrences: Multiplicity) -> Occurrences {
    // NOTE: no specification governs this: our own design. Whether a
    // repeatable group is an inline repeater, a modal or a table is a layout
    // decision, so the definition carries the numbers and nothing else.
    match occurrences.upper() {
        None => Occurrences::unbounded_from(occurrences.lower()),
        Some(upper) => Occurrences::bounded(occurrences.lower(), upper),
    }
}

fn key_step(identity: &NodeIdentity) -> KeyStep {
    KeyStep {
        rm_attribute: RmAttributeName::new(identity.rm_attribute().as_str()),
        node_id: identity.node_id().map(|code| LocalCode::new(code.as_str())),
        archetype_id: identity
            .archetype_id()
            .map(|id| ArchetypeId::new(id.as_str())),
        rm_type: RmTypeName::new(identity.rm_type().as_str()),
        pinned_name: identity.pinned_name().map(ToOwned::to_owned),
        sibling_ordinal: identity.sibling_ordinal(),
    }
}

fn child_key(parent: &NodeKey, identity: &NodeIdentity, tied: bool) -> NodeKey {
    let mut key = parent.child(key_step(identity));
    key.is_positional = key.is_positional || tied;
    key
}

/// The five parts that tell one child apart from its siblings, without the
/// ordinal.
type Discriminator = (String, String, String, String, String);

/// What tells one child apart from its siblings, without its ordinal.
///
/// The five parts of `docs/architecture.md` section 6.2, in the order that
/// section lists them. Two children with the same tuple are separated by
/// nothing but their position.
fn discriminator(identity: &NodeIdentity) -> Discriminator {
    (
        identity.rm_attribute().as_str().to_owned(),
        identity
            .node_id()
            .map(|code| code.as_str().to_owned())
            .unwrap_or_default(),
        identity
            .archetype_id()
            .map(|id| id.as_str().to_owned())
            .unwrap_or_default(),
        identity.rm_type().as_str().to_owned(),
        identity.pinned_name().unwrap_or_default().to_owned(),
    )
}

/// The discriminators that more than one of `siblings` shares.
///
/// A step is positionally keyed only where its whole tuple ties, which is what
/// section 6.2 decided and what a caller holding one identity cannot know.
fn tied_discriminators<'a>(
    siblings: impl Iterator<Item = &'a ConstraintNode>,
) -> BTreeSet<Discriminator> {
    let mut seen: BTreeMap<Discriminator, usize> = BTreeMap::new();
    for sibling in siblings {
        *seen.entry(discriminator(sibling.identity())).or_default() += 1;
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(tuple, _)| tuple)
        .collect()
}

/// What shape the Reference Model gives a group's content.
///
/// openEHR RM Release-1.1.0 `data_structures.html` sections 4.3.2 to 4.3.5 and
/// 5.2.2.
fn group_shape(rm_type: &str) -> GroupShape {
    // NOTE: no specification governs this: our own design. The shape names
    // the Reference Model container and nothing more, because nothing says
    // how a tree becomes pages, tabs, columns or fieldsets.
    match rm_type {
        "ITEM_SINGLE" => GroupShape::Single,
        "ITEM_LIST" => GroupShape::List,
        "ITEM_TABLE" => GroupShape::Table,
        "ITEM_TREE" => GroupShape::Tree,
        "CLUSTER" => GroupShape::Cluster,
        _ => GroupShape::Plain,
    }
}

fn slot_assertions(source: &[ModelSlotAssertion]) -> Vec<SlotAssertion> {
    source
        .iter()
        .map(|assertion| match *assertion {
            ModelSlotAssertion::ArchetypeIdPattern(ref pattern) => {
                SlotAssertion::ArchetypeIdPattern(pattern.clone())
            }
            ModelSlotAssertion::ArchetypeId(ref id) => SlotAssertion::ArchetypeId(id.clone()),
            ModelSlotAssertion::Opaque(ref text) => SlotAssertion::Opaque(text.clone()),
        })
        .collect()
}

/// What one node of the constraint model became.
#[derive(Debug)]
enum Derived {
    Group(Box<FormGroup>),
    Field(Box<FormField>),
    Undetermined(UndeterminedContent),
}

/// The walk that turns one template into one form.
#[derive(Debug)]
struct Deriver<'a> {
    terms: Terms<'a>,
    default_language: LanguageTag,
}

impl Deriver<'_> {
    /// The label and the help text of one node.
    fn labels(&self, node: &ConstraintNode) -> (Localized, Localized) {
        // NOTE: openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2 gives a
        // term a `text` and a `description` and says which is which for
        // neither; our own design makes the short one the label.
        let (mut label, help) = match node.identity().node_id() {
            Some(code) => self.terms.rubrics(
                node.terminology_scope(),
                &crate::model::ids::LocalCode::new(code.as_str()),
            ),
            None => (Localized::empty(), Localized::empty()),
        };
        // NOTE: no specification governs this: our own design. Nothing
        // distinguishes help text from a label, so the long rubric is the
        // help text and a node without one gets none.
        if label.is_empty()
            && let Some(name) = node.identity().pinned_name()
        {
            label.insert(self.default_language.clone(), name);
        }
        (label, help)
    }

    fn undetermined(
        &self,
        node: &ConstraintNode,
        key: NodeKey,
        reason: UndeterminedReason,
    ) -> UndeterminedContent {
        let (label, _help) = self.labels(node);
        UndeterminedContent {
            key,
            rm_type: RmTypeName::new(node.identity().rm_type().as_str()),
            label,
            occurrences: occurrences_of(node.occurrences()),
            reason,
        }
    }

    /// Derives one node, plus any content the node's own children left
    /// undetermined and cannot record themselves.
    fn node(
        &self,
        node: &ConstraintNode,
        key: NodeKey,
    ) -> Result<(Derived, Vec<UndeterminedContent>), DeriveError> {
        if let ConstraintPayload::OpenSlot(ref slot) = *node.payload() {
            return Ok((
                Derived::Undetermined(self.open_slot(node, key, slot)),
                Vec::new(),
            ));
        }
        let rm_type = node.identity().rm_type().as_str();
        if rm_type == "ELEMENT" {
            return self.element(node, key);
        }
        if field::is_data_value(rm_type) {
            return self.plain_field(node, key);
        }
        let group = self.group(node, key)?;
        Ok((Derived::Group(Box::new(group)), Vec::new()))
    }

    fn open_slot(
        &self,
        node: &ConstraintNode,
        key: NodeKey,
        slot: &OpenSlot,
    ) -> UndeterminedContent {
        self.undetermined(
            node,
            key,
            UndeterminedReason::OpenSlot {
                includes: slot_assertions(&slot.includes),
                excludes: slot_assertions(&slot.excludes),
            },
        )
    }

    /// A structural node and everything under it.
    fn group(&self, node: &ConstraintNode, key: NodeKey) -> Result<FormGroup, DeriveError> {
        let (label, help) = self.labels(node);
        let mut items = Vec::new();
        let mut undetermined = Vec::new();
        let mut name_constraint = None;
        let tied = tied_discriminators(node.children().iter());
        // NOTE: no specification governs this: our own design. Items keep
        // template order, the only order there is, and the cardinality flag
        // below says whether that order carries meaning.
        for child in node.children() {
            let attribute = child.identity().rm_attribute().as_str();
            if attribute == ATTRIBUTE_NAME {
                if node.identity().pinned_name().is_none() {
                    name_constraint = self.name_constraint(child, &key)?;
                }
                continue;
            }
            if field::is_never_entered(attribute) {
                continue;
            }
            let is_tied = tied.contains(&discriminator(child.identity()));
            let (derived, extra) = self.node(child, child_key(&key, child.identity(), is_tied))?;
            undetermined.extend(extra);
            match derived {
                Derived::Group(group) => items.push(FormItem::Group(group)),
                Derived::Field(field) => items.push(FormItem::Field(field)),
                Derived::Undetermined(content) => undetermined.push(content),
            }
        }
        let cardinality = node.attribute().cardinality();
        Ok(FormGroup {
            key,
            rm_type: RmTypeName::new(node.identity().rm_type().as_str()),
            archetype_id: node
                .identity()
                .archetype_id()
                .map(|id| ArchetypeId::new(id.as_str())),
            label,
            help,
            occurrences: occurrences_of(node.occurrences()),
            shape: group_shape(node.identity().rm_type().as_str()),
            name_constraint,
            is_ordered: cardinality.is_some_and(Cardinality::is_ordered),
            is_unique: cardinality.is_some_and(Cardinality::is_unique),
            is_deprecated: node.is_deprecated(),
            items,
            undetermined,
        })
    }

    /// The constraint on a node's Reference Model name, where the template
    /// leaves the name open.
    fn name_constraint(
        &self,
        node: &ConstraintNode,
        parent: &NodeKey,
    ) -> Result<Option<NameConstraint>, DeriveError> {
        let key = child_key(parent, node.identity(), false);
        let path = key.to_string();
        Ok(
            field::value_kind(&self.terms, node, &path)?.map(|kind| NameConstraint {
                key,
                rm_type: RmTypeName::new(node.identity().rm_type().as_str()),
                kind,
            }),
        )
    }

    /// The one field several `value` alternatives are presented as.
    ///
    /// openEHR AM Release-2.3.0 `AOM2.html` section 4.2.8.2 prescribes sibling
    /// nodes for a value that may be a code or free text; presenting one field
    /// for them is our own design.
    fn choice(
        &self,
        alternatives: Vec<(&ConstraintNode, NodeKey, FieldKind)>,
    ) -> Result<FieldKind, DeriveError> {
        let mut collected = Vec::with_capacity(alternatives.len());
        for (child, alternative_key, kind) in alternatives {
            let (label, _help) = self.labels(child);
            collected.push(field::alternative(
                &self.terms,
                alternative_key,
                child,
                kind,
                label,
            )?);
        }
        Ok(FieldKind::Choice(ChoiceField {
            alternatives: collected,
        }))
    }

    /// An `ELEMENT`: one field, whose kind comes from what the template lets
    /// its `value` hold.
    fn element(
        &self,
        node: &ConstraintNode,
        key: NodeKey,
    ) -> Result<(Derived, Vec<UndeterminedContent>), DeriveError> {
        let mut name_constraint = None;
        for child in node.children() {
            let attribute = child.identity().rm_attribute().as_str();
            if attribute == ATTRIBUTE_NAME {
                if node.identity().pinned_name().is_none() {
                    name_constraint = self.name_constraint(child, &key)?;
                }
            } else if attribute != "value" && !field::is_never_entered(attribute) {
                return Err(DeriveError::UnmodelledAttribute {
                    path: key.to_string(),
                    rm_type: "ELEMENT".to_owned(),
                    attribute: attribute.to_owned(),
                });
            }
        }

        let values = field::value_children(node);
        let tied_values = tied_discriminators(values.iter().copied());
        let mut alternatives = Vec::new();
        for child in &values {
            let is_tied = tied_values.contains(&discriminator(child.identity()));
            let value_key = child_key(&key, child.identity(), is_tied);
            let path = value_key.to_string();
            match field::value_kind(&self.terms, child, &path)? {
                None => {}
                Some(kind) => alternatives.push((*child, value_key, kind)),
            }
        }

        if alternatives.is_empty() {
            let reason = if values.is_empty() {
                UndeterminedReason::UnconstrainedValue
            } else {
                UndeterminedReason::UntypedInterval
            };
            return Ok((
                Derived::Undetermined(self.undetermined(node, key, reason)),
                Vec::new(),
            ));
        }

        let mut undetermined = Vec::new();
        for child in &values {
            let is_tied = tied_values.contains(&discriminator(child.identity()));
            let value_key = child_key(&key, child.identity(), is_tied);
            if !alternatives.iter().any(|(_, key, _)| *key == value_key) {
                undetermined.push(self.undetermined(
                    child,
                    value_key,
                    UndeterminedReason::UntypedInterval,
                ));
            }
        }

        let (label, help) = self.labels(node);
        let (rm_type, kind, prefill, reference_ranges) = match <[_; 1]>::try_from(alternatives) {
            Ok([(child, value_key, kind)]) => (
                child.identity().rm_type().as_str().to_owned(),
                kind,
                node.default_value()
                    .or_else(|| child.default_value())
                    .map(field::prefill),
                // openEHR RM Release-1.1.0 `data_types.html` section 6.2.1
                // puts the reference bands on the data value, so they are
                // read from the value node rather than from the `ELEMENT`.
                field::reference_ranges(&self.terms, child, &value_key.to_string())?,
            ),
            Err(alternatives) => (
                DATA_VALUE.to_owned(),
                self.choice(alternatives)?,
                node.default_value().map(field::prefill),
                // Each alternative is a data value of its own, so each
                // carries its own bands.
                None,
            ),
        };

        let occurrences = occurrences_of(node.occurrences());
        let cardinality = node.attribute().cardinality();
        // NOTE: no specification governs this: our own design. The field
        // carries the constraint and never a message, so how a violation is
        // presented stays a renderer decision.
        Ok((
            Derived::Field(Box::new(FormField {
                key,
                rm_type: RmTypeName::new(rm_type),
                label,
                help,
                occurrences,
                is_fixed: field::is_fixed(&kind),
                kind,
                name_constraint,
                reference_ranges,
                is_ordered: cardinality.is_some_and(Cardinality::is_ordered),
                is_unique: cardinality.is_some_and(Cardinality::is_unique),
                // openEHR RM Release-1.1.0 `data_structures.html` section
                // 5.2.3 puts the null mechanism on `ELEMENT` and nowhere
                // else, and a field has either a value or a null flavour.
                null_flavour: if occurrences.is_optional() {
                    NullFlavour::offered()
                } else {
                    NullFlavour::absent()
                },
                prefill,
                is_deprecated: node.is_deprecated(),
            })),
            undetermined,
        ))
    }

    /// A data value the template constrains outside an `ELEMENT`.
    fn plain_field(
        &self,
        node: &ConstraintNode,
        key: NodeKey,
    ) -> Result<(Derived, Vec<UndeterminedContent>), DeriveError> {
        let path = key.to_string();
        let Some(kind) = field::value_kind(&self.terms, node, &path)? else {
            return Ok((
                Derived::Undetermined(self.undetermined(
                    node,
                    key,
                    UndeterminedReason::UntypedInterval,
                )),
                Vec::new(),
            ));
        };
        let (label, help) = self.labels(node);
        let cardinality = node.attribute().cardinality();
        Ok((
            Derived::Field(Box::new(FormField {
                key,
                rm_type: RmTypeName::new(node.identity().rm_type().as_str()),
                label,
                help,
                occurrences: occurrences_of(node.occurrences()),
                is_fixed: field::is_fixed(&kind),
                kind,
                name_constraint: None,
                reference_ranges: field::reference_ranges(&self.terms, node, &path)?,
                is_ordered: cardinality.is_some_and(Cardinality::is_ordered),
                is_unique: cardinality.is_some_and(Cardinality::is_unique),
                // The null mechanism belongs to `ELEMENT`, so a value
                // constrained anywhere else carries no null flavour.
                null_flavour: NullFlavour::absent(),
                prefill: node.default_value().map(field::prefill),
                is_deprecated: node.is_deprecated(),
            })),
            Vec::new(),
        ))
    }
}
