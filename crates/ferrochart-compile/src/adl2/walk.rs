// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The walk from a generated ADL 2 operational template to the internal model.

use std::collections::BTreeMap;

use openehr_adl::validate::rm::{ProductionRmModel, RmModel};
use openehr_am::v2_4::aom2::archetype::operational_template::OperationalTemplate;
use openehr_am::v2_4::aom2::constraint_model::c_archetype_root::CArchetypeRoot;
use openehr_am::v2_4::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::v2_4::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::v2_4::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::v2_4::aom2::constraint_model::c_object::CObject;
use openehr_am::v2_4::aom2::rules::expr_constraint::ExprConstraint;
use openehr_am::v2_4::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::v2_4::beom::core::assertion::Assertion;
use openehr_am::v2_4::beom::core::expression::Expression;
use openehr_base::v1_3::foundation_types::interval::multiplicity_interval::MultiplicityInterval;

use crate::adl2::{payload, terminology};
use crate::error::{MAX_DEPTH, ReadError};
use crate::model::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId,
};
use crate::model::multiplicity::{AttributeContext, Cardinality};
use crate::model::node::{ConstraintNode, ConstraintTemplate, NodeIdentity};
use crate::model::payload::{ConstraintPayload, DefaultValue, OpenSlot, SlotAssertion};
use crate::model::terminology::Terminology;

/// The state one walk carries down the tree.
struct Walk<'a> {
    template: &'a OperationalTemplate,
    terminologies: BTreeMap<ArchetypeId, Terminology>,
}

/// The archetype whose terminology governs the node the walk is at.
#[derive(Clone, Copy)]
struct Scope<'a> {
    archetype: &'a ArchetypeId,
    terminology: Option<&'a ArchetypeTerminology>,
}

pub(crate) fn read(template: &OperationalTemplate) -> Result<ConstraintTemplate, ReadError> {
    let root_archetype = ArchetypeId::new(template.archetype_id.physical_id());
    let walk = Walk {
        template,
        terminologies: terminology::read(template),
    };
    let scope = Scope {
        archetype: &root_archetype,
        terminology: Some(&template.terminology),
    };
    let root_path = format!("[{root_archetype}]");

    let parts = complex_parts(&template.definition);
    let occurrences_stated = parts.occurrences.is_some();
    let occurrences = match parts.occurrences {
        Some(interval) => payload::multiplicity(interval, &root_path)?,
        None => crate::model::multiplicity::Multiplicity::exactly(1),
    };

    let (folded, children) = walk.body(
        parts.attributes,
        parts.tuples,
        parts.rm_type,
        scope,
        &root_path,
        1,
    )?;
    let root = ConstraintNode::new(
        NodeIdentity::new(
            RmAttributeName::new(String::new()),
            local_code(parts.node_id),
            Some(root_archetype.clone()),
            RmTypeName::new(parts.rm_type.to_owned()),
            pinned_name(parts.attributes),
            0,
        ),
        occurrences,
        occurrences_stated,
        AttributeContext::single(None),
        folded,
        root_archetype,
        false,
        parts.default_value.map(json_default),
        children,
    );

    Ok(ConstraintTemplate::new(
        TemplateId::new(template.archetype_id.physical_id()),
        LanguageTag::new(template.original_language.code_string.clone()),
        root,
        walk.terminologies,
    ))
}

fn local_code(node_id: &str) -> Option<LocalCode> {
    (!node_id.is_empty() && node_id != "Primitive_node_id")
        .then(|| LocalCode::new(node_id.to_owned()))
}

/// The parts a `C_COMPLEX_OBJECT` carries, whether or not it is an archetype
/// root.
struct ComplexParts<'a> {
    node_id: &'a str,
    rm_type: &'a str,
    occurrences: Option<&'a MultiplicityInterval>,
    attributes: &'a [CAttribute],
    tuples: &'a [CAttributeTuple],
    default_value: Option<&'a serde_json::Value>,
}

