// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! AOM 2 constrainer classes to the internal constraint payload.

use openehr_am::v2_4::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::v2_4::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::v2_4::aom2::constraint_model::c_object::CObject;
use openehr_am::v2_4::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::v2_4::aom2::constraint_model::primitive::c_terminology_code::CTerminologyCode;
use openehr_am::v2_4::aom2::constraint_model::primitive::constraint_status::ConstraintStatus;
use openehr_am::v2_4::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_base::v1_3::foundation_types::interval::interval::Interval;
use openehr_base::v1_3::foundation_types::interval::multiplicity_interval::MultiplicityInterval;
use openehr_base::v1_3::foundation_types::terminology::terminology_code::TerminologyCode;
use std::collections::BTreeMap;

use crate::error::ReadError;
use crate::model::ids::{LocalCode, RmAttributeName, TerminologyName};
use crate::model::multiplicity::{Bounds, Multiplicity};
use crate::model::payload::{
    BindingStatus, BooleanConstraint, CodeSource, CodedConstraint, CodedValue, ConstraintPayload,
    ExternalSet, IntegerConstraint, OrdinalConstraint, OrdinalOption, QuantityConstraint,
    QuantityUnit, RealConstraint, TemporalConstraint, TextConstraint, TupleConstraint,
};

/// The archetype-local terminology identifier.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 gives
/// `CODE_PHRASE.terminology_id` the value `local` for a code the archetype
/// defines itself, which is how an ADL 1.4 template spells an `at`-code's
/// terminology. ADL 2 leaves the terminology implicit on an `at`-code, so the
/// reader records the same `local` here and the two generations agree.
pub(crate) const LOCAL_TERMINOLOGY: &str = "local";

/// Converts an AOM 2 count interval.
pub(crate) fn multiplicity(
    interval: &MultiplicityInterval,
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
        if interval.lower_included {
            stated
        } else {
            stated.saturating_add(1)
        }
    };
    let upper = if interval.upper_unbounded {
        None
    } else {
        match interval.upper {
            None => None,
            Some(raw) => {
                let stated = count(raw)?;
                Some(if interval.upper_included {
                    stated
                } else {
                    stated.saturating_sub(1)
                })
            }
        }
    };
    Ok(match upper {
        Some(upper) => Multiplicity::bounded(lower, upper),
        None => Multiplicity::unbounded_from(lower),
    })
}

fn bounds<T: Clone>(interval: &Interval<T>) -> Bounds<T> {
    match *interval {
        Interval::PointInterval(ref point) => Bounds::new(
            point.lower.clone(),
            point.upper.clone(),
            point.lower_included,
            point.upper_included,
        ),
        Interval::ProperInterval(
            openehr_base::v1_3::foundation_types::interval::proper_interval::ProperInterval::ProperInterval(
                ref proper,
            ),
        ) => Bounds::new(
            if proper.lower_unbounded {
                None
            } else {
                proper.lower.clone()
            },
            if proper.upper_unbounded {
                None
            } else {
                proper.upper.clone()
            },
            proper.lower_included,
            proper.upper_included,
        ),
        // A count interval standing in for a value range constrains no value.
        Interval::ProperInterval(
            openehr_base::v1_3::foundation_types::interval::proper_interval::ProperInterval::MultiplicityInterval(
                _,
            ),
        ) => Bounds::new(None, None, true, true),
    }
}

/// The integer an AOM 2 `C_INTEGER.assumed_value` states.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.14 types the attribute as
/// a Real in the BMM even though the constraint is on an integer. Rust
/// saturates a float-to-integer cast, so a magnitude past the integer range
/// clamps rather than wrapping into a different number.
#[expect(
    clippy::cast_possible_truncation,
    reason = "AOM2 types C_INTEGER.assumed_value as Real; the cast saturates, so no value wraps into a different one"
)]
fn integer_assumption(value: f64) -> i64 {
    value as i64
}

/// Widens a count-sized integer bound to the real the model records.
#[expect(
    clippy::cast_precision_loss,
    reason = "an ordinal score and a quantity bound are clinical magnitudes; nothing near the 53-bit mantissa limit reaches here"
)]
fn widen(value: i64) -> f64 {
    value as f64
}

fn integer_bounds(interval: &Interval<i32>) -> Bounds<i64> {
    let read = bounds(interval);
    Bounds::new(
        read.lower().copied().map(i64::from),
        read.upper().copied().map(i64::from),
        read.lower_included(),
        read.upper_included(),
    )
}

