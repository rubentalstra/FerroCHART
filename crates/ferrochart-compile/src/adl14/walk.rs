// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The walk from a parsed operational template to the internal model.

use std::collections::BTreeMap;

use openehr_its::opt14::types as opt;

use crate::adl14::{defaults, name, path, payload, slot, terminology};
use crate::error::{MAX_DEPTH, ReadError};
use crate::model::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId, TerminologyName,
};
use crate::model::multiplicity::{AttributeContext, Cardinality, Multiplicity};
use crate::model::node::{ConstraintNode, ConstraintTemplate, NodeIdentity};
use crate::model::payload::{ConstraintPayload, OpenSlot};
use crate::model::terminology::Terminology;

/// The state one walk carries down the tree.
struct Walk {
    language: LanguageTag,
    terminologies: BTreeMap<ArchetypeId, Terminology>,
}

/// Where the walk currently is: the archetype whose terminology governs the
/// node, and the archetype root an internal reference resolves against.
#[derive(Clone, Copy)]
struct Scope<'a> {
    archetype: &'a ArchetypeId,
    root: &'a opt::CObject,
}

pub(crate) fn read(template: &opt::OperationalTemplate) -> Result<ConstraintTemplate, ReadError> {
    let language = LanguageTag::new(template.language.code_string.clone());
    let mut walk = Walk {
        language: language.clone(),
        terminologies: terminology::from_ontologies(template),
    };

    let root_archetype = ArchetypeId::new(template.definition.archetype_id.value.clone());
    let root_object = opt::CObject::CArchetypeRoot(template.definition.clone());
    let root_path = format!("[{root_archetype}]");
    let scope = Scope {
        archetype: &root_archetype,
        root: &root_object,
    };

    let root_occurrences = payload::multiplicity(&template.definition.occurrences, &root_path)?;
    terminology::add_archetype_root(
        &mut walk.terminologies,
        &template.definition,
        &walk.language.clone(),
    );

    let identity = NodeIdentity::new(
        // The root sits under no attribute of anything, and the empty name is
        // what an overlay key step carries there.
        RmAttributeName::new(String::new()),
        local_code(&template.definition.node_id),
        Some(root_archetype.clone()),
        RmTypeName::new(template.definition.rm_type_name.clone()),
        name::pinned(&root_object),
        0,
    );
    let children = walk.attributes(&template.definition.attributes, scope, &root_path, 1)?;
    let mut root = ConstraintNode::new(
        identity,
        root_occurrences,
        true,
        AttributeContext::single(None),
        ConstraintPayload::Structure,
        root_archetype,
        false,
        None,
        children,
    );

    defaults::apply(&mut root, template.constraints.as_ref())?;

    Ok(ConstraintTemplate::new(
        TemplateId::new(template.template_id.value.clone()),
        language,
        root,
        walk.terminologies,
    ))
}

fn local_code(node_id: &str) -> Option<LocalCode> {
    (!node_id.is_empty()).then(|| LocalCode::new(node_id.to_owned()))
}

fn step_path(parent: &str, attribute: &str, object: &opt::CObject) -> String {
    let predicate = if let opt::CObject::CArchetypeRoot(ref root) = *object {
        Some(root.archetype_id.value.clone())
    } else {
        let node_id = path::node_id(object);
        (!node_id.is_empty()).then(|| node_id.to_owned())
    };
    match predicate {
        Some(predicate) => format!("{parent}/{attribute}[{predicate}]"),
        None => format!("{parent}/{attribute}"),
    }
}

impl Walk {
    /// Walks every attribute of one object, in template order.
    fn attributes(
        &mut self,
        attributes: &[opt::CAttribute],
        scope: Scope<'_>,
        parent_path: &str,
        depth: usize,
    ) -> Result<Vec<ConstraintNode>, ReadError> {
        let mut built = Vec::new();
        for attribute in attributes {
            let (attribute_name, children) = path::attribute_parts(attribute);
            let context = attribute_context(attribute, parent_path)?;
            let mut ordinal = 0_usize;
            for child in children {
                let child_path = step_path(parent_path, attribute_name, child);
                if let Some(node) = self.object(
                    child,
                    attribute_name,
                    ordinal,
                    context,
                    scope,
                    &child_path,
                    depth,
                )? {
                    built.push(node);
                    ordinal = ordinal.saturating_add(1);
                }
            }
        }
        Ok(built)
    }

