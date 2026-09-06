// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! ADL 1.4 constrainer classes to the internal constraint payload.

use openehr_its::opt14::types as opt;

use crate::error::ReadError;
use crate::model::ids::TerminologyName;
use crate::model::multiplicity::{Bounds, Multiplicity};
use crate::model::payload::{
    BooleanConstraint, CodeSource, CodedConstraint, CodedValue, ConstraintPayload, ExternalSet,
    IntegerConstraint, OrdinalConstraint, OrdinalOption, QuantityConstraint, QuantityUnit,
    RealConstraint, StateConstraint, StateNode, StateTransition, TemporalConstraint,
    TextConstraint, TimezoneValidity,
};
use std::collections::BTreeMap;

/// The openEHR BASE foundation-type name for an ADL 1.4 `C_PRIMITIVE_OBJECT`
/// `rm_type_name`.
///
/// A real `.opt` spells the primitive types in upper case (`STRING`,
/// `INTEGER`), while openEHR BASE Release-1.2.0 `foundation_types.html` names
/// the classes `String`, `Integer`, `Real`, `Boolean`, `Character` and the
/// `Iso8601_*` family, which is also what an ADL 2 artefact carries. The
/// internal model records the BASE spelling, so an RM type name is not a trace
/// of which generation produced the node.
pub(crate) fn foundation_type_name(rm_type: &str) -> Option<&'static str> {
    match rm_type {
        "STRING" | "String" => Some("String"),
        "INTEGER" | "Integer" => Some("Integer"),
        "REAL" | "Real" => Some("Real"),
        "BOOLEAN" | "Boolean" => Some("Boolean"),
        "CHARACTER" | "Character" => Some("Character"),
        "DATE" | "Iso8601_date" => Some("Iso8601_date"),
        "TIME" | "Iso8601_time" => Some("Iso8601_time"),
        "DATE_TIME" | "Iso8601_date_time" => Some("Iso8601_date_time"),
        "DURATION" | "Iso8601_duration" => Some("Iso8601_duration"),
        _ => None,
    }
}

/// Converts a count interval, refusing a bound that is not a count.
pub(crate) fn multiplicity(
    interval: &opt::Intervalofinteger,
    path: &str,
) -> Result<Multiplicity, ReadError> {
    let count = |raw: i32| -> Result<u32, ReadError> {
        u32::try_from(raw).map_err(|_| ReadError::NegativeMultiplicity {
            path: path.to_owned(),
            bound: raw,
        })
    };
    let lower = if interval.lower_unbounded {
        0
    } else {
        let stated = count(interval.lower.unwrap_or(0))?;
        // An open lower end admits everything above it, so the first count it
        // admits is one more.
        if interval.lower_included == Some(false) {
            stated.saturating_add(1)
        } else {
            stated
        }
    };
    let upper = if interval.upper_unbounded {
        None
    } else {
        match interval.upper {
            None => None,
            Some(raw) => {
                let stated = count(raw)?;
                Some(if interval.upper_included == Some(false) {
                    stated.saturating_sub(1)
                } else {
                    stated
                })
            }
        }
    };
    Ok(match upper {
        Some(upper) => Multiplicity::bounded(lower, upper),
        None => Multiplicity::unbounded_from(lower),
    })
}

fn real_bounds(interval: &opt::Intervalofreal) -> Bounds<f64> {
    Bounds::new(
        if interval.lower_unbounded {
            None
        } else {
            interval.lower
        },
        if interval.upper_unbounded {
            None
        } else {
            interval.upper
        },
        interval.lower_included != Some(false),
        interval.upper_included != Some(false),
    )
}

fn integer_bounds(interval: &opt::Intervalofinteger) -> Bounds<i64> {
    Bounds::new(
        if interval.lower_unbounded {
            None
        } else {
            interval.lower.map(i64::from)
        },
        if interval.upper_unbounded {
            None
        } else {
            interval.upper.map(i64::from)
        },
        interval.lower_included != Some(false),
        interval.upper_included != Some(false),
    )
}

macro_rules! lexical_bounds {
    ($name:ident, $ty:ty) => {
        fn $name(interval: &$ty) -> Bounds<String> {
            Bounds::new(
                if interval.lower_unbounded {
                    None
                } else {
                    interval.lower.clone()
                },
                if interval.upper_unbounded {
                    None
                } else {
                    interval.upper.clone()
                },
                interval.lower_included != Some(false),
                interval.upper_included != Some(false),
            )
        }
    };
}

lexical_bounds!(date_bounds, opt::Intervalofdate);
lexical_bounds!(time_bounds, opt::Intervaloftime);
lexical_bounds!(date_time_bounds, opt::Intervalofdatetime);
lexical_bounds!(duration_bounds, opt::Intervalofduration);

fn timezone_validity(kind: Option<opt::ValidityKind>) -> Option<TimezoneValidity> {
    kind.map(|kind| match kind {
        opt::ValidityKind::Mandatory => TimezoneValidity::Mandatory,
        opt::ValidityKind::Optional => TimezoneValidity::Optional,
        opt::ValidityKind::Disallowed => TimezoneValidity::Disallowed,
    })
}