fn complex_parts(object: &CComplexObject) -> ComplexParts<'_> {
    match *object {
        CComplexObject::CArchetypeRoot(ref root) => root_parts(root),
        CComplexObject::CComplexObject(ref data) => ComplexParts {
            node_id: &data.node_id,
            rm_type: &data.rm_type_name,
            occurrences: data.occurrences.as_ref(),
            attributes: data.attributes.as_deref().unwrap_or_default(),
            tuples: data.attribute_tuples.as_deref().unwrap_or_default(),
            default_value: data.default_value.as_ref(),
        },
    }
}

fn root_parts(root: &CArchetypeRoot) -> ComplexParts<'_> {
    ComplexParts {
        node_id: &root.node_id,
        rm_type: &root.rm_type_name,
        occurrences: root.occurrences.as_ref(),
        attributes: root.attributes.as_deref().unwrap_or_default(),
        tuples: root.attribute_tuples.as_deref().unwrap_or_default(),
        default_value: root.default_value.as_ref(),
    }
}

/// The name the template pins on an object, where it constrains `name/value`
/// to exactly one string.
fn pinned_name(attributes: &[CAttribute]) -> Option<String> {
    let name = attributes
        .iter()
        .find(|attribute| attribute.rm_attribute_name == "name")?;
    let CObject::CComplexObject(ref text) = *name.children.as_deref()?.first()? else {
        return None;
    };
    let value = complex_parts(text)
        .attributes
        .iter()
        .find(|attribute| attribute.rm_attribute_name == "value")?;
    let CObject::CString(ref string) = *value.children.as_deref()?.first()? else {
        return None;
    };
    match string.constraint.as_deref() {
        Some([only]) => Some(only.clone()),
        _ => None,
    }
}

/// The multiplicity the reference model gives one attribute.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.2 infers unstated
/// occurrences from the containing attribute, and this is the reference-model
/// half of that rule.
fn rm_multiplicity(rm_type: &str, attribute_path: &str) -> MultiplicityInterval {
    let open = MultiplicityInterval {
        lower: Some(0),
        upper: None,
        lower_unbounded: false,
        upper_unbounded: true,
        lower_included: true,
        upper_included: false,
    };
    let name = attribute_path.rsplit('/').next().unwrap_or(attribute_path);
    match ProductionRmModel.attribute(rm_type, name) {
        Some(attribute) if attribute.is_multiple => attribute.cardinality.map_or(
            open,
            openehr_adl::aom::interval::Bounds::to_multiplicity_interval,
        ),
        Some(attribute) => attribute.existence.to_multiplicity_interval(),
        None => open,
    }
}

/// A default value an ADL 2 template states.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.5 types
/// `C_DEFINED_OBJECT.default_value` as an instance of the reference-model type
/// the node constrains, and publishes no serialisation for it, so the reader
/// keeps the payload as the text the template states rather than guessing at
/// its shape.
fn json_default(value: &serde_json::Value) -> DefaultValue {
    match *value {
        serde_json::Value::Bool(flag) => DefaultValue::Boolean(flag),
        serde_json::Value::String(ref text) => DefaultValue::Text(text.clone()),
        serde_json::Value::Number(ref number) => number.as_i64().map_or_else(
            || {
                number.as_f64().map_or_else(
                    || DefaultValue::Opaque(number.to_string()),
                    DefaultValue::Real,
                )
            },
            DefaultValue::Integer,
        ),
        _ => DefaultValue::Opaque(value.to_string()),
    }
}

