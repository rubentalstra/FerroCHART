// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The rule from a Reference Model type plus its constraint to a field kind.
//!
//! One arm per Reference Model class of `docs/architecture.md` section 5. The
//! sections cited on each arm are openEHR RM Release-1.1.0 `data_types.html`
//! and openEHR AM Release-2.3.0 `AOM1.4.html`; the arm reads the internal
//! constraint model, so it is written once for both ADL generations.

use ferrochart_form::field::{
    BooleanField, ChoiceAlternative, CountField, DateField, DateTimeField, DurationField,
    FieldKind, IdentifierField, IntervalField, MultimediaField, OrdinalField, OrdinalOption,
    ParsableField, ProportionField, ProportionKind, QuantityField, QuantityUnitOption, RealField,
    ReferenceRange, ReferenceRanges, StateField, StateOption, StateTransitionOption, TextField,
    TimeField, UriField,
};
use ferrochart_form::ids::{RmTypeName, TerminologyName};
use ferrochart_form::range::Range;
use ferrochart_form::value::{
    BindingStrictness, Code, CodedOption, EnumeratedSet, ExpansionSource, Prefill, ValueSet,
};

use crate::derive::error::DeriveError;
use crate::derive::terms::Terms;
use crate::derive::{self, ATTRIBUTE_NAME};
use crate::model::ids::{ArchetypeId, LocalCode};
use crate::model::multiplicity::Bounds;
use crate::model::node::ConstraintNode;
use crate::model::payload::{
    BindingStatus, CodeSource, CodedConstraint, ConstraintPayload, DefaultValue, ExternalSet,
    IntegerConstraint, OrdinalConstraint, QuantityConstraint, RealConstraint, StateConstraint,
    TemporalConstraint, TextConstraint, TupleConstraint,
};

/// Attributes every `LOCATABLE` carries for the record's own bookkeeping.
///
/// openEHR RM Release-1.1.0 `common.html` defines them on `LOCATABLE`; none is
/// content a clinician enters, so a constraint on one shapes no field.
const HOUSEKEEPING: &[&str] = &[
    "archetype_node_id",
    "uid",
    "links",
    "archetype_details",
    "feeder_audit",
];

/// Attributes the Reference Model fills rather than a clinician.
///
/// `mappings` holds `TERM_MAPPING` (openEHR RM Release-1.1.0
/// `data_types.html` section 5.2.2), which the composition builder writes when
/// a coded value carries a mapping, and which a form neither collects nor
/// shows.
const NEVER_ENTERED: &[&str] = &["mappings"];

/// The `DV_ORDERED` attributes a form shows beside a value rather than
/// collecting.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.1. A constraint on
/// one shapes no field, so no arm of this module reads it as a value;
/// [`reference_ranges`] carries all three as display metadata.
const DISPLAY_METADATA: &[&str] = &["normal_status", "normal_range", "other_reference_ranges"];

/// Whether a constraint on `attribute` shapes no field at all.
pub(crate) fn is_never_entered(attribute: &str) -> bool {
    HOUSEKEEPING.contains(&attribute) || NEVER_ENTERED.contains(&attribute)
}

/// Whether a constraint on `attribute` is shown beside a value rather than
/// entered.
pub(crate) fn is_display_metadata(attribute: &str) -> bool {
    DISPLAY_METADATA.contains(&attribute)
}

/// Whether `rm_type` is a data value rather than a structure that holds one.
pub(crate) fn is_data_value(rm_type: &str) -> bool {
    rm_type.starts_with("DV_") || matches!(rm_type, "CODE_PHRASE" | "TERM_MAPPING")
}

/// The Reference Model class a generic type name parameterises.
///
/// `DV_INTERVAL<DV_QUANTITY>` names `DV_QUANTITY`; a bare `DV_INTERVAL` names
/// nothing.
fn type_parameter(rm_type: &str) -> Option<&str> {
    let (_, rest) = rm_type.split_once('<')?;
    rest.strip_suffix('>').filter(|inner| !inner.is_empty())
}

fn children_under<'n>(node: &'n ConstraintNode, attribute: &str) -> Vec<&'n ConstraintNode> {
    node.children()
        .iter()
        .filter(|child| child.identity().rm_attribute().as_str() == attribute)
        .collect()
}

fn sole_child<'n>(node: &'n ConstraintNode, attribute: &str) -> Option<&'n ConstraintNode> {
    node.children()
        .iter()
        .find(|child| child.identity().rm_attribute().as_str() == attribute)
}

/// The attributes of one data value, however the template states them.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.25 lets an ADL 2 archetype
/// state several attributes of one object as a `C_ATTRIBUTE_TUPLE` rather than
/// as separate children, with one row per admitted combination. A single row
/// says exactly what a constraint per attribute says, so this view reads
/// either spelling; more rows are refused rather than widened.
#[derive(Debug)]
struct Attributes<'n> {
    node: &'n ConstraintNode,
    tuple: Option<&'n TupleConstraint>,
}

impl<'n> Attributes<'n> {
    fn read(node: &'n ConstraintNode, path: &str) -> Result<Self, DeriveError> {
        let tuple = match *node.payload() {
            ConstraintPayload::Tuple(ref tuple) if tuple.rows.len() == 1 => Some(tuple),
            ConstraintPayload::Tuple(ref tuple) => {
                return Err(DeriveError::UnfoldedTuple {
                    path: path.to_owned(),
                    rm_type: node.identity().rm_type().as_str().to_owned(),
                    rows: tuple.rows.len(),
                });
            }
            _ => None,
        };
        Ok(Self { node, tuple })
    }