pub(crate) fn code_phrase(
    phrase: &openehr_base::v1_3::base_types::terminology::code_phrase::CodePhrase,
) -> CodedValue {
    CodedValue::new(
        TerminologyName::new(phrase.terminology_id.value.clone()),
        phrase.code_string.clone(),
        phrase.preferred_term.clone(),
    )
}

fn rm_code_phrase(
    phrase: &openehr_rm::v1_2::data_types::text::code_phrase::CodePhrase,
) -> CodedValue {
    CodedValue::new(
        TerminologyName::new(phrase.terminology_id.value.clone()),
        phrase.code_string.clone(),
        phrase.preferred_term.clone(),
    )
}

/// The payload of a `C_PRIMITIVE_OBJECT`.
///
/// The constraint the template states decides the payload; the
/// `rm_type_name` decides it only when the object constrains nothing, which
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.7 calls `any_allowed`.
pub(crate) fn primitive(
    object: &opt::CPrimitiveObject,
    path: &str,
) -> Result<ConstraintPayload, ReadError> {
    let Some(item) = object.item.as_deref() else {
        return unconstrained_primitive(&object.rm_type_name, path);
    };
    Ok(match *item {
        opt::CPrimitive::CBoolean(ref c) => ConstraintPayload::Boolean(BooleanConstraint {
            true_valid: c.true_valid,
            false_valid: c.false_valid,
            assumed_value: c.assumed_value,
        }),
        opt::CPrimitive::CInteger(ref c) => ConstraintPayload::Integer(IntegerConstraint {
            list: c.list.iter().copied().map(i64::from).collect(),
            ranges: c.range.as_ref().map(integer_bounds).into_iter().collect(),
            assumed_value: c.assumed_value.map(i64::from),
        }),
        opt::CPrimitive::CReal(ref c) => ConstraintPayload::Real(RealConstraint {
            list: c.list.clone(),
            ranges: c.range.as_ref().map(real_bounds).into_iter().collect(),
            assumed_value: c.assumed_value,
        }),
        opt::CPrimitive::CString(ref c) => ConstraintPayload::Text(TextConstraint {
            patterns: c.pattern.clone().into_iter().collect(),
            list: c.list.clone(),
            list_open: c.list_open,
            assumed_value: c.assumed_value.clone(),
        }),
        opt::CPrimitive::CDate(ref c) => ConstraintPayload::Date(TemporalConstraint {
            pattern: c.pattern.clone(),
            ranges: c.range.as_ref().map(date_bounds).into_iter().collect(),
            timezone_validity: timezone_validity(c.timezone_validity),
            assumed_value: c.assumed_value.clone(),
        }),
        opt::CPrimitive::CTime(ref c) => ConstraintPayload::Time(TemporalConstraint {
            pattern: c.pattern.clone(),
            ranges: c.range.as_ref().map(time_bounds).into_iter().collect(),
            timezone_validity: timezone_validity(c.timezone_validity),
            assumed_value: c.assumed_value.clone(),
        }),
        opt::CPrimitive::CDateTime(ref c) => ConstraintPayload::DateTime(TemporalConstraint {
            pattern: c.pattern.clone(),
            ranges: c.range.as_ref().map(date_time_bounds).into_iter().collect(),
            timezone_validity: timezone_validity(c.timezone_validity),
            assumed_value: c.assumed_value.clone(),
        }),
        opt::CPrimitive::CDuration(ref c) => ConstraintPayload::Duration(TemporalConstraint {
            pattern: c.pattern.clone(),
            ranges: c.range.as_ref().map(duration_bounds).into_iter().collect(),
            timezone_validity: None,
            assumed_value: c.assumed_value.clone(),
        }),
    })
}

fn unconstrained_primitive(rm_type: &str, path: &str) -> Result<ConstraintPayload, ReadError> {
    let empty_temporal = TemporalConstraint {
        pattern: None,
        ranges: Vec::new(),
        timezone_validity: None,
        assumed_value: None,
    };
    Ok(match foundation_type_name(rm_type) {
        Some("Boolean") => ConstraintPayload::Boolean(BooleanConstraint {
            true_valid: true,
            false_valid: true,
            assumed_value: None,
        }),
        Some("Integer") => ConstraintPayload::Integer(IntegerConstraint {
            list: Vec::new(),
            ranges: Vec::new(),
            assumed_value: None,
        }),
        Some("Real") => ConstraintPayload::Real(RealConstraint {
            list: Vec::new(),
            ranges: Vec::new(),
            assumed_value: None,
        }),
        Some("String" | "Character") => ConstraintPayload::Text(TextConstraint {
            patterns: Vec::new(),
            list: Vec::new(),
            list_open: None,
            assumed_value: None,
        }),
        Some("Iso8601_date") => ConstraintPayload::Date(empty_temporal),
        Some("Iso8601_time") => ConstraintPayload::Time(empty_temporal),
        Some("Iso8601_date_time") => ConstraintPayload::DateTime(empty_temporal),
        Some("Iso8601_duration") => ConstraintPayload::Duration(empty_temporal),
        _ => {
            return Err(ReadError::UninterpretableConstraint {
                path: path.to_owned(),
                rm_type: rm_type.to_owned(),
                constrainer: "C_PRIMITIVE_OBJECT with no item".to_owned(),
            });
        }
    })
}