impl Walk<'_> {
    /// The payload and children of one complex object, folding any tuple.
    fn body(
        &self,
        attributes: &[CAttribute],
        tuples: &[CAttributeTuple],
        owner_rm_type: &str,
        scope: Scope<'_>,
        parent_path: &str,
        depth: usize,
    ) -> Result<(ConstraintPayload, Vec<ConstraintNode>), ReadError> {
        // openEHR AM Release-2.3.0 AOM2.html section 4.3 §Tuple Constraints:
        // "The tuple constraint type replaces all domain-specific constraint
        // types defined in ADL/AOM 1.4, including C_DV_QUANTITY and
        // C_DV_ORDINAL", so those two shapes fold into the payload the ADL 1.4
        // domain types produce and any other pairing is carried whole.
        let read: Vec<payload::Tuple> = tuples.iter().filter_map(payload::tuple).collect();
        if read.len() > 1 {
            // One node holds one payload, so a node whose tuples would need
            // two is refused rather than compiled with one of them dropped.
            return Err(ReadError::UninterpretableConstraint {
                path: parent_path.to_owned(),
                rm_type: owner_rm_type.to_owned(),
                constrainer: "more than one C_ATTRIBUTE_TUPLE".to_owned(),
            });
        }
        let payload = match read.into_iter().next() {
            Some(payload::Tuple::Quantity(quantity)) => ConstraintPayload::Quantity(quantity),
            Some(payload::Tuple::Ordinal(ordinal)) => ConstraintPayload::Ordinal(ordinal),
            Some(payload::Tuple::Other(other)) => ConstraintPayload::Tuple(other),
            None => ConstraintPayload::Structure,
        };
        let children = self.attributes(attributes, owner_rm_type, scope, parent_path, depth)?;
        Ok((payload, children))
    }

    fn attributes(
        &self,
        attributes: &[CAttribute],
        owner_rm_type: &str,
        scope: Scope<'_>,
        parent_path: &str,
        depth: usize,
    ) -> Result<Vec<ConstraintNode>, ReadError> {
        let mut built = Vec::new();
        for attribute in attributes {
            let context = attribute_context(attribute, parent_path)?;
            let mut ordinal = 0_usize;
            for child in attribute.children.as_deref().unwrap_or_default() {
                let child_path = step_path(parent_path, &attribute.rm_attribute_name, child);
                if let Some(node) = self.object(
                    child,
                    attribute,
                    ordinal,
                    context,
                    owner_rm_type,
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

    #[expect(
        clippy::too_many_arguments,
        reason = "the walk threads the node's owning attribute, scope and path down the tree; a parameter struct would only rename them"
    )]
    fn object(
        &self,
        object: &CObject,
        owning: &CAttribute,
        ordinal: usize,
        context: AttributeContext,
        owner_rm_type: &str,
        scope: Scope<'_>,
        node_path: &str,
        depth: usize,
    ) -> Result<Option<ConstraintNode>, ReadError> {
        if depth > MAX_DEPTH {
            return Err(ReadError::TooDeep {
                path: node_path.to_owned(),
            });
        }
        let stated = object.occurrences().is_some();
        let effective =
            object.effective_occurrences(Some(owning), Some(owner_rm_type), &rm_multiplicity);
        let occurrences = payload::multiplicity(&effective, node_path)?;
        if occurrences.is_prohibited() {
            return Ok(None);
        }

        if let CObject::CComplexObjectProxy(ref proxy) = *object {
            // openEHR AM Release-2.3.0 OPT2.html section 3.3 replaces every
            // `use_node` proxy with an inline copy of its target while it
            // generates the operational template, so one still standing here
            // means the target was never found.
            return Err(ReadError::UnresolvedInternalReference {
                path: node_path.to_owned(),
                target: proxy.target_path.clone(),
            });
        }

        let (payload, children, archetype_id, scope_archetype, node_id, rm_type, pinned, default) =
            self.parts(object, scope, node_path, occurrences, depth)?;

        Ok(Some(ConstraintNode::new(
            NodeIdentity::new(
                RmAttributeName::new(owning.rm_attribute_name.clone()),
                local_code(&node_id),
                archetype_id,
                RmTypeName::new(rm_type),
                pinned,
                ordinal,
            ),
            occurrences,
            stated,
            context,
            payload,
            scope_archetype,
            object_is_deprecated(object),
            default,
            children,
        )))
    }

    #[expect(
        clippy::type_complexity,
        reason = "the tuple is the node's parts on their way into ConstraintNode::new, consumed immediately by the one caller"
    )]
    fn parts(
        &self,
        object: &CObject,
        scope: Scope<'_>,
        node_path: &str,
        occurrences: crate::model::multiplicity::Multiplicity,
        depth: usize,
    ) -> Result<
        (
            ConstraintPayload,
            Vec<ConstraintNode>,
            Option<ArchetypeId>,
            ArchetypeId,
            String,
            String,
            Option<String>,
            Option<DefaultValue>,
        ),
        ReadError,
    > {
        let next_depth = depth.saturating_add(1);
        match *object {
            CObject::CComplexObject(ref complex) => {
                let inner = complex_parts(complex);
                let archetype_id = match *complex {
                    CComplexObject::CArchetypeRoot(ref root) => {
                        Some(ArchetypeId::new(root.archetype_ref.clone()))
                    }
                    CComplexObject::CComplexObject(_) => None,
                };
                let inner_scope = match archetype_id {
                    Some(ref id) => Scope {
                        archetype: id,
                        terminology: self
                            .template
                            .component_terminologies
                            .as_ref()
                            .and_then(|map| map.get(id.as_str())),
                    },
                    None => scope,
                };
                let (payload, children) = self.body(
                    inner.attributes,
                    inner.tuples,
                    inner.rm_type,
                    inner_scope,
                    node_path,
                    next_depth,
                )?;
                Ok((
                    payload,
                    children,
                    archetype_id.clone(),
                    inner_scope.archetype.clone(),
                    inner.node_id.to_owned(),
                    inner.rm_type.to_owned(),
                    pinned_name(inner.attributes),
                    inner.default_value.map(json_default),
                ))
            }
            CObject::ArchetypeSlot(ref slot) => {
                if occurrences.is_mandatory() {
                    return Err(ReadError::RequiredSlotUnfilled {
                        path: node_path.to_owned(),
                        node_id: slot.node_id.clone(),
                        rm_type: slot.rm_type_name.clone(),
                        occurrences,
                    });
                }
                Ok((
                    ConstraintPayload::OpenSlot(OpenSlot {
                        includes: assertions(slot.includes.as_deref().unwrap_or_default()),
                        excludes: assertions(slot.excludes.as_deref().unwrap_or_default()),
                    }),
                    Vec::new(),
                    None,
                    scope.archetype.clone(),
                    slot.node_id.clone(),
                    slot.rm_type_name.clone(),
                    None,
                    None,
                ))
            }
            CObject::CComplexObjectProxy(ref proxy) => {
                Err(ReadError::UnresolvedInternalReference {
                    path: node_path.to_owned(),
                    target: proxy.target_path.clone(),
                })
            }
            _ => {
                let payload = payload::primitive(object, scope.terminology).ok_or_else(|| {
                    ReadError::UninterpretableConstraint {
                        path: node_path.to_owned(),
                        rm_type: object.rm_type_name().to_owned(),
                        constrainer: "C_OBJECT".to_owned(),
                    }
                })?;
                Ok((
                    payload,
                    Vec::new(),
                    None,
                    scope.archetype.clone(),
                    object.node_id().to_owned(),
                    rm_type_name(object),
                    None,
                    None,
                ))
            }
        }
    }
}