fn real_bounds(interval: &Interval<f64>) -> Bounds<f64> {
    bounds(interval)
}

fn lexical_bounds<T: Clone>(
    interval: &Interval<T>,
    lexical: impl Fn(&T) -> String,
) -> Bounds<String> {
    let read = bounds(interval);
    Bounds::new(
        read.lower().map(&lexical),
        read.upper().map(&lexical),
        read.lower_included(),
        read.upper_included(),
    )
}

pub(crate) fn binding_status(status: Option<ConstraintStatus>) -> Option<BindingStatus> {
    status.map(|status| match status {
        ConstraintStatus::Required => BindingStatus::Required,
        ConstraintStatus::Extensible => BindingStatus::Extensible,
        ConstraintStatus::Preferred => BindingStatus::Preferred,
        ConstraintStatus::Example => BindingStatus::Example,
        ConstraintStatus::Other(value) => BindingStatus::Other(value),
    })
}

fn terminology_code(code: &TerminologyCode) -> CodedValue {
    let terminology = if code.terminology_id.is_empty() {
        LOCAL_TERMINOLOGY.to_owned()
    } else {
        code.terminology_id.clone()
    };
    CodedValue::new(
        TerminologyName::new(terminology),
        code.code_string.clone(),
        None,
    )
}

/// The payload of a `C_TERMINOLOGY_CODE`.
///
/// openEHR AM Release-2.3.0 `ADL2.html` section 7.13.5.1 spells the constraint
/// as a single code: an `at`-code fixes one value, an `ac`-code names a set the
/// archetype's terminology defines, and the operational `acN@terminology` form
/// pins the set to a terminology. An `ac`-code whose members the terminology
/// enumerates is read as the enumeration itself, which is the same
/// [`CodeSource::Enumerated`] an ADL 1.4 `C_CODE_PHRASE` with a `code_list`
/// produces.
pub(crate) fn terminology_code_constraint(
    constraint: &CTerminologyCode,
    terminology: Option<&ArchetypeTerminology>,
) -> CodedConstraint {
    let (code, operational_terminology) = match constraint.constraint.split_once('@') {
        Some((code, name)) => (code, Some(TerminologyName::new(name.to_owned()))),
        None => (constraint.constraint.as_str(), None),
    };
    let local = LocalCode::new(code.to_owned());
    let members = terminology
        .and_then(|terminology| terminology.value_sets.as_ref())
        .and_then(|sets| sets.get(code));

    let source = if let Some(set) = members {
        CodeSource::Enumerated {
            terminology: TerminologyName::new(LOCAL_TERMINOLOGY),
            codes: set.members.iter().cloned().collect(),
        }
    } else if code.is_empty() {
        CodeSource::Unconstrained
    } else if local.kind() == crate::model::ids::CodeKind::ValueSet {
        CodeSource::External(ExternalSet::ConstraintCode {
            bindings: constraint_bindings(terminology, code),
            code: local,
            operational_terminology,
        })
    } else {
        CodeSource::Enumerated {
            terminology: TerminologyName::new(LOCAL_TERMINOLOGY),
            codes: vec![code.to_owned()],
        }
    };

    CodedConstraint {
        source,
        assumed_value: constraint.assumed_value.as_ref().map(terminology_code),
        status: binding_status(constraint.constraint_status),
    }
}

fn constraint_bindings(
    terminology: Option<&ArchetypeTerminology>,
    code: &str,
) -> BTreeMap<TerminologyName, String> {
    let mut found = BTreeMap::new();
    if let Some(bindings) = terminology.and_then(|t| t.term_bindings.as_ref()) {
        for (name, entries) in bindings {
            if let Some(target) = entries.get(code) {
                found.insert(TerminologyName::new(name.clone()), target.clone());
            }
        }
    }
    found
}