    /// Walks one `C_OBJECT`, returning nothing when the template removed it.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk threads the node's attribute context, scope and path down the tree; a parameter struct would only rename them"
    )]
    fn object(
        &mut self,
        object: &opt::CObject,
        attribute_name: &str,
        ordinal: usize,
        context: AttributeContext,
        scope: Scope<'_>,
        node_path: &str,
        depth: usize,
    ) -> Result<Option<ConstraintNode>, ReadError> {
        if depth > MAX_DEPTH {
            return Err(ReadError::TooDeep {
                path: node_path.to_owned(),
            });
        }
        let occurrences = payload::multiplicity(path::occurrences(object), node_path)?;
        // openEHR AM Release-2.3.0 OPT2.html section 3.3: a node with
        // `occurrences matches {0}` is removed from the operational template,
        // so the template has taken it away and no form renders it.
        if occurrences.is_prohibited() {
            return Ok(None);
        }

        if let opt::CObject::ArchetypeInternalRef(ref reference) = *object {
            return self
                .internal_reference(
                    reference,
                    attribute_name,
                    ordinal,
                    context,
                    scope,
                    node_path,
                    depth,
                )
                .map(Some);
        }

        let (payload, children, scope_archetype, archetype_id) =
            self.payload(object, scope, node_path, occurrences, depth)?;

        Ok(Some(ConstraintNode::new(
            NodeIdentity::new(
                RmAttributeName::new(attribute_name.to_owned()),
                local_code(path::node_id(object)),
                archetype_id,
                RmTypeName::new(rm_type_name(object)),
                name::pinned(object),
                ordinal,
            ),
            occurrences,
            // The ITS-XML `Archetype.xsd` makes `C_OBJECT.occurrences`
            // mandatory, so an ADL 1.4 node always states it.
            true,
            context,
            payload,
            scope_archetype,
            // ADL 1.4 has no `C_OBJECT.is_deprecated`.
            false,
            None,
            children,
        )))
    }

    /// The payload, children, terminology scope and archetype id of one
    /// object.
    fn payload(
        &mut self,
        object: &opt::CObject,
        scope: Scope<'_>,
        node_path: &str,
        occurrences: Multiplicity,
        depth: usize,
    ) -> Result<
        (
            ConstraintPayload,
            Vec<ConstraintNode>,
            ArchetypeId,
            Option<ArchetypeId>,
        ),
        ReadError,
    > {
        let next_depth = depth.saturating_add(1);
        match *object {
            opt::CObject::CArchetypeRoot(ref root) => {
                let archetype = ArchetypeId::new(root.archetype_id.value.clone());
                terminology::add_archetype_root(
                    &mut self.terminologies,
                    root,
                    &self.language.clone(),
                );
                let inner = Scope {
                    archetype: &archetype,
                    root: object,
                };
                let children = self.attributes(&root.attributes, inner, node_path, next_depth)?;
                Ok((
                    ConstraintPayload::Structure,
                    children,
                    archetype.clone(),
                    Some(archetype),
                ))
            }
            opt::CObject::CComplexObject(_) | opt::CObject::TComplexObject(_) => {
                let children =
                    self.attributes(path::attributes(object), scope, node_path, next_depth)?;
                Ok((
                    ConstraintPayload::Structure,
                    children,
                    scope.archetype.clone(),
                    None,
                ))
            }
            opt::CObject::CPrimitiveObject(ref primitive) => Ok((
                payload::primitive(primitive, node_path)?,
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::CCodePhrase(ref phrase) => Ok((
                ConstraintPayload::Coded(payload::code_phrase_constraint(phrase)),
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::CCodeReference(ref reference) => Ok((
                ConstraintPayload::Coded(payload::code_reference_constraint(reference)),
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::ConstraintRef(ref reference) => {
                let bindings = self.constraint_bindings(scope.archetype, &reference.reference);
                Ok((
                    ConstraintPayload::Coded(payload::constraint_ref(reference, bindings)),
                    Vec::new(),
                    scope.archetype.clone(),
                    None,
                ))
            }
            opt::CObject::CDvOrdinal(ref ordinal) => Ok((
                ConstraintPayload::Ordinal(payload::ordinal(ordinal)),
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::CDvQuantity(ref quantity) => Ok((
                ConstraintPayload::Quantity(payload::quantity(quantity)),
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::CDvState(ref state) => Ok((
                ConstraintPayload::State(payload::state(state)),
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::ArchetypeSlot(ref slot_object) => Ok((
                open_slot(slot_object, occurrences, node_path)?,
                Vec::new(),
                scope.archetype.clone(),
                None,
            )),
            opt::CObject::CDefinedObject(_) => Err(ReadError::UninterpretableConstraint {
                path: node_path.to_owned(),
                rm_type: path::rm_type(object).to_owned(),
                constrainer: "C_DEFINED_OBJECT".to_owned(),
            }),
            opt::CObject::ArchetypeInternalRef(_) => Err(ReadError::UninterpretableConstraint {
                path: node_path.to_owned(),
                rm_type: path::rm_type(object).to_owned(),
                constrainer: "ARCHETYPE_INTERNAL_REF".to_owned(),
            }),
        }
    }

    /// Resolves a `use_node` internal reference against the archetype root it
    /// sits in, and walks the target in its place.
    ///
    /// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 replaces every
    /// `use_node` reference with an inline copy of its target, but the ADL 1.4
    /// flattener is a vendor tool and does not always do so. The reference's
    /// own node identifier and occurrences win over the target's, because they
    /// are what the template states at this point.
    #[expect(
        clippy::too_many_arguments,
        reason = "the resolution takes the same context the walk threads, plus the reference it is expanding"
    )]
    fn internal_reference(
        &mut self,
        reference: &opt::ArchetypeInternalRef,
        attribute_name: &str,
        ordinal: usize,
        context: AttributeContext,
        scope: Scope<'_>,
        node_path: &str,
        depth: usize,
    ) -> Result<ConstraintNode, ReadError> {
        let occurrences = payload::multiplicity(&reference.occurrences, node_path)?;
        let steps = path::parse(&reference.target_path);
        let target = path::resolve(scope.root, &steps).ok_or_else(|| {
            ReadError::UnresolvedInternalReference {
                path: node_path.to_owned(),
                target: reference.target_path.clone(),
            }
        })?;
        let expanded = self
            .object(
                target,
                attribute_name,
                ordinal,
                context,
                scope,
                node_path,
                depth.saturating_add(1),
            )?
            .ok_or_else(|| ReadError::UnresolvedInternalReference {
                path: node_path.to_owned(),
                target: reference.target_path.clone(),
            })?;

        Ok(ConstraintNode::new(
            NodeIdentity::new(
                RmAttributeName::new(attribute_name.to_owned()),
                local_code(&reference.node_id),
                expanded.identity().archetype_id().cloned(),
                RmTypeName::new(reference.rm_type_name.clone()),
                expanded.identity().pinned_name().map(str::to_owned),
                ordinal,
            ),
            occurrences,
            true,
            context,
            expanded.payload().clone(),
            expanded.terminology_scope().clone(),
            false,
            expanded.default_value().cloned(),
            expanded.children().to_vec(),
        ))
    }

    /// The bindings the archetype that owns `code` states for it.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3 puts
    /// `constraint_bindings` in an `ARCHETYPE_ONTOLOGY`, which belongs to one
    /// archetype, so only that archetype's terminology is consulted. A binding
    /// stated nowhere is absent here rather than guessed at from another
    /// archetype's.
    fn constraint_bindings(
        &self,
        archetype: &ArchetypeId,
        code: &str,
    ) -> BTreeMap<TerminologyName, String> {
        let code = LocalCode::new(code.to_owned());
        let mut found = BTreeMap::new();
        if let Some(terminology) = self.terminologies.get(archetype)
            && let Some(bindings) = terminology.constraint_bindings(&code)
        {
            for (terminology_name, term) in bindings {
                found.insert(terminology_name.clone(), term.value().to_owned());
            }
        }
        found
    }
}

/// The payload of an unfilled archetype slot.
///
/// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 removes only `closed`
/// slots and inlines every filler, so an open slot is a legitimate runtime
/// extension point that survives into an operational template. What goes there
/// is undetermined, so a form renders nothing for it.
fn open_slot(
    slot_object: &opt::ArchetypeSlot,
    occurrences: Multiplicity,
    node_path: &str,
) -> Result<ConstraintPayload, ReadError> {
    // Where the occurrences make that undetermined content mandatory, no form
    // this compiler produces can satisfy the template, so the template is
    // refused rather than guessed at.
    if occurrences.is_mandatory() {
        return Err(ReadError::RequiredSlotUnfilled {
            path: node_path.to_owned(),
            node_id: slot_object.node_id.clone(),
            rm_type: slot_object.rm_type_name.clone(),
            occurrences,
        });
    }
    Ok(ConstraintPayload::OpenSlot(OpenSlot {
        includes: slot_object
            .includes
            .iter()
            .flat_map(slot::assertion)
            .collect(),
        excludes: slot_object
            .excludes
            .iter()
            .flat_map(slot::assertion)
            .collect(),
    }))
}

/// The Reference Model type name, with a foundation type recorded under the
/// openEHR BASE spelling.
fn rm_type_name(object: &opt::CObject) -> String {
    let stated = path::rm_type(object);
    match *object {
        opt::CObject::CPrimitiveObject(_) => {
            payload::foundation_type_name(stated).map_or_else(|| stated.to_owned(), str::to_owned)
        }
        _ => stated.to_owned(),
    }
}

fn attribute_context(
    attribute: &opt::CAttribute,
    path: &str,
) -> Result<AttributeContext, ReadError> {
    Ok(match *attribute {
        opt::CAttribute::CSingleAttribute(ref single) => {
            AttributeContext::single(Some(payload::multiplicity(&single.existence, path)?))
        }
        opt::CAttribute::CMultipleAttribute(ref multiple) => AttributeContext::container(
            Some(payload::multiplicity(&multiple.existence, path)?),
            Some(Cardinality::new(
                payload::multiplicity(&multiple.cardinality.interval, path)?,
                multiple.cardinality.is_ordered,
                multiple.cardinality.is_unique,
            )),
        ),
    })
}