/// Reads the archetype-id constraints on a slot.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.9 says a slot assertion
/// takes the form `EXPR_ARCHETYPE_REF matches EXPR_ARCHETYPE_ID_CONSTRAINT`,
/// so that shape is read into a typed assertion and every other shape is kept
/// as text rather than dropped.
fn assertions(source: &[Assertion]) -> Vec<SlotAssertion> {
    source.iter().flat_map(one_assertion).collect()
}

fn one_assertion(assertion: &Assertion) -> Vec<SlotAssertion> {
    if let Some(read) = archetype_id_match(&assertion.expression) {
        return read;
    }
    vec![SlotAssertion::Opaque(
        assertion
            .string_expression
            .clone()
            .or_else(|| assertion.tag.clone())
            .unwrap_or_else(|| "an assertion this reader does not interpret".to_owned()),
    )]
}

fn archetype_id_match(expression: &Expression) -> Option<Vec<SlotAssertion>> {
    let Expression::ExprBinaryOperator(ref binary) = *expression else {
        return None;
    };
    let Expression::ExprConstraint(ExprConstraint::ExprArchetypeIdConstraint(ref constraint)) =
        *binary.right_operand
    else {
        return None;
    };
    let read: Vec<SlotAssertion> = constraint
        .item
        .constraint
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|pattern| {
            // ADL 2 writes a contained regexp with its delimiters; ADL 1.4
            // states the bare expression, so both readers record the bare one.
            let (patterns, literals) =
                payload::split_string_constraint(std::slice::from_ref(pattern));
            patterns.into_iter().chain(literals).next().map_or_else(
                || SlotAssertion::Opaque(pattern.clone()),
                SlotAssertion::ArchetypeIdPattern,
            )
        })
        .collect();
    (!read.is_empty()).then_some(read)
}

