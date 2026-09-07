// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What one node constrains, expressed once for both ADL generations.
//!
//! The two generations spell the same clinical constraint through different
//! classes: ADL 1.4 constrains a quantity with the `C_DV_QUANTITY` domain type,
//! while AOM 2 constrains the same fact with a `C_COMPLEX_OBJECT` over
//! `magnitude` and `units` tied by a `C_PRIMITIVE_TUPLE` (openEHR AM
//! Release-2.3.0 `AOM2.html` sections 4.3.1 and 4.5.25). Each reader
//! normalizes into the payloads below, so the field derivation is written once
//! and nothing above this point can tell which generation produced a node.

use crate::model::ids::{LocalCode, RmAttributeName, TerminologyName};
use crate::model::multiplicity::Bounds;
use std::collections::BTreeMap;

/// A code and the terminology that defines it.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3, `CODE_PHRASE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodedValue {
    terminology: TerminologyName,
    code: String,
    preferred_term: Option<String>,
}

impl CodedValue {
    /// The pair a selection emits.
    #[must_use]
    pub fn new(
        terminology: TerminologyName,
        code: impl Into<String>,
        preferred_term: Option<String>,
    ) -> Self {
        Self {
            terminology,
            code: code.into(),
            preferred_term,
        }
    }

    /// The terminology the code is drawn from.
    #[must_use]
    pub fn terminology(&self) -> &TerminologyName {
        &self.terminology
    }

    /// The code string.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The display text the template carries beside the code, which openEHR RM
    /// Release-1.1.0 `data_types.html` section 5.2.3 marks as display only.
    #[must_use]
    pub fn preferred_term(&self) -> Option<&str> {
        self.preferred_term.as_deref()
    }
}

/// How strictly a value set binds.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.10,
/// `C_PRIMITIVE_OBJECT.constraint_status`. ADL 1.4 has no way to say this, so
/// a node read from an ADL 1.4 template leaves it unstated. Unstated means the
/// template did not say, never "this came from ADL 1.4".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindingStatus {
    /// The value must come from the set.
    Required,
    /// The set may be extended.
    Extensible,
    /// The set is preferred but not enforced.
    Preferred,
    /// The set is illustrative.
    Example,
    /// A status value outside the four the specification names, kept as the
    /// integer the template states rather than dropped.
    Other(i32),
}

/// The value a template prefills a node with.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.2 keeps this distinct
/// from an assumed value: "default values do appear in data, while assumed
/// values don't".
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DefaultValue {
    /// A `DV_BOOLEAN` value.
    Boolean(bool),
    /// A `DV_COUNT` magnitude.
    Integer(i64),
    /// A real magnitude with no units.
    Real(f64),
    /// A `DV_TEXT` value, or any other plain string.
    Text(String),
    /// A `DV_CODED_TEXT` defining code, with its rubric where the template
    /// states one.
    Coded {
        /// The code the composition carries.
        code: CodedValue,
        /// The rubric the template states beside it.
        rubric: Option<String>,
    },
    /// A `DV_QUANTITY` value.
    Quantity {
        /// The magnitude.
        magnitude: f64,
        /// The units the magnitude is stated in.
        units: String,
        /// The decimal places the value carries.
        precision: Option<i32>,
    },
    /// A `DV_ORDINAL` or `DV_SCALE` value.
    Ordinal {
        /// The score.
        value: f64,
        /// The symbol that carries the score.
        symbol: CodedValue,
    },
    /// An ISO 8601 date, time, date and time, or duration, as the template
    /// spells it.
    Temporal(String),
    /// A default this reader does not model, kept as the text the template
    /// states so nothing is dropped.
    Opaque(String),
}

/// A constraint on a boolean.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.2 `C_BOOLEAN` and `AOM2.html`
/// section 4.5.11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanConstraint {
    /// Whether `true` is admitted.
    pub true_valid: bool,
    /// Whether `false` is admitted.
    pub false_valid: bool,
    /// The value assumed when an optional node is omitted, which never reaches
    /// the data.
    pub assumed_value: Option<bool>,
}

/// A constraint on an integer.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.4 `C_INTEGER` and `AOM2.html`
/// section 4.5.14.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerConstraint {
    /// The values the template enumerates, in the order it lists them.
    pub list: Vec<i64>,
    /// Every range the template admits.
    pub ranges: Vec<Bounds<i64>>,
    /// The value assumed when an optional node is omitted.
    pub assumed_value: Option<i64>,
}

/// A constraint on a real number.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.5 `C_REAL` and `AOM2.html`
/// section 4.5.15.
#[derive(Debug, Clone, PartialEq)]
pub struct RealConstraint {
    /// The values the template enumerates, in the order it lists them.
    pub list: Vec<f64>,
    /// Every range the template admits.
    pub ranges: Vec<Bounds<f64>>,
    /// The value assumed when an optional node is omitted.
    pub assumed_value: Option<f64>,
}

