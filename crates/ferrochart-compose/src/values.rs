// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a clinician entered, keyed the way the form definition is keyed.
//!
//! No specification governs this shape: our own design. It is the input to the
//! builder and the output of the read-back, so the two directions are inverses
//! over one type rather than over two that have to be kept in step.

use std::collections::BTreeMap;

use ferrochart_form::ids::LocalCode;
use ferrochart_form::key::NodeKey;

/// One entered value, or the reason there is none.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
/// `ELEMENT` the invariants `Inv_is_null_valid: is_null() = (value = Void)`
/// and `Inv_null_flavour_indicated: is_null() xor null_flavour = Void`, so an
/// element carries exactly one of a value and a null flavour: never both, and
/// never neither. This enum is that invariant as a type.
#[derive(Debug, Clone, PartialEq)]
pub enum Entered {
    /// The value the clinician entered.
    Value(Datum),
    /// The reason no value was entered.
    ///
    /// The code is one of the four in the openEHR `null flavours` group
    /// (`253` unknown, `271` no information, `272` masked, `273` not
    /// applicable), which `Inv_null_flavour_valid` requires.
    Null {
        /// The null flavour code.
        code: LocalCode,
        /// The specific reason, where one was given. `null_reason` is legal
        /// only beside a null flavour (`Inv_null_reason_valid`).
        reason: Option<String>,
    },
}

/// A value as the form holds it, before it becomes a Reference Model instance.
///
/// One variant per shape the derivation table produces, so a field kind and a
/// datum are checked against each other at the seam rather than deep inside a
/// serialiser.
#[derive(Debug, Clone, PartialEq)]
pub enum Datum {
    /// A `DV_BOOLEAN`.
    Boolean(bool),
    /// A `DV_TEXT` the clinician typed, or edited after choosing it.
    ///
    /// RM `data_types.html` section 5.1.2: "If the user makes even the
    /// slightest modification during data entry, a mapping to a `DV_TEXT`
    /// should be used instead" of a `DV_CODED_TEXT`.
    Text(String),
    /// A `DV_CODED_TEXT`: the code, and the rubric that code carries.
    Coded {
        /// The terminology the code belongs to.
        terminology: String,
        /// The code itself.
        code: String,
        /// The rubric, in the composition's language. RM section 5.2.4: a
        /// coded text is "the rubric of that term, from a terminology
        /// service, in the language in which the data were authored".
        rubric: String,
    },
    /// A `DV_ORDINAL`: the symbol's code, and the integer it ranks at.
    Ordinal {
        /// The symbol's terminology.
        terminology: String,
        /// The symbol's code.
        code: String,
        /// The symbol's rubric.
        rubric: String,
        /// The ordinal position. The Reference Model states no invariant
        /// tying this to the symbol; the template does.
        value: i64,
    },
    /// A `DV_SCALE`, which is a `DV_ORDINAL` whose value is real.
    Scale {
        /// The symbol's terminology.
        terminology: String,
        /// The symbol's code.
        code: String,
        /// The symbol's rubric.
        rubric: String,
        /// The scale position.
        value: f64,
    },
    /// A `DV_COUNT`.
    Count(i64),
    /// A `DV_QUANTITY`: a magnitude in one of the units the template permits.
    Quantity {
        /// The magnitude.
        magnitude: f64,
        /// The units, in UCUM syntax unless the template names another
        /// system.
        units: String,
        /// The number of decimal places, where the template fixed one.
        precision: Option<i32>,
    },
    /// A `DV_PROPORTION`.
    Proportion {
        /// The numerator.
        numerator: f64,
        /// The denominator, which `Valid_denominator` forbids from being
        /// zero.
        denominator: f64,
        /// The `PROPORTION_KIND` code.
        kind: i64,
        /// The number of decimal places, where the template fixed one.
        precision: Option<i32>,
    },
    /// A `DV_DATE`, as an ISO 8601 date, possibly partial.
    Date(String),
    /// A `DV_TIME`, as an ISO 8601 time, possibly partial.
    Time(String),
    /// A `DV_DATE_TIME`, as an ISO 8601 date and time, possibly partial.
    DateTime(String),
    /// A `DV_DURATION`, as an ISO 8601 duration.
    Duration(String),
    /// A `DV_IDENTIFIER`.
    Identifier {
        /// The identifier, which `Id_valid` forbids from being empty.
        id: String,
        /// Who issued it.
        issuer: Option<String>,
        /// Who assigned it.
        assigner: Option<String>,
        /// What kind of identifier it is.
        identifier_type: Option<String>,
    },
    /// A `DV_URI`, or a `DV_EHR_URI` where the field's class says so.
    Uri(String),
    /// A `DV_PARSABLE`.
    Parsable {
        /// The text, "which may validly be empty in some syntaxes".
        value: String,
        /// The formalism, which `Formalism_valid` forbids from being empty.
        formalism: String,
    },
    /// A `DV_INTERVAL`, whose ends are themselves data.
    Interval {
        /// The lower bound, absent where the interval is unbounded below.
        lower: Option<Box<Datum>>,
        /// The upper bound, absent where the interval is unbounded above.
        upper: Option<Box<Datum>>,
        /// Whether the lower bound is part of the interval.
        lower_included: bool,
        /// Whether the upper bound is part of the interval.
        upper_included: bool,
    },
}

/// Every value a clinician entered against one form definition.
///
/// A key names a field of the definition; an occurrence index names which
/// repeat of that field, because a repeatable group produces several data
/// nodes sharing one `archetype_node_id` (openEHR BASE Release-1.2.0
/// `architecture_overview.html` section 10.4: "a single archetype node may be
/// replicated in the data").
///
/// The map is ordered, so a build is byte-deterministic and a repeat keeps the
/// order the form put it in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormValues {
    entries: BTreeMap<Slot, Entered>,
}

/// One field of one occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slot {
    /// The field the value belongs to.
    pub key: NodeKey,
    /// Which repeat of that field, counting from zero. A field that cannot
    /// repeat always uses zero.
    pub occurrence: usize,
}

impl FormValues {
    /// An empty set of values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `entered` against the first occurrence of `key`.
    pub fn set(&mut self, key: NodeKey, entered: Entered) {
        self.set_at(key, 0, entered);
    }

    /// Records `entered` against occurrence `occurrence` of `key`.
    pub fn set_at(&mut self, key: NodeKey, occurrence: usize, entered: Entered) {
        self.entries.insert(Slot { key, occurrence }, entered);
    }

    /// What was entered against the first occurrence of `key`.
    #[must_use]
    pub fn get(&self, key: &NodeKey) -> Option<&Entered> {
        self.get_at(key, 0)
    }

    /// What was entered against occurrence `occurrence` of `key`.
    #[must_use]
    pub fn get_at(&self, key: &NodeKey, occurrence: usize) -> Option<&Entered> {
        self.entries.get(&Slot {
            key: key.clone(),
            occurrence,
        })
    }

    /// How many occurrences of `key` carry a value.
    ///
    /// Counts entries rather than the highest index, so a gap left by an
    /// editing session does not inflate the answer.
    #[must_use]
    pub fn occurrences_of(&self, key: &NodeKey) -> usize {
        self.entries.keys().filter(|slot| slot.key == *key).count()
    }

    /// Every slot and value, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&Slot, &Entered)> {
        self.entries.iter()
    }

    /// Whether nothing was entered at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many values were entered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