    /// The constraint on `attribute`, from the tuple row or from a child.
    fn payload(&self, attribute: &str) -> Option<&'n ConstraintPayload> {
        if let Some(tuple) = self.tuple
            && let Some(row) = tuple.rows.first()
            && let Some(index) = tuple
                .members
                .iter()
                .position(|member| member.as_str() == attribute)
            && let Some(cell) = row.get(index)
        {
            return Some(cell);
        }
        sole_child(self.node, attribute).map(ConstraintNode::payload)
    }

    /// Whether the template constrains no attribute of this value at all.
    fn is_empty(&self) -> bool {
        self.tuple.is_none() && self.node.children().is_empty()
    }

    /// Refuses a constrained attribute the derivation has no field for.
    ///
    /// Dropping one would widen the field past what the template admits, so it
    /// is named instead.
    fn reject_unmodelled(&self, allowed: &[&str], path: &str) -> Result<(), DeriveError> {
        let members = self
            .tuple
            .map(|tuple| tuple.members.as_slice())
            .unwrap_or_default();
        let stated = self
            .node
            .children()
            .iter()
            .map(|child| child.identity().rm_attribute().as_str())
            .chain(
                members
                    .iter()
                    .map(crate::model::ids::RmAttributeName::as_str),
            );
        for attribute in stated {
            if allowed.contains(&attribute)
                || is_never_entered(attribute)
                || is_display_metadata(attribute)
                || attribute == ATTRIBUTE_NAME
            {
                continue;
            }
            return Err(DeriveError::UnmodelledAttribute {
                path: path.to_owned(),
                rm_type: self.node.identity().rm_type().as_str().to_owned(),
                attribute: attribute.to_owned(),
            });
        }
        Ok(())
    }
}

fn mismatch(node: &ConstraintNode, path: &str) -> DeriveError {
    cell_mismatch(node, node.payload(), path)
}

fn cell_mismatch(node: &ConstraintNode, payload: &ConstraintPayload, path: &str) -> DeriveError {
    DeriveError::PayloadMismatch {
        path: path.to_owned(),
        rm_type: node.identity().rm_type().as_str().to_owned(),
        payload: payload.kind(),
    }
}

fn range_of<T: Clone>(bounds: &Bounds<T>) -> Range<T> {
    Range::new(
        bounds.lower().cloned(),
        bounds.upper().cloned(),
        bounds.lower_included(),
        bounds.upper_included(),
    )
}

fn text_field(constraint: &TextConstraint) -> TextField {
    TextField {
        patterns: constraint.patterns.clone(),
        options: constraint.list.clone(),
        // openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.3 defines
        // `C_STRING.list_open` as "True if the list is being used to specify
        // the constraint but is not considered exhaustive", so an unstated
        // flag leaves the list exhaustive.
        options_closed: !constraint.list.is_empty() && constraint.list_open != Some(true),
    }
}

fn text_at(
    attributes: &Attributes<'_>,
    attribute: &str,
    path: &str,
) -> Result<TextField, DeriveError> {
    match attributes.payload(attribute) {
        None => Ok(TextField::default()),
        // Only a string constrains a string.
        Some(payload) => match *payload {
            ConstraintPayload::Text(ref constraint) => Ok(text_field(constraint)),
            ref other => Err(cell_mismatch(attributes.node, other, path)),
        },
    }
}

fn count_of(constraint: &IntegerConstraint) -> CountField {
    CountField {
        options: constraint.list.clone(),
        ranges: constraint.ranges.iter().map(range_of).collect(),
    }
}

fn real_of(constraint: &RealConstraint) -> RealField {
    RealField {
        options: constraint.list.clone(),
        ranges: constraint.ranges.iter().map(range_of).collect(),
    }
}

fn count_at(
    attributes: &Attributes<'_>,
    attribute: &str,
    path: &str,
) -> Result<CountField, DeriveError> {
    match attributes.payload(attribute) {
        None => Ok(CountField::default()),
        Some(payload) => match *payload {
            ConstraintPayload::Integer(ref constraint) => Ok(count_of(constraint)),
            ref other => Err(cell_mismatch(attributes.node, other, path)),
        },
    }
}

fn real_at(
    attributes: &Attributes<'_>,
    attribute: &str,
    path: &str,
) -> Result<RealField, DeriveError> {
    match attributes.payload(attribute) {
        None => Ok(RealField::default()),
        Some(payload) => match *payload {
            ConstraintPayload::Real(ref constraint) => Ok(real_of(constraint)),
            // openEHR RM Release-1.1.0 `data_types.html` section 6.2.10 types
            // both parts of a proportion as `Real`, so an integer bound on one
            // is the same bound in a narrower spelling.
            ConstraintPayload::Integer(ref constraint) => Ok(RealField {
                options: constraint.list.iter().copied().map(widen).collect(),
                ranges: constraint
                    .ranges
                    .iter()
                    .map(|bounds| {
                        Range::new(
                            bounds.lower().copied().map(widen),
                            bounds.upper().copied().map(widen),
                            bounds.lower_included(),
                            bounds.upper_included(),
                        )
                    })
                    .collect(),
            }),
            ref other => Err(cell_mismatch(attributes.node, other, path)),
        },
    }
}

/// An integer bound read as a real one.
#[expect(
    clippy::cast_precision_loss,
    reason = "a magnitude beyond 2^53 is not a clinical proportion, and the alternative is to drop the bound"
)]
fn widen(value: i64) -> f64 {
    value as f64
}

fn fixed_boolean(
    attributes: &Attributes<'_>,
    attribute: &str,
    path: &str,
) -> Result<Option<bool>, DeriveError> {
    let Some(payload) = attributes.payload(attribute) else {
        return Ok(None);
    };
    match *payload {
        ConstraintPayload::Boolean(ref constraint) => {
            Ok(match (constraint.true_valid, constraint.false_valid) {
                (true, false) => Some(true),
                (false, true) => Some(false),
                _ => None,
            })
        }
        ref other => Err(cell_mismatch(attributes.node, other, path)),
    }
}