/// The payload of a primitive `C_OBJECT`, where the object is one.
pub(crate) fn primitive(
    object: &CObject,
    terminology: Option<&ArchetypeTerminology>,
) -> Option<ConstraintPayload> {
    let as_primitive = match *object {
        CObject::CBoolean(ref c) => CPrimitiveObject::CBoolean(c.clone()),
        CObject::CDate(ref c) => CPrimitiveObject::CDate(c.clone()),
        CObject::CDateTime(ref c) => CPrimitiveObject::CDateTime(c.clone()),
        CObject::CDuration(ref c) => CPrimitiveObject::CDuration(c.clone()),
        CObject::CInteger(ref c) => CPrimitiveObject::CInteger(c.clone()),
        CObject::CReal(ref c) => CPrimitiveObject::CReal(c.clone()),
        CObject::CString(ref c) => CPrimitiveObject::CString(c.clone()),
        CObject::CTerminologyCode(ref c) => CPrimitiveObject::CTerminologyCode(c.clone()),
        CObject::CTime(ref c) => CPrimitiveObject::CTime(c.clone()),
        CObject::ArchetypeSlot(_)
        | CObject::CComplexObject(_)
        | CObject::CComplexObjectProxy(_) => {
            return None;
        }
    };
    Some(primitive_object(&as_primitive, terminology))
}

