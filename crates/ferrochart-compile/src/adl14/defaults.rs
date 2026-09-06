// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The default values an ADL 1.4 operational template states.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.2 keeps a default
//! value distinct from an assumed value: "default values do appear in data,
//! while assumed values don't". An operational template states them in its
//! `constraints` section, as `T_COMPLEX_OBJECT.default_value` under a
//! `T_ATTRIBUTE` whose `differential_path` names the node they belong to.

use openehr_its::opt14::types as opt;
use openehr_rm::v1_2::data_types::basic::data_value::DataValue;
use openehr_rm::v1_2::data_types::text::dv_text::DvText;
use openehr_rm::v1_2::data_types::uri::dv_uri::DvUri;

use crate::adl14::path::{self, PathStep};
use crate::error::ReadError;
use crate::model::node::ConstraintNode;
use crate::model::payload::{CodedValue, DefaultValue};

/// Attaches every default value the template's `constraints` section states.
///
/// # Errors
/// [`ReadError::UnresolvedDefaultPath`] when a differential path names a node
/// the definition does not carry, which means the template contradicts itself.
pub(crate) fn apply(
    root: &mut ConstraintNode,
    constraints: Option<&opt::TConstraint>,
) -> Result<(), ReadError> {
    let Some(constraints) = constraints else {
        return Ok(());
    };
    for attribute in &constraints.attributes {
        let steps = path::parse(&attribute.differential_path);
        for child in &attribute.children {
            let Some(value) = child.default_value.as_ref() else {
                continue;
            };
            let target =
                resolve(root, &steps, &attribute.rm_attribute_name, child).ok_or_else(|| {
                    ReadError::UnresolvedDefaultPath {
                        path: format!(
                            "{}/{}",
                            attribute.differential_path, attribute.rm_attribute_name
                        ),
                    }
                })?;
            target.set_default_value(from_data_value(value));
        }
    }
    Ok(())
}

fn resolve<'a>(
    root: &'a mut ConstraintNode,
    steps: &[PathStep],
    attribute: &str,
    child: &opt::TComplexObject,
) -> Option<&'a mut ConstraintNode> {
    let mut current = root;
    for step in steps {
        let Some(ref attribute_name) = step.attribute else {
            if !step_matches(current, step) {
                return None;
            }
            continue;
        };
        current = current.children_mut().iter_mut().find(|node| {
            node.identity().rm_attribute().as_str() == attribute_name && step_matches(node, step)
        })?;
    }
    current.children_mut().iter_mut().find(|node| {
        node.identity().rm_attribute().as_str() == attribute
            && node.identity().rm_type().as_str() == child.rm_type_name
            && (child.node_id.is_empty()
                || node
                    .identity()
                    .node_id()
                    .map(crate::model::ids::LocalCode::as_str)
                    == Some(child.node_id.as_str()))
    })
}

fn step_matches(node: &ConstraintNode, step: &PathStep) -> bool {
    if let Some(ref wanted) = step.node_id
        && node
            .identity()
            .node_id()
            .map(crate::model::ids::LocalCode::as_str)
            != Some(wanted.as_str())
    {
        return false;
    }
    if let Some(ref wanted) = step.archetype_id
        && node
            .identity()
            .archetype_id()
            .map(crate::model::ids::ArchetypeId::as_str)
            != Some(wanted.as_str())
    {
        return false;
    }
    if let Some(ref wanted) = step.name
        && node.identity().pinned_name() != Some(wanted.as_str())
    {
        return false;
    }
    true
}

fn coded(text: &openehr_rm::v1_2::data_types::text::dv_coded_text::DvCodedText) -> CodedValue {
    CodedValue::new(
        crate::model::ids::TerminologyName::new(text.defining_code.terminology_id.value.clone()),
        text.defining_code.code_string.clone(),
        text.defining_code.preferred_term.clone(),
    )
}

/// Reads a Reference Model data value into the model's generation-blind
/// default.
///
/// A value this reader does not model becomes [`DefaultValue::Opaque`] naming
/// its Reference Model class, so a template's default is never dropped.
fn from_data_value(value: &DataValue) -> DefaultValue {
    match *value {
        DataValue::DvBoolean(ref v) => DefaultValue::Boolean(v.value),
        DataValue::DvCount(ref v) => DefaultValue::Integer(v.magnitude),
        DataValue::DvText(DvText::DvText(ref v)) => DefaultValue::Text(v.value.clone()),
        DataValue::DvText(DvText::DvCodedText(ref v)) => DefaultValue::Coded {
            code: coded(v),
            rubric: Some(v.value.clone()),
        },
        DataValue::DvUri(DvUri::DvUri(ref v)) => DefaultValue::Text(v.value.clone()),
        DataValue::DvUri(DvUri::DvEhrUri(ref v)) => DefaultValue::Text(v.value.clone()),
        DataValue::DvQuantity(ref v) => DefaultValue::Quantity {
            magnitude: v.magnitude,
            units: v.units.clone(),
            precision: v.precision,
        },
        DataValue::DvOrdinal(ref v) => DefaultValue::Ordinal {
            value: f64::from(v.value),
            symbol: coded(&v.symbol),
        },
        DataValue::DvScale(ref v) => DefaultValue::Ordinal {
            value: v.value,
            symbol: coded(&v.symbol),
        },
        DataValue::DvDate(ref v) => DefaultValue::Temporal(v.value.clone()),
        DataValue::DvTime(ref v) => DefaultValue::Temporal(v.value.clone()),
        DataValue::DvDateTime(ref v) => DefaultValue::Temporal(v.value.clone()),
        DataValue::DvDuration(ref v) => DefaultValue::Temporal(v.value.clone()),
        DataValue::DvIdentifier(ref v) => DefaultValue::Text(v.id.clone()),
        DataValue::DvParagraph(_) => DefaultValue::Opaque("DV_PARAGRAPH".to_owned()),
        DataValue::DvParsable(ref v) => DefaultValue::Text(v.value.clone()),
        DataValue::DvProportion(_) => DefaultValue::Opaque("DV_PROPORTION".to_owned()),
        DataValue::DvMultimedia(_) => DefaultValue::Opaque("DV_MULTIMEDIA".to_owned()),
        DataValue::DvState(_) => DefaultValue::Opaque("DV_STATE".to_owned()),
        DataValue::DvInterval(_) => DefaultValue::Opaque("DV_INTERVAL".to_owned()),
        DataValue::DvGeneralTimeSpecification(_) => {
            DefaultValue::Opaque("DV_GENERAL_TIME_SPECIFICATION".to_owned())
        }
        DataValue::DvPeriodicTimeSpecification(_) => {
            DefaultValue::Opaque("DV_PERIODIC_TIME_SPECIFICATION".to_owned())
        }
    }
}