fn code_of(value: &crate::model::payload::CodedValue) -> Code {
    Code::new(
        TerminologyName::new(value.terminology().as_str()),
        value.code(),
    )
}

fn strictness_of(status: Option<BindingStatus>) -> Option<BindingStrictness> {
    status.map(|status| match status {
        BindingStatus::Required => BindingStrictness::Required,
        BindingStatus::Extensible => BindingStrictness::Extensible,
        BindingStatus::Preferred => BindingStrictness::Preferred,
        BindingStatus::Example => BindingStrictness::Example,
        BindingStatus::Other(code) => BindingStrictness::Other(code),
    })
}

/// The value the template prefills a field with.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.2: "default values do
/// appear in data, while assumed values don't". Only a default reaches a form,
/// so no arm of this module reads an `assumed_value`.
pub(crate) fn prefill(value: &DefaultValue) -> Prefill {
    match *value {
        DefaultValue::Boolean(value) => Prefill::Boolean(value),
        DefaultValue::Integer(value) => Prefill::Integer(value),
        DefaultValue::Real(value) => Prefill::Real(value),
        DefaultValue::Text(ref value) => Prefill::Text(value.clone()),
        DefaultValue::Coded {
            ref code,
            ref rubric,
        } => Prefill::Coded {
            code: code_of(code),
            rubric: rubric.clone(),
        },
        DefaultValue::Quantity {
            magnitude,
            ref units,
            precision,
        } => Prefill::Quantity {
            magnitude,
            units: units.clone(),
            precision,
        },
        DefaultValue::Ordinal { value, ref symbol } => Prefill::Ordinal {
            value,
            symbol: code_of(symbol),
        },
        DefaultValue::Temporal(ref value) => Prefill::Temporal(value.clone()),
        DefaultValue::Opaque(ref value) => Prefill::Opaque(value.clone()),
    }
}

/// The permitted codes of a coded field, with the rubrics the template
/// carries for them.
fn value_set(terms: &Terms<'_>, scope: &ArchetypeId, constraint: &CodedConstraint) -> ValueSet {
    match constraint.source {
        CodeSource::Enumerated {
            ref terminology,
            ref codes,
        } => enumerated(
            terms,
            scope,
            terminology.as_str(),
            codes.iter().map(String::as_str),
        ),
        CodeSource::OpenTerminology { ref terminology } => ValueSet::OpenTerminology {
            terminology: TerminologyName::new(terminology.as_str()),
        },
        CodeSource::External(ref external) => external_set(terms, scope, external),
        CodeSource::Unconstrained => ValueSet::Unconstrained,
    }
}

fn enumerated<'c>(
    terms: &Terms<'_>,
    scope: &ArchetypeId,
    terminology: &str,
    codes: impl Iterator<Item = &'c str>,
) -> ValueSet {
    let mut options = Vec::new();
    let mut needs_display_lookup = false;
    for code in codes {
        let (label, description) = terms.rubrics(scope, &LocalCode::new(code));
        if label.is_empty() {
            needs_display_lookup = true;
        }
        options.push(CodedOption {
            code: Code::new(TerminologyName::new(terminology), code),
            label,
            description,
        });
    }
    ValueSet::Enumerated(EnumeratedSet {
        terminology: TerminologyName::new(terminology),
        options,
        needs_display_lookup,
    })
}

/// A set the template names rather than lists.
///
/// A set whose members the archetype's own terminology enumerates is resolved
/// here; one it does not is marked for expansion by a terminology server at
/// render time, because nothing in the template says what is in it.
fn external_set(terms: &Terms<'_>, scope: &ArchetypeId, external: &ExternalSet) -> ValueSet {
    match *external {
        ExternalSet::ConstraintCode {
            ref code,
            ref bindings,
            ref operational_terminology,
        } => {
            if let Some(members) = terms.value_set(scope, code)
                && !members.is_empty()
            {
                // openEHR RM Release-1.1.0 `data_types.html` section 5.2.3
                // reserves `local` for codes the archetype defines itself.
                return enumerated(terms, scope, "local", members.iter().map(LocalCode::as_str));
            }
            ValueSet::Expansion(ExpansionSource::ConstraintCode {
                code: ferrochart_form::ids::LocalCode::new(code.as_str()),
                bindings: bindings
                    .iter()
                    .map(|(terminology, target)| {
                        (TerminologyName::new(terminology.as_str()), target.clone())
                    })
                    .collect(),
                pinned_terminology: operational_terminology
                    .as_ref()
                    .map(|terminology| TerminologyName::new(terminology.as_str())),
            })
        }
        ExternalSet::ReferenceSet { ref uri } => {
            ValueSet::Expansion(ExpansionSource::ReferenceSet { uri: uri.clone() })
        }
    }
}

fn coded_field(
    terms: &Terms<'_>,
    scope: &ArchetypeId,
    constraint: &CodedConstraint,
    rubric: Option<TextField>,
) -> FieldKind {
    FieldKind::Coded(ferrochart_form::field::CodedField {
        value_set: value_set(terms, scope, constraint),
        strictness: strictness_of(constraint.status),
        rubric,
    })
}

fn ordinal_field(
    terms: &Terms<'_>,
    scope: &ArchetypeId,
    constraint: &OrdinalConstraint,
) -> FieldKind {
    FieldKind::Ordinal(OrdinalField {
        options: constraint
            .options
            .iter()
            .map(|option| {
                let (label, description) =
                    terms.rubrics(scope, &LocalCode::new(option.symbol.code()));
                OrdinalOption {
                    score: option.value,
                    symbol: code_of(&option.symbol),
                    label,
                    description,
                }
            })
            .collect(),
    })
}