/// The payload of a `C_PRIMITIVE_OBJECT`.
pub(crate) fn primitive_object(
    object: &CPrimitiveObject,
    terminology: Option<&ArchetypeTerminology>,
) -> ConstraintPayload {
    match *object {
        CPrimitiveObject::CBoolean(ref c) => ConstraintPayload::Boolean(BooleanConstraint {
            // AOM 2 states the admitted booleans as a list rather than the two
            // flags of AOM 1.4, so an empty list admits both.
            true_valid: c
                .constraint
                .as_ref()
                .is_none_or(|list| list.contains(&true)),
            false_valid: c
                .constraint
                .as_ref()
                .is_none_or(|list| list.contains(&false)),
            assumed_value: c.assumed_value,
        }),
        CPrimitiveObject::CInteger(ref c) => ConstraintPayload::Integer(IntegerConstraint {
            list: Vec::new(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(integer_bounds)
                .collect(),
            assumed_value: c.assumed_value.map(integer_assumption),
        }),
        CPrimitiveObject::CReal(ref c) => ConstraintPayload::Real(RealConstraint {
            list: Vec::new(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(real_bounds)
                .collect(),
            assumed_value: c.assumed_value,
        }),
        CPrimitiveObject::CString(ref c) => {
            let (patterns, list) =
                split_string_constraint(c.constraint.as_deref().unwrap_or_default());
            ConstraintPayload::Text(TextConstraint {
                patterns,
                list,
                list_open: None,
                assumed_value: c.assumed_value.clone(),
            })
        }
        CPrimitiveObject::CDate(ref c) => ConstraintPayload::Date(TemporalConstraint {
            pattern: c.pattern_constraint.clone(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|interval| lexical_bounds(interval, |value| value.value.clone()))
                .collect(),
            timezone_validity: None,
            assumed_value: c.assumed_value.as_ref().map(|value| value.value.clone()),
        }),
        CPrimitiveObject::CTime(ref c) => ConstraintPayload::Time(TemporalConstraint {
            pattern: c.pattern_constraint.clone(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|interval| lexical_bounds(interval, |value| value.value.clone()))
                .collect(),
            timezone_validity: None,
            assumed_value: c.assumed_value.as_ref().map(|value| value.value.clone()),
        }),
        CPrimitiveObject::CDateTime(ref c) => ConstraintPayload::DateTime(TemporalConstraint {
            pattern: c.pattern_constraint.clone(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|interval| lexical_bounds(interval, |value| value.value.clone()))
                .collect(),
            timezone_validity: None,
            assumed_value: c.assumed_value.as_ref().map(|value| value.value.clone()),
        }),
        CPrimitiveObject::CDuration(ref c) => ConstraintPayload::Duration(TemporalConstraint {
            pattern: c.pattern_constraint.clone(),
            ranges: c
                .constraint
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|interval| lexical_bounds(interval, |value| value.value.clone()))
                .collect(),
            timezone_validity: None,
            assumed_value: c.assumed_value.as_ref().map(|value| value.value.clone()),
        }),
        CPrimitiveObject::CTerminologyCode(ref c) => {
            ConstraintPayload::Coded(terminology_code_constraint(c, terminology))
        }
    }
}

/// Splits an AOM 2 string constraint into its regular expressions and its
/// literals.
///
/// openEHR AM Release-2.3.0 `ADL2.html` section 4.5 writes a regular
/// expression as a contained regexp, delimited by `/` or `^`, in the same list
/// as the literal alternatives. The delimiters are stripped, so a pattern from
/// either generation reads the same.
pub(crate) fn split_string_constraint(constraint: &[String]) -> (Vec<String>, Vec<String>) {
    let mut patterns = Vec::new();
    let mut list = Vec::new();
    for item in constraint {
        match delimited_regex(item) {
            Some(inner) => patterns.push(inner.to_owned()),
            None => list.push(item.clone()),
        }
    }
    (patterns, list)
}

fn delimited_regex(item: &str) -> Option<&str> {
    ['/', '^'].into_iter().find_map(|delimiter| {
        item.strip_prefix(delimiter)
            .and_then(|rest| rest.strip_suffix(delimiter))
    })
}

/// One attribute tuple, read as a quantity or an ordinal where its members say
/// so.
///
/// openEHR AM Release-2.3.0 `AOM2.html` sections 4.5.25 and 4.5.26 tie sibling
/// attributes with a `C_ATTRIBUTE_TUPLE` whose rows are `C_PRIMITIVE_TUPLE`s.
/// A tuple over `magnitude` and `units` is a quantity; one over `symbol` and
/// `value` is an ordinal or a scale.
pub(crate) enum Tuple {
    /// The tuple constrains a quantity.
    Quantity(QuantityConstraint),
    /// The tuple constrains an ordinal or a scale.
    Ordinal(OrdinalConstraint),
    /// A pairing this reader does not fold, kept whole.
    Other(TupleConstraint),
}

/// Reads a tuple, returning nothing when it names no members at all.
pub(crate) fn tuple(attribute_tuple: &CAttributeTuple) -> Option<Tuple> {
    let members = attribute_tuple.members.as_deref()?;
    let index = |name: &str| {
        members
            .iter()
            .position(|member: &CAttribute| member.rm_attribute_name == name)
    };
    let rows = attribute_tuple.tuples.as_deref().unwrap_or_default();

    if let (Some(magnitude), Some(units)) = (index("magnitude"), index("units")) {
        let mut permitted = Vec::new();
        for row in rows {
            let Some(CPrimitiveObject::CString(unit)) = row.members.get(units) else {
                continue;
            };
            let magnitude_bounds = match row.members.get(magnitude) {
                Some(CPrimitiveObject::CReal(real)) => real
                    .constraint
                    .as_deref()
                    .and_then(<[Interval<f64>]>::first)
                    .map(real_bounds),
                Some(CPrimitiveObject::CInteger(integer)) => integer
                    .constraint
                    .as_deref()
                    .and_then(<[Interval<i32>]>::first)
                    .map(|interval| {
                        let read = integer_bounds(interval);
                        Bounds::new(
                            read.lower().copied().map(widen),
                            read.upper().copied().map(widen),
                            read.lower_included(),
                            read.upper_included(),
                        )
                    }),
                _ => None,
            };
            for symbol in unit.constraint.as_deref().unwrap_or_default() {
                permitted.push(QuantityUnit {
                    units: symbol.clone(),
                    magnitude: magnitude_bounds.clone(),
                    precision: None,
                });
            }
        }
        return Some(Tuple::Quantity(QuantityConstraint {
            property: None,
            units: permitted,
            assumed_value: None,
        }));
    }

    if let (Some(symbol), Some(value)) = (index("symbol"), index("value")) {
        let mut options = Vec::new();
        for row in rows {
            let Some(CPrimitiveObject::CTerminologyCode(code)) = row.members.get(symbol) else {
                continue;
            };
            let score = match row.members.get(value) {
                Some(CPrimitiveObject::CInteger(integer)) => integer
                    .constraint
                    .as_deref()
                    .and_then(<[Interval<i32>]>::first)
                    .and_then(|interval| integer_bounds(interval).lower().copied())
                    .map(widen),
                Some(CPrimitiveObject::CReal(real)) => real
                    .constraint
                    .as_deref()
                    .and_then(<[Interval<f64>]>::first)
                    .and_then(|interval| real_bounds(interval).lower().copied()),
                _ => None,
            };
            let Some(score) = score else { continue };
            options.push(OrdinalOption {
                value: score,
                symbol: CodedValue::new(
                    TerminologyName::new(LOCAL_TERMINOLOGY),
                    code.constraint.clone(),
                    None,
                ),
            });
        }
        return Some(Tuple::Ordinal(OrdinalConstraint {
            options,
            assumed_value: None,
        }));
    }

    Some(Tuple::Other(TupleConstraint {
        members: members
            .iter()
            .map(|member| RmAttributeName::new(member.rm_attribute_name.clone()))
            .collect(),
        rows: rows
            .iter()
            .map(|row| {
                row.members
                    .iter()
                    .map(|member| primitive_object(member, None))
                    .collect()
            })
            .collect(),
    }))
}