/// The payload of a `C_CODE_PHRASE`.
pub(crate) fn code_phrase_constraint(constraint: &opt::CCodePhrase) -> CodedConstraint {
    let terminology = constraint
        .terminology_id
        .as_ref()
        .map(|id| TerminologyName::new(id.value.clone()));
    let source = match (terminology, constraint.code_list.is_empty()) {
        (Some(terminology), false) => CodeSource::Enumerated {
            terminology,
            codes: constraint.code_list.clone(),
        },
        (Some(terminology), true) => CodeSource::OpenTerminology { terminology },
        (None, _) => CodeSource::Unconstrained,
    };
    CodedConstraint {
        source,
        assumed_value: constraint.assumed_value.as_ref().map(code_phrase),
        // ADL 1.4 has no `constraint_status`: the ITS-XML `OpenehrProfile.xsd`
        // `C_CODE_PHRASE` declares only `terminology_id`, `code_list` and
        // `assumed_value`.
        status: None,
    }
}

/// The payload of a `C_CODE_REFERENCE`.
///
/// `C_CODE_REFERENCE` is declared only in the ITS-XML `Template.xsd`; neither
/// AOM 1.4 nor AOM 2 defines it. Its `referenceSetUri` names a set a
/// terminology server expands.
pub(crate) fn code_reference_constraint(constraint: &opt::CCodeReference) -> CodedConstraint {
    CodedConstraint {
        source: CodeSource::External(ExternalSet::ReferenceSet {
            uri: constraint.referenceSetUri.clone(),
        }),
        assumed_value: constraint.assumed_value.as_ref().map(code_phrase),
        status: None,
    }
}

/// The payload of a `CONSTRAINT_REF`, with the bindings the archetype states
/// for the `ac`-code it names.
pub(crate) fn constraint_ref(
    constraint: &opt::ConstraintRef,
    bindings: BTreeMap<TerminologyName, String>,
) -> CodedConstraint {
    CodedConstraint {
        source: CodeSource::External(ExternalSet::ConstraintCode {
            code: crate::model::ids::LocalCode::new(constraint.reference.clone()),
            bindings,
            operational_terminology: None,
        }),
        assumed_value: None,
        status: None,
    }
}

/// The payload of a `C_DV_ORDINAL`.
pub(crate) fn ordinal(constraint: &opt::CDvOrdinal) -> OrdinalConstraint {
    let option =
        |value: &openehr_rm::v1_2::data_types::quantity::dv_ordinal::DvOrdinal| OrdinalOption {
            value: f64::from(value.value),
            symbol: rm_code_phrase(&value.symbol.defining_code),
        };
    OrdinalConstraint {
        options: constraint.list.iter().map(option).collect(),
        assumed_value: constraint.assumed_value.as_ref().map(option),
    }
}

/// The payload of a `C_DV_QUANTITY`.
pub(crate) fn quantity(constraint: &opt::CDvQuantity) -> QuantityConstraint {
    QuantityConstraint {
        property: constraint.property.as_ref().map(code_phrase),
        units: constraint
            .list
            .iter()
            .map(|item| QuantityUnit {
                units: item.units.clone(),
                magnitude: item.magnitude.as_ref().map(real_bounds),
                precision: item.precision.as_ref().map(integer_bounds),
            })
            .collect(),
        assumed_value: constraint
            .assumed_value
            .as_ref()
            .map(|value| (value.magnitude, value.units.clone())),
    }
}

/// The payload of a `C_DV_STATE`.
pub(crate) fn state(constraint: &opt::CDvState) -> StateConstraint {
    fn state_name(state: &opt::State) -> String {
        match *state {
            opt::State::NonTerminalState(ref s) => s.name.clone(),
            opt::State::TerminalState(ref s) => s.name.clone(),
        }
    }
    let node = |state: &opt::State| match *state {
        opt::State::TerminalState(ref s) => StateNode {
            name: s.name.clone(),
            is_terminal: true,
            transitions: Vec::new(),
        },
        opt::State::NonTerminalState(ref s) => StateNode {
            name: s.name.clone(),
            is_terminal: false,
            transitions: s
                .transitions
                .iter()
                .map(|t| StateTransition {
                    event: t.event.clone(),
                    action: t.action.clone(),
                    guard: t.guard.clone(),
                    next_state: t.next_state.as_deref().map(state_name),
                })
                .collect(),
        },
    };
    StateConstraint {
        states: constraint.value.states.iter().map(node).collect(),
    }
}