fn quantity_field(constraint: &QuantityConstraint) -> FieldKind {
    // NOTE: no specification governs this: our own design. No unit is marked
    // preferred, because the constrainer gives an ordered list and says
    // nothing about preference.
    FieldKind::Quantity(QuantityField {
        property: constraint.property.as_ref().map(code_of),
        units: constraint
            .units
            .iter()
            .map(|unit| QuantityUnitOption {
                units: unit.units.clone(),
                magnitude: unit.magnitude.as_ref().map(range_of),
                decimals: unit.precision.as_ref().map(range_of),
            })
            .collect(),
    })
}

/// A quantity an ADL 2 template states as a complex object rather than a
/// tuple.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.25 ties `magnitude` to
/// `units` with a `C_PRIMITIVE_TUPLE`, one row per admitted combination, and
/// the ADL 2 reader folds a recognised tuple into a quantity constraint. Where
/// the template states no tuple, the magnitude range it states applies to
/// every unit it lists, because there is nothing that says otherwise.
fn unfolded_quantity(attributes: &Attributes<'_>, path: &str) -> Result<FieldKind, DeriveError> {
    attributes.reject_unmodelled(&["magnitude", "units", "precision", "property"], path)?;
    let units = text_at(attributes, "units", path)?;
    // Every permitted unit carries the same magnitude and precision in the
    // unfolded shape, so a refusal names all of them rather than the first.
    let under = || units.options.join(", ");

    // One permitted unit carries one magnitude interval and one precision
    // (RM Release-1.1.0 `data_types.html` section 6.2.8), so anything the
    // template states beyond that is refused rather than trimmed to fit.
    // Trimming would make the field admit or refuse values the template does
    // not, which is the one thing a compiled form may never do.
    let stated = real_at(attributes, "magnitude", path)?;
    if !stated.options.is_empty() {
        return Err(DeriveError::UnrepresentableUnitBound {
            path: path.to_owned(),
            attribute: "magnitude",
            units: under(),
            found: format!("an enumeration of {} values", stated.options.len()),
        });
    }
    if stated.ranges.len() > 1 {
        return Err(DeriveError::UnrepresentableUnitBound {
            path: path.to_owned(),
            attribute: "magnitude",
            units: under(),
            found: format!("{} separate ranges", stated.ranges.len()),
        });
    }
    let magnitude = stated.ranges.first().cloned();

    let stated = count_at(attributes, "precision", path)?;
    if !stated.options.is_empty() {
        return Err(DeriveError::UnrepresentableUnitBound {
            path: path.to_owned(),
            attribute: "precision",
            units: under(),
            found: format!("an enumeration of {} values", stated.options.len()),
        });
    }
    if stated.ranges.len() > 1 {
        return Err(DeriveError::UnrepresentableUnitBound {
            path: path.to_owned(),
            attribute: "precision",
            units: under(),
            found: format!("{} separate ranges", stated.ranges.len()),
        });
    }
    let decimals = stated.ranges.first().cloned();

    Ok(FieldKind::Quantity(QuantityField {
        property: None,
        units: units
            .options
            .into_iter()
            .map(|symbol| QuantityUnitOption {
                units: symbol,
                magnitude: magnitude.clone(),
                decimals: decimals.clone(),
            })
            .collect(),
    }))
}

fn proportion_field(attributes: &Attributes<'_>, path: &str) -> Result<FieldKind, DeriveError> {
    attributes.reject_unmodelled(
        &[
            "numerator",
            "denominator",
            "type",
            "precision",
            "is_integral",
        ],
        path,
    )?;
    let kinds = match attributes.payload("type") {
        None => Vec::new(),
        Some(payload) => match *payload {
            ConstraintPayload::Integer(ref constraint) => constraint
                .list
                .iter()
                .map(|&code| ProportionKind::from_code(code))
                .collect(),
            ref other => return Err(cell_mismatch(attributes.node, other, path)),
        },
    };
    Ok(FieldKind::Proportion(ProportionField {
        kinds,
        numerator: real_at(attributes, "numerator", path)?,
        denominator: real_at(attributes, "denominator", path)?,
        decimals: count_at(attributes, "precision", path)?,
        is_integral: fixed_boolean(attributes, "is_integral", path)?,
    }))
}