/// A constraint on a string.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.3 `C_STRING` and `AOM2.html`
/// section 4.5.12.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextConstraint {
    /// The regular expressions the value may match, in the order the template
    /// lists them, each without the delimiters ADL 2 writes around one.
    pub patterns: Vec<String>,
    /// The values the template enumerates, in the order it lists them.
    pub list: Vec<String>,
    /// Whether the enumeration is open, where the template says.
    pub list_open: Option<bool>,
    /// The value assumed when an optional node is omitted.
    pub assumed_value: Option<String>,
}

/// Where a coded field's permitted codes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeSource {
    /// Every code the template lists, all from one terminology.
    Enumerated {
        /// The terminology the codes are drawn from.
        terminology: TerminologyName,
        /// The codes, in the order the template lists them.
        codes: Vec<String>,
    },
    /// Any code from the named terminology; the template lists none.
    OpenTerminology {
        /// The terminology every value must come from.
        terminology: TerminologyName,
    },
    /// A set the template names but does not enumerate, which a terminology
    /// server expands.
    External(ExternalSet),
    /// Neither a terminology nor a list: any code at all.
    Unconstrained,
}

/// A value set a terminology server has to expand.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExternalSet {
    /// A set named by a local `ac`-code.
    ///
    /// ADL 1.4 spells this as a `CONSTRAINT_REF` whose `reference` names an
    /// entry in the archetype's `constraint_definitions`, resolved through
    /// `constraint_bindings` (openEHR AM Release-2.3.0 `AOM1.4.html`
    /// section 4.3.14). ADL 2 spells it as a `C_TERMINOLOGY_CODE` whose
    /// constraint is that `ac`-code (`ADL2.html` section 7.13.5.1).
    ConstraintCode {
        /// The `ac`-code naming the set.
        code: LocalCode,
        /// Every target the archetype binds the set to, keyed by terminology.
        bindings: BTreeMap<TerminologyName, String>,
        /// The terminology an ADL 2 operational binding pins the set to, where
        /// the template states one.
        operational_terminology: Option<TerminologyName>,
    },
    /// A set named by URI.
    ///
    /// `C_CODE_REFERENCE.referenceSetUri` exists only in the ITS-XML
    /// `Template.xsd`; no AOM prose defines the class. See the module note on
    /// [`crate::adl14`].
    ReferenceSet {
        /// The URI the template states.
        uri: String,
    },
}

/// A constraint on a coded value.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.4, `DV_CODED_TEXT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodedConstraint {
    /// Where the permitted codes come from.
    pub source: CodeSource,
    /// The code assumed when an optional node is omitted.
    pub assumed_value: Option<CodedValue>,
    /// How strictly the set binds, where the template says.
    pub status: Option<BindingStatus>,
}

/// One option of an ordinal or scale.
#[derive(Debug, Clone, PartialEq)]
pub struct OrdinalOption {
    /// The score.
    pub value: f64,
    /// The symbol that carries the score, which is what a composition stores.
    pub symbol: CodedValue,
}

/// A constraint on an ordinal or a scale.
///
/// openEHR RM Release-1.1.0 `data_types.html` sections 6.2.4 and 6.2.5. List
/// order is display order, because the specification gives an ordered list and
/// nothing else to order by.
#[derive(Debug, Clone, PartialEq)]
pub struct OrdinalConstraint {
    /// The options, in the order the template lists them.
    pub options: Vec<OrdinalOption>,
    /// The option assumed when an optional node is omitted.
    pub assumed_value: Option<OrdinalOption>,
}

/// One permitted unit of a quantity, with the range and precision that unit
/// carries.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityUnit {
    /// The unit symbol.
    pub units: String,
    /// The magnitudes this unit admits.
    pub magnitude: Option<Bounds<f64>>,
    /// The decimal places this unit admits.
    pub precision: Option<Bounds<i64>>,
}

/// A constraint on a quantity.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.8. Changing the unit
/// changes the permitted range and the decimals, because each permitted unit
/// carries its own.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityConstraint {
    /// The property the magnitude measures, where the template names one.
    pub property: Option<CodedValue>,
    /// The permitted units, in the order the template lists them.
    pub units: Vec<QuantityUnit>,
    /// The value assumed when an optional node is omitted.
    pub assumed_value: Option<(f64, String)>,
}

/// A constraint on a date, a time, a date and time, or a duration.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` sections 6.2.6 `C_DATE`, 6.2.7
/// `C_TIME`, 6.2.8 `C_DATE_TIME` and 6.2.9 `C_DURATION` spell the pattern as
/// `C_DATE.pattern`; `AOM2.html` sections 4.5.18 to 4.5.21 spell it as
/// `C_TEMPORAL.pattern_constraint`. Both land here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalConstraint {
    /// The pattern saying which components the value must, may and must not
    /// carry.
    pub pattern: Option<String>,
    /// Every range the template admits, with each end as the template spells
    /// it.
    pub ranges: Vec<Bounds<String>>,
    /// Whether a timezone is mandatory, optional or disallowed, where the
    /// template says.
    pub timezone_validity: Option<TimezoneValidity>,
    /// The value assumed when an optional node is omitted.
    pub assumed_value: Option<String>,
}