fn object_is_deprecated(object: &CObject) -> bool {
    match *object {
        CObject::ArchetypeSlot(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CBoolean(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(ref o)) => {
            o.is_deprecated.unwrap_or(false)
        }
        CObject::CComplexObject(CComplexObject::CComplexObject(ref o)) => {
            o.is_deprecated.unwrap_or(false)
        }
        CObject::CComplexObjectProxy(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CDate(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CDateTime(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CDuration(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CInteger(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CReal(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CString(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CTerminologyCode(ref o) => o.is_deprecated.unwrap_or(false),
        CObject::CTime(ref o) => o.is_deprecated.unwrap_or(false),
    }
}

fn step_path(parent: &str, attribute: &str, object: &CObject) -> String {
    let predicate =
        if let CObject::CComplexObject(CComplexObject::CArchetypeRoot(ref root)) = *object {
            Some(root.archetype_ref.clone())
        } else {
            let node_id = object.node_id();
            (!node_id.is_empty()).then(|| node_id.to_owned())
        };
    match predicate {
        Some(predicate) => format!("{parent}/{attribute}[{predicate}]"),
        None => format!("{parent}/{attribute}"),
    }
}

fn attribute_context(attribute: &CAttribute, path: &str) -> Result<AttributeContext, ReadError> {
    let existence = attribute
        .existence
        .as_ref()
        .map(|interval| payload::multiplicity(interval, path))
        .transpose()?;
    if attribute.is_multiple {
        let cardinality = attribute
            .cardinality
            .as_ref()
            .map(|cardinality| {
                Ok::<_, ReadError>(Cardinality::new(
                    payload::multiplicity(&cardinality.interval, path)?,
                    cardinality.is_ordered,
                    cardinality.is_unique,
                ))
            })
            .transpose()?;
        Ok(AttributeContext::container(existence, cardinality))
    } else {
        Ok(AttributeContext::single(existence))
    }
}

/// The Reference Model type name a node constrains.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 11 §Mapping RM Entities to AOM
/// Entities publishes the openEHR AOM profile mapping from the AOM built-in
/// type `TERMINOLOGY_CODE` to the reference model's `CODE_PHRASE`, so a
/// `C_TERMINOLOGY_CODE` node records the reference model's own class name and
/// an RM type name is not a trace of which generation produced the node.
fn rm_type_name(object: &CObject) -> String {
    match *object {
        CObject::CTerminologyCode(_) => "CODE_PHRASE".to_owned(),
        _ => object.rm_type_name().to_owned(),
    }
}