fn state_field(constraint: &StateConstraint) -> FieldKind {
    // NOTE: no specification governs this: our own design. `C_DV_STATE` is
    // declared in the openEHR ITS-XML 2.0.0 `OpenehrProfile.xsd` and by no
    // AOM prose, so the field carries the states and reads nothing into them.
    FieldKind::State(StateField {
        states: constraint
            .states
            .iter()
            .map(|state| StateOption {
                name: state.name.clone(),
                is_terminal: state.is_terminal,
                transitions: state
                    .transitions
                    .iter()
                    .map(|transition| StateTransitionOption {
                        event: transition.event.clone(),
                        action: transition.action.clone(),
                        guard: transition.guard.clone(),
                        next_state: transition.next_state.clone(),
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn temporal_ranges(constraint: &TemporalConstraint) -> Vec<Range<String>> {
    constraint.ranges.iter().map(range_of).collect()
}

/// The temporal constraint a `value` attribute carries, where it carries one.
fn temporal_of<'n>(
    attributes: &Attributes<'n>,
    path: &str,
    expected: fn(&ConstraintPayload) -> Option<&TemporalConstraint>,
) -> Result<Option<&'n TemporalConstraint>, DeriveError> {
    attributes.reject_unmodelled(&["value"], path)?;
    let Some(payload) = attributes.payload("value") else {
        return Ok(None);
    };
    expected(payload)
        .map(Some)
        .ok_or_else(|| cell_mismatch(attributes.node, payload, path))
}

fn as_date(payload: &ConstraintPayload) -> Option<&TemporalConstraint> {
    match *payload {
        ConstraintPayload::Date(ref constraint) => Some(constraint),
        _ => None,
    }
}

fn as_time(payload: &ConstraintPayload) -> Option<&TemporalConstraint> {
    match *payload {
        ConstraintPayload::Time(ref constraint) => Some(constraint),
        _ => None,
    }
}

fn as_date_time(payload: &ConstraintPayload) -> Option<&TemporalConstraint> {
    match *payload {
        ConstraintPayload::DateTime(ref constraint) => Some(constraint),
        _ => None,
    }
}

fn as_duration(payload: &ConstraintPayload) -> Option<&TemporalConstraint> {
    match *payload {
        ConstraintPayload::Duration(ref constraint) => Some(constraint),
        _ => None,
    }
}

/// The field a data value with no constraint at all collects.
///
/// A template that names a Reference Model class and constrains nothing has
/// still determined the class, so the form collects that class unrestricted.
fn unconstrained(rm_type: &str) -> Option<FieldKind> {
    let kind = match rm_type {
        "DV_BOOLEAN" => FieldKind::Boolean(BooleanField {
            true_allowed: true,
            false_allowed: true,
        }),
        "DV_TEXT" => FieldKind::Text(TextField::default()),
        "DV_URI" => FieldKind::Uri(UriField {
            patterns: Vec::new(),
            required_scheme: None,
        }),
        "DV_EHR_URI" => FieldKind::Uri(UriField {
            patterns: Vec::new(),
            required_scheme: Some("ehr".to_owned()),
        }),
        "DV_CODED_TEXT" | "CODE_PHRASE" => FieldKind::Coded(ferrochart_form::field::CodedField {
            value_set: ValueSet::Unconstrained,
            strictness: None,
            rubric: None,
        }),
        "DV_ORDINAL" | "DV_SCALE" => FieldKind::Ordinal(OrdinalField {
            options: Vec::new(),
        }),
        "DV_COUNT" => FieldKind::Count(CountField::default()),
        "DV_QUANTITY" => FieldKind::Quantity(QuantityField {
            property: None,
            units: Vec::new(),
        }),
        "DV_PROPORTION" => FieldKind::Proportion(ProportionField {
            kinds: Vec::new(),
            numerator: RealField::default(),
            denominator: RealField::default(),
            decimals: CountField::default(),
            is_integral: None,
        }),
        "DV_DATE" => FieldKind::Date(DateField {
            month: derive::temporal::date(None).0,
            day: derive::temporal::date(None).1,
            ranges: Vec::new(),
        }),
        "DV_TIME" => FieldKind::Time(TimeField {
            minute: derive::temporal::time(None).0,
            second: derive::temporal::time(None).1,
            timezone: derive::temporal::timezone(None),
            ranges: Vec::new(),
        }),
        "DV_DATE_TIME" => {
            let (month, day, hour, minute, second) = derive::temporal::date_time(None);
            FieldKind::DateTime(DateTimeField {
                month,
                day,
                hour,
                minute,
                second,
                timezone: derive::temporal::timezone(None),
                ranges: Vec::new(),
            })
        }
        "DV_DURATION" => FieldKind::Duration(DurationField {
            components: derive::temporal::duration(None),
            ranges: Vec::new(),
        }),
        "DV_IDENTIFIER" => FieldKind::Identifier(IdentifierField::default()),
        "DV_MULTIMEDIA" => FieldKind::Multimedia(MultimediaField {
            media_types: ValueSet::Unconstrained,
            uri: None,
        }),
        "DV_PARSABLE" => FieldKind::Parsable(ParsableField::default()),
        "DV_STATE" => FieldKind::State(StateField { states: Vec::new() }),
        _ => return None,
    };
    Some(kind)
}

/// A `DV_CODED_TEXT`: its defining code, and the rubric it inherits from
/// `DV_TEXT`.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.4.
fn coded_text_field(
    terms: &Terms<'_>,
    attributes: &Attributes<'_>,
    path: &str,
) -> Result<FieldKind, DeriveError> {
    attributes.reject_unmodelled(&["defining_code", "value"], path)?;
    let rubric = match attributes.payload("value") {
        None => None,
        Some(payload) => match *payload {
            ConstraintPayload::Text(ref constraint) => Some(text_field(constraint)),
            ref other => return Err(cell_mismatch(attributes.node, other, path)),
        },
    };
    match attributes.payload("defining_code") {
        None => Ok(FieldKind::Coded(ferrochart_form::field::CodedField {
            value_set: ValueSet::Unconstrained,
            strictness: None,
            rubric,
        })),
        Some(payload) => match *payload {
            ConstraintPayload::Coded(ref constraint) => Ok(coded_field(
                terms,
                attributes.node.terminology_scope(),
                constraint,
                rubric,
            )),
            ref other => Err(cell_mismatch(attributes.node, other, path)),
        },
    }
}

/// A `DV_MULTIMEDIA`: the media types it accepts, and the reference it
/// carries.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 9.2.2.
fn multimedia_field(
    terms: &Terms<'_>,
    attributes: &Attributes<'_>,
    path: &str,
) -> Result<FieldKind, DeriveError> {
    attributes.reject_unmodelled(&["media_type", "uri"], path)?;
    let media_types = match attributes.payload("media_type") {
        None => ValueSet::Unconstrained,
        Some(payload) => match *payload {
            ConstraintPayload::Coded(ref constraint) => {
                value_set(terms, attributes.node.terminology_scope(), constraint)
            }
            ref other => return Err(cell_mismatch(attributes.node, other, path)),
        },
    };
    Ok(FieldKind::Multimedia(MultimediaField {
        media_types,
        uri: match attributes.payload("uri") {
            None => None,
            Some(payload) => match *payload {
                ConstraintPayload::Text(ref constraint) => Some(text_field(constraint)),
                ref other => return Err(cell_mismatch(attributes.node, other, path)),
            },
        },
    }))
}

/// The field one data-value node collects.
///
/// Returns `None` where the template named a class but never determined which
/// value it holds, which the caller records as undetermined content rather
/// than rendering.
///
/// # Errors
/// [`DeriveError`] when the node collects a Reference Model class this
/// derivation does not model, constrains an attribute it does not model, or
/// carries a constraint that cannot sit under its class.
pub(crate) fn value_kind(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    path: &str,
) -> Result<Option<FieldKind>, DeriveError> {
    // NOTE: no specification governs this: our own design. The kind names
    // what the field collects and never a widget, because nothing in RM or
    // AM says whether a four-member set is radio buttons or a dropdown.
    let rm_type = node.identity().rm_type().as_str();
    let scope = node.terminology_scope();

    // A domain constrainer decides the field on its own; the arms below read
    // the complex-object spelling of the same fact.
    match *node.payload() {
        ConstraintPayload::Ordinal(ref constraint) => {
            return Ok(Some(ordinal_field(terms, scope, constraint)));
        }
        ConstraintPayload::Quantity(ref constraint) => {
            return Ok(Some(quantity_field(constraint)));
        }
        ConstraintPayload::State(ref constraint) => {
            return Ok(Some(state_field(constraint)));
        }
        ConstraintPayload::Coded(ref constraint) => {
            return Ok(Some(coded_field(terms, scope, constraint, None)));
        }
        // A tuple states the attributes of a complex object in another
        // spelling, which the attribute view below reads.
        ConstraintPayload::Structure | ConstraintPayload::Tuple(_) => {}
        // A primitive constrainer standing where a data value belongs is a
        // shape neither generation produces.
        _ => return Err(mismatch(node, path)),
    }

    let attributes = Attributes::read(node, path)?;
    if attributes.is_empty() {
        if rm_type.starts_with("DV_INTERVAL") {
            return interval_field(terms, &attributes, path);
        }
        return unconstrained(rm_type)
            .map(Some)
            .ok_or_else(|| DeriveError::UnmodelledDataValue {
                path: path.to_owned(),
                rm_type: rm_type.to_owned(),
            });
    }
    if let Some(kind) = temporal_kind(&attributes, rm_type, path)? {
        return Ok(Some(kind));
    }
    constrained_kind(terms, &attributes, rm_type, path)
}

/// The field a date, time, date-and-time or duration collects, where that is
/// what the node is.
fn temporal_kind(
    attributes: &Attributes<'_>,
    rm_type: &str,
    path: &str,
) -> Result<Option<FieldKind>, DeriveError> {
    let kind = match rm_type {
        "DV_DATE" => {
            let constraint = temporal_of(attributes, path, as_date)?;
            let pattern = constraint.and_then(|c| c.pattern.as_deref());
            let (month, day) = derive::temporal::date(pattern);
            FieldKind::Date(DateField {
                month,
                day,
                ranges: constraint.map(temporal_ranges).unwrap_or_default(),
            })
        }
        "DV_TIME" => {
            let constraint = temporal_of(attributes, path, as_time)?;
            let pattern = constraint.and_then(|c| c.pattern.as_deref());
            let (minute, second) = derive::temporal::time(pattern);
            FieldKind::Time(TimeField {
                minute,
                second,
                timezone: derive::temporal::timezone(constraint.and_then(|c| c.timezone_validity)),
                ranges: constraint.map(temporal_ranges).unwrap_or_default(),
            })
        }
        "DV_DATE_TIME" => {
            let constraint = temporal_of(attributes, path, as_date_time)?;
            let pattern = constraint.and_then(|c| c.pattern.as_deref());
            let (month, day, hour, minute, second) = derive::temporal::date_time(pattern);
            FieldKind::DateTime(DateTimeField {
                month,
                day,
                hour,
                minute,
                second,
                timezone: derive::temporal::timezone(constraint.and_then(|c| c.timezone_validity)),
                ranges: constraint.map(temporal_ranges).unwrap_or_default(),
            })
        }
        "DV_DURATION" => {
            let constraint = temporal_of(attributes, path, as_duration)?;
            let pattern = constraint.and_then(|c| c.pattern.as_deref());
            FieldKind::Duration(DurationField {
                components: derive::temporal::duration(pattern),
                ranges: constraint.map(temporal_ranges).unwrap_or_default(),
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(kind))
}

/// The field a data value with at least one constrained attribute collects.
fn constrained_kind(
    terms: &Terms<'_>,
    attributes: &Attributes<'_>,
    rm_type: &str,
    path: &str,
) -> Result<Option<FieldKind>, DeriveError> {
    let kind = match rm_type {
        "DV_BOOLEAN" => {
            attributes.reject_unmodelled(&["value"], path)?;
            FieldKind::Boolean(match fixed_boolean(attributes, "value", path)? {
                Some(true) => BooleanField {
                    true_allowed: true,
                    false_allowed: false,
                },
                Some(false) => BooleanField {
                    true_allowed: false,
                    false_allowed: true,
                },
                None => BooleanField {
                    true_allowed: true,
                    false_allowed: true,
                },
            })
        }
        "DV_TEXT" => {
            attributes.reject_unmodelled(&["value"], path)?;
            FieldKind::Text(text_at(attributes, "value", path)?)
        }
        "DV_URI" | "DV_EHR_URI" => {
            attributes.reject_unmodelled(&["value"], path)?;
            FieldKind::Uri(UriField {
                patterns: text_at(attributes, "value", path)?.patterns,
                // openEHR RM Release-1.1.0 `data_types.html` section 10.3.2
                // invariant `Scheme_valid` requires the `ehr` scheme.
                required_scheme: (rm_type == "DV_EHR_URI").then(|| "ehr".to_owned()),
            })
        }
        "DV_CODED_TEXT" => coded_text_field(terms, attributes, path)?,
        "DV_COUNT" => {
            attributes.reject_unmodelled(&["magnitude"], path)?;
            FieldKind::Count(count_at(attributes, "magnitude", path)?)
        }
        "DV_QUANTITY" => unfolded_quantity(attributes, path)?,
        "DV_PROPORTION" => proportion_field(attributes, path)?,
        "DV_IDENTIFIER" => {
            attributes.reject_unmodelled(&["issuer", "assigner", "id", "type"], path)?;
            FieldKind::Identifier(IdentifierField {
                issuer: text_at(attributes, "issuer", path)?,
                assigner: text_at(attributes, "assigner", path)?,
                id: text_at(attributes, "id", path)?,
                identifier_type: text_at(attributes, "type", path)?,
            })
        }
        "DV_MULTIMEDIA" => multimedia_field(terms, attributes, path)?,
        "DV_PARSABLE" => {
            attributes.reject_unmodelled(&["value", "formalism"], path)?;
            FieldKind::Parsable(ParsableField {
                formalism: text_at(attributes, "formalism", path)?,
                value: text_at(attributes, "value", path)?,
            })
        }
        "DV_STATE" => {
            attributes.reject_unmodelled(&["value", "is_terminal"], path)?;
            FieldKind::State(StateField { states: Vec::new() })
        }
        other if other.starts_with("DV_INTERVAL") => {
            return interval_field(terms, attributes, path);
        }
        other => {
            return Err(DeriveError::UnmodelledDataValue {
                path: path.to_owned(),
                rm_type: other.to_owned(),
            });
        }
    };
    Ok(Some(kind))
}

/// Both ends of an interval, each constrained by the constrainer of the
/// element type.
fn interval_field(
    terms: &Terms<'_>,
    attributes: &Attributes<'_>,
    path: &str,
) -> Result<Option<FieldKind>, DeriveError> {
    attributes.reject_unmodelled(
        &[
            "lower",
            "upper",
            "lower_included",
            "upper_included",
            "lower_unbounded",
            "upper_unbounded",
        ],
        path,
    )?;
    let node = attributes.node;
    let rm_type = node.identity().rm_type().as_str();
    let lower_node = sole_child(node, "lower");
    let upper_node = sole_child(node, "upper");
    let element = type_parameter(rm_type)
        .or_else(|| lower_node.map(|child| child.identity().rm_type().as_str()))
        .or_else(|| upper_node.map(|child| child.identity().rm_type().as_str()));
    // openEHR RM Release-1.1.0 `data_types.html` section 6.2.2 types both ends
    // as the same `DV_ORDERED` descendant; a template that names neither the
    // type parameter nor either end has not said which.
    let Some(element) = element else {
        return Ok(None);
    };
    let end = |child: Option<&ConstraintNode>| -> Result<FieldKind, DeriveError> {
        match child {
            Some(child) => {
                // An interval end is itself a `DV_ORDERED` (RM Release-1.1.0
                // `data_types.html` section 6.2.2), so it may state a
                // reference band. `IntervalField` has nowhere to put one, and
                // a band on the bound of a range is refused rather than
                // dropped, because every other constraint this derivation
                // cannot carry is refused by name (#87).
                if let Some(stated) = child
                    .children()
                    .iter()
                    .map(|node| node.identity().rm_attribute().as_str())
                    .find(|attribute| is_display_metadata(attribute))
                {
                    return Err(DeriveError::UnmodelledAttribute {
                        path: path.to_owned(),
                        rm_type: child.identity().rm_type().as_str().to_owned(),
                        attribute: stated.to_owned(),
                    });
                }
                value_kind(terms, child, path)?.ok_or_else(|| DeriveError::UnmodelledDataValue {
                    path: path.to_owned(),
                    rm_type: child.identity().rm_type().as_str().to_owned(),
                })
            }
            None => unconstrained(element).ok_or_else(|| DeriveError::UnmodelledDataValue {
                path: path.to_owned(),
                rm_type: element.to_owned(),
            }),
        }
    };
    Ok(Some(FieldKind::Interval(Box::new(IntervalField {
        element_rm_type: RmTypeName::new(element),
        lower: end(lower_node)?,
        upper: end(upper_node)?,
        lower_included: fixed_boolean(attributes, "lower_included", path)?,
        upper_included: fixed_boolean(attributes, "upper_included", path)?,
        lower_unbounded: fixed_boolean(attributes, "lower_unbounded", path)?,
        upper_unbounded: fixed_boolean(attributes, "upper_unbounded", path)?,
    }))))
}

/// The reference bands one data value states beside itself.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 gives every
/// `DV_ORDERED` a `normal_status`, a `normal_range` and
/// `other_reference_ranges`, and none of the three is entered, so each is read
/// here as display metadata rather than as a field. A band reuses the value
/// shape of the class it is stated over: `normal_status` is a coded value and
/// resolves its rubrics as any coded field does, and a range is an interval
/// over the class the value itself collects.
///
/// Returns `None` where the node states none of the three.
///
/// # Errors
/// [`DeriveError`] when an attribute carries a value shape the Reference Model
/// does not give it, when a range never says what it is an interval of, or
/// when a `REFERENCE_RANGE` constrains an attribute other than its `meaning`
/// and its `range`.
pub(crate) fn reference_ranges(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    path: &str,
) -> Result<Option<Box<ReferenceRanges>>, DeriveError> {
    // NOTE: openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 types
    // `normal_status` and `normal_range` as `0..1`, so one child carries the
    // whole constraint on either.
    let normal_status = match sole_child(node, "normal_status") {
        None => None,
        Some(child) => Some(status_band(terms, child, path)?),
    };
    let normal_range = match sole_child(node, "normal_range") {
        None => None,
        Some(child) => Some(interval_band(terms, child, "normal_range", path)?),
    };
    // The Reference Model types `other_reference_ranges` as a `List`, so the
    // order the template states them in is the order they are shown in.
    let mut other = Vec::new();
    for child in children_under(node, "other_reference_ranges") {
        other.push(reference_range(terms, child, path)?);
    }
    let bands = ReferenceRanges {
        normal_status,
        normal_range,
        other,
    };
    Ok((!bands.is_empty()).then(|| Box::new(bands)))
}

/// The refusal of a band the template states as something other than the
/// Reference Model's type for it.
fn metadata_shape(
    kind: Option<&FieldKind>,
    attribute: &'static str,
    expected: &'static str,
    path: &str,
) -> DeriveError {
    DeriveError::MetadataShape {
        path: path.to_owned(),
        attribute,
        // An interval of no stated element type is the one shape
        // `value_kind` leaves undetermined.
        found: kind.map_or("interval", FieldKind::name),
        expected,
    }
}

/// A `normal_status`, whose value set is read as any coded field's is.
fn status_band(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    path: &str,
) -> Result<ferrochart_form::field::CodedField, DeriveError> {
    match value_kind(terms, node, path)? {
        Some(FieldKind::Coded(field)) => Ok(field),
        other => Err(metadata_shape(
            other.as_ref(),
            "normal_status",
            "a CODE_PHRASE",
            path,
        )),
    }
}

/// One band, as the interval over the class the value collects.
fn interval_band(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    attribute: &'static str,
    path: &str,
) -> Result<IntervalField, DeriveError> {
    match value_kind(terms, node, path)? {
        Some(FieldKind::Interval(field)) => Ok(*field),
        None => Err(DeriveError::UntypedMetadataRange {
            path: path.to_owned(),
            attribute,
        }),
        Some(other) => Err(metadata_shape(
            Some(&other),
            attribute,
            "a DV_INTERVAL over the class the value collects",
            path,
        )),
    }
}

/// One `REFERENCE_RANGE`: what the band means, and the band itself.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.3 gives it a
/// `meaning` and a `range`, both mandatory in data. A template that
/// constrains neither has stated a band and left both open, which is what an
/// absent one records.
fn reference_range(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    path: &str,
) -> Result<ReferenceRange, DeriveError> {
    if !matches!(*node.payload(), ConstraintPayload::Structure) {
        return Err(mismatch(node, path));
    }
    for child in node.children() {
        let attribute = child.identity().rm_attribute().as_str();
        if attribute != "meaning" && attribute != "range" {
            return Err(DeriveError::UnmodelledAttribute {
                path: path.to_owned(),
                rm_type: node.identity().rm_type().as_str().to_owned(),
                attribute: attribute.to_owned(),
            });
        }
    }
    Ok(ReferenceRange {
        meaning: match sole_child(node, "meaning") {
            None => None,
            Some(child) => Some(meaning_band(terms, child, path)?),
        },
        range: match sole_child(node, "range") {
            None => None,
            Some(child) => Some(interval_band(terms, child, "range", path)?),
        },
    })
}

/// What a `REFERENCE_RANGE.meaning` states the band means.
fn meaning_band(
    terms: &Terms<'_>,
    node: &ConstraintNode,
    path: &str,
) -> Result<FieldKind, DeriveError> {
    match value_kind(terms, node, path)? {
        // openEHR RM Release-1.1.0 `data_types.html` section 5.2.4 makes
        // `DV_CODED_TEXT` a `DV_TEXT`, so a coded meaning is the same
        // attribute stated more narrowly.
        Some(kind @ (FieldKind::Text(_) | FieldKind::Coded(_))) => Ok(kind),
        other => Err(metadata_shape(other.as_ref(), "meaning", "a DV_TEXT", path)),
    }
}

/// Whether the constraint admits exactly one value, so the template has
/// already decided it and nothing is entered.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.2 is where the rule is
/// visible in the specification: a `C_BOOLEAN` with one of `true_valid` and
/// `false_valid` set has fixed the value. No specification governs the same
/// reading of a one-member list, and FerroCHART applies it: a field with one
/// admitted value collects no decision.
pub(crate) fn is_fixed(kind: &FieldKind) -> bool {
    match *kind {
        FieldKind::Boolean(ref field) => field.true_allowed != field.false_allowed,
        FieldKind::Text(ref field) => {
            field.options_closed && field.options.len() == 1 && field.patterns.is_empty()
        }
        FieldKind::Coded(ref field) => match field.value_set {
            ValueSet::Enumerated(ref set) => set.options.len() == 1,
            _ => false,
        },
        FieldKind::Count(ref field) => field.options.len() == 1 && field.ranges.is_empty(),
        FieldKind::Ordinal(ref field) => field.options.len() == 1,
        _ => false,
    }
}

/// One alternative of a choice.
///
/// # Errors
/// [`DeriveError`] when the alternative states a reference band this
/// derivation cannot show beside it.
pub(crate) fn alternative(
    terms: &Terms<'_>,
    key: ferrochart_form::key::NodeKey,
    node: &ConstraintNode,
    kind: FieldKind,
    label: ferrochart_form::text::Localized,
) -> Result<ChoiceAlternative, DeriveError> {
    let bands = reference_ranges(terms, node, &key.to_string())?;
    Ok(ChoiceAlternative {
        key,
        rm_type: RmTypeName::new(node.identity().rm_type().as_str()),
        label,
        occurrences: derive::occurrences_of(node.occurrences()),
        kind,
        reference_ranges: bands,
        prefill: node.default_value().map(prefill),
    })
}

/// Every `value` alternative of an `ELEMENT`, and the codes it draws on.
pub(crate) fn value_children(node: &ConstraintNode) -> Vec<&ConstraintNode> {
    children_under(node, "value")
}