/// Whether a temporal value must carry a timezone.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` sections 6.2.7 `C_TIME` and 6.2.8
/// `C_DATE_TIME`, whose `timezone_validity` is a `VALIDITY_KIND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimezoneValidity {
    /// A timezone is required.
    Mandatory,
    /// A timezone may be present.
    Optional,
    /// A timezone must be absent.
    Disallowed,
}

/// One state of a state machine, and the transitions out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateNode {
    /// The state's name.
    pub name: String,
    /// Whether the state machine ends here.
    pub is_terminal: bool,
    /// The transitions out of this state.
    pub transitions: Vec<StateTransition>,
}

/// One transition out of a state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransition {
    /// The event that fires the transition.
    pub event: String,
    /// The action the transition performs.
    pub action: Option<String>,
    /// The guard the transition is subject to.
    pub guard: Option<String>,
    /// The state the transition leads to.
    pub next_state: Option<String>,
}

/// A constraint on a state.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 4.2.3, `DV_STATE`. The
/// constrainer that expresses it, `C_DV_STATE`, is defined only in the ITS-XML
/// `OpenehrProfile.xsd` and by no AOM prose in either generation. See the
/// module note on [`crate::adl14`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateConstraint {
    /// The states, in the order the template lists them.
    pub states: Vec<StateNode>,
}

/// One assertion on an archetype slot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotAssertion {
    /// A regular expression over archetype identifiers.
    ArchetypeIdPattern(String),
    /// An archetype identifier the template names outright.
    ArchetypeId(String),
    /// An assertion this reader does not interpret, kept as text so nothing is
    /// dropped.
    Opaque(String),
}

/// A slot the template leaves open.
///
/// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 removes only `closed`
/// slots when it flattens, so an open slot legitimately survives into an
/// operational template as a runtime extension point. What goes there is
/// undetermined, so a form renders nothing for it and never guesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSlot {
    /// The archetypes the slot admits.
    pub includes: Vec<SlotAssertion>,
    /// The archetypes the slot refuses.
    pub excludes: Vec<SlotAssertion>,
}

/// Sibling attributes an AOM 2 archetype ties together, where the pairing is
/// not one this reader folds into a quantity or an ordinal.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.25 ties attributes with a
/// `C_ATTRIBUTE_TUPLE` whose rows are `C_PRIMITIVE_TUPLE`s, one row per
/// admitted combination. ADL 1.4 has no equivalent, so a tuple the reader does
/// not recognise is carried here rather than dropped, and the member
/// attributes remain the node's children.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleConstraint {
    /// The attributes the tuple ties together, in the order the template lists
    /// them.
    pub members: Vec<RmAttributeName>,
    /// One row per admitted combination; the cell at each position constrains
    /// the member at the same position.
    pub rows: Vec<Vec<ConstraintPayload>>,
}

/// What a node constrains.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConstraintPayload {
    /// A structural node: it holds children and constrains no value of its
    /// own.
    Structure,
    /// A boolean.
    Boolean(BooleanConstraint),
    /// An integer.
    Integer(IntegerConstraint),
    /// A real number.
    Real(RealConstraint),
    /// A string.
    Text(TextConstraint),
    /// A coded value.
    Coded(CodedConstraint),
    /// An ordinal or a scale.
    Ordinal(OrdinalConstraint),
    /// A quantity.
    Quantity(QuantityConstraint),
    /// A date.
    Date(TemporalConstraint),
    /// A time.
    Time(TemporalConstraint),
    /// A date and time.
    DateTime(TemporalConstraint),
    /// A duration.
    Duration(TemporalConstraint),
    /// A state.
    State(StateConstraint),
    /// A slot the template leaves open.
    OpenSlot(OpenSlot),
    /// Sibling attributes tied together in a shape this reader does not fold.
    Tuple(TupleConstraint),
}

impl ConstraintPayload {
    /// A short, stable name for the payload kind, for a report a person reads.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match *self {
            Self::Structure => "structure",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Real(_) => "real",
            Self::Text(_) => "text",
            Self::Coded(_) => "coded",
            Self::Ordinal(_) => "ordinal",
            Self::Quantity(_) => "quantity",
            Self::Date(_) => "date",
            Self::Time(_) => "time",
            Self::DateTime(_) => "date-time",
            Self::Duration(_) => "duration",
            Self::State(_) => "state",
            Self::OpenSlot(_) => "open-slot",
            Self::Tuple(_) => "tuple",
        }
    }
}
