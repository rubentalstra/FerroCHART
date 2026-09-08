// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a clinician entered, keyed the way the form definition is keyed.
//!
//! No specification governs this shape: our own design. It is the input to the
//! composition builder and the output of the read-back, so the two directions
//! are inverses over one type rather than over two that have to be kept in
//! step.
//!
//! It lives beside the definition rather than beside the builder for the
//! reason `docs/architecture.md` section 11 gives for the layout types: a
//! renderer has to COLLECT values and has no business with the builder, the
//! read-back, or the envelope. That keeps the browser on one crate of this
//! tree, which is what makes the form definition a contract a third party can
//! implement against.
//!
//! # The wire
//!
//! The values are published beside the definition and carry the same format
//! number, [`crate::definition::FORMAT_VERSION`]. A [`FormValues`] is a JSON
//! ARRAY of [`ValueEntry`], because its map is keyed by a struct and a JSON
//! object cannot be. Every entry spells the three parts of its [`Slot`] out,
//! the occurrence path among them, so a value that belongs to the second
//! repeat of a group comes back to the second repeat of that group.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::LocalCode;
use crate::key::NodeKey;

/// One entered value, or the reason there is none.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
/// `ELEMENT` the invariants `Inv_is_null_valid: is_null() = (value = Void)`
/// and `Inv_null_flavour_indicated: is_null() xor null_flavour = Void`, so an
/// element carries exactly one of a value and a null flavour: never both, and
/// never neither. This enum is that invariant as a type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
/// A key names a field of the definition. Two indices say WHICH one: the
/// occurrence path names which repeat of each repeating group above the
/// field, and the occurrence names which repeat of the field itself. Both are
/// needed because a repeatable group produces several data nodes sharing one
/// `archetype_node_id` (openEHR BASE Release-1.2.0
/// `architecture_overview.html` section 10.4: "a single archetype node may be
/// replicated in the data").
///
/// The map is ordered, so a build is byte-deterministic and a repeat keeps the
/// order the form put it in. On the wire it is an array of [`ValueEntry`] in
/// that same order, because a map keyed by a struct is not a JSON object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Vec<ValueEntry>", into = "Vec<ValueEntry>")]
pub struct FormValues {
    entries: BTreeMap<Slot, Entered>,
}

/// One slot and what was entered against it, as the wire carries the pair.
///
/// The three parts of the [`Slot`] are spelled out beside the value rather
/// than nested, so a reader of the document sees the occurrence path without
/// descending into a member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueEntry {
    /// The field the value belongs to.
    pub key: NodeKey,
    /// Which occurrence of each repeating ancestor group, outermost first.
    pub group_path: Vec<usize>,
    /// Which repeat of the field itself, counting from zero.
    pub occurrence: usize,
    /// What was entered.
    pub entered: Entered,
}

impl From<FormValues> for Vec<ValueEntry> {
    fn from(values: FormValues) -> Self {
        values
            .entries
            .into_iter()
            .map(|(slot, entered)| ValueEntry {
                key: slot.key,
                group_path: slot.group_path,
                occurrence: slot.occurrence,
                entered,
            })
            .collect()
    }
}

impl From<Vec<ValueEntry>> for FormValues {
    /// Reads the array back into the ordered map.
    ///
    /// Two entries naming the same slot collapse to the last one, which is
    /// what [`FormValues::set_in`] does with the same pair.
    fn from(entries: Vec<ValueEntry>) -> Self {
        let mut values = Self::new();
        for entry in entries {
            values.set_in(entry.key, entry.group_path, entry.occurrence, entry.entered);
        }
        values
    }
}

/// One field of one occurrence, inside one occurrence of each repeating group
/// above it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Slot {
    /// The field the value belongs to.
    pub key: NodeKey,
    /// Which occurrence of each REPEATING ancestor group, outermost first.
    /// Empty where no ancestor of the field repeats.
    ///
    /// One index per repeating ancestor rather than one per ancestor. A
    /// [`NodeKey`] is already a step chain from the root, so which ancestors
    /// there are is a fact of the definition; which repeat of each is the
    /// only thing the definition cannot supply. Counting only the repeating
    /// ones also means adding a non-repeating group to a template does not
    /// shift every path beneath it.
    pub group_path: Vec<usize>,
    /// Which repeat of the field itself, counting from zero. A field that
    /// cannot repeat always uses zero.
    pub occurrence: usize,
}

impl FormValues {
    /// An empty set of values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `entered` against the first occurrence of `key`, under no
    /// repeating group.
    pub fn set(&mut self, key: NodeKey, entered: Entered) {
        self.set_at(key, 0, entered);
    }

    /// Records `entered` against occurrence `occurrence` of `key`, under no
    /// repeating group.
    pub fn set_at(&mut self, key: NodeKey, occurrence: usize, entered: Entered) {
        self.set_in(key, Vec::new(), occurrence, entered);
    }

    /// Records `entered` against one field of one occurrence path.
    pub fn set_in(
        &mut self,
        key: NodeKey,
        group_path: Vec<usize>,
        occurrence: usize,
        entered: Entered,
    ) {
        self.entries.insert(
            Slot {
                key,
                group_path,
                occurrence,
            },
            entered,
        );
    }

    /// What was entered against the first occurrence of `key`, under no
    /// repeating group.
    #[must_use]
    pub fn get(&self, key: &NodeKey) -> Option<&Entered> {
        self.get_at(key, 0)
    }

    /// What was entered against occurrence `occurrence` of `key`, under no
    /// repeating group.
    #[must_use]
    pub fn get_at(&self, key: &NodeKey, occurrence: usize) -> Option<&Entered> {
        self.get_in(key, &[], occurrence)
    }

    /// What was entered against one field of one occurrence path.
    #[must_use]
    pub fn get_in(
        &self,
        key: &NodeKey,
        group_path: &[usize],
        occurrence: usize,
    ) -> Option<&Entered> {
        self.entries.get(&Slot {
            key: key.clone(),
            group_path: group_path.to_vec(),
            occurrence,
        })
    }

    /// How many occurrences of `key` carry a value, across every occurrence
    /// path.
    ///
    /// Counts entries rather than the highest index, so a gap left by an
    /// editing session does not inflate the answer.
    #[must_use]
    pub fn occurrences_of(&self, key: &NodeKey) -> usize {
        self.entries.keys().filter(|slot| slot.key == *key).count()
    }

    /// How many occurrences of `key` carry a value under one occurrence path.
    #[must_use]
    pub fn occurrences_in(&self, key: &NodeKey, group_path: &[usize]) -> usize {
        self.entries
            .keys()
            .filter(|slot| slot.key == *key && slot.group_path == group_path)
            .count()
    }

    /// Which occurrences of `group` the values name, under the occurrence
    /// path `prefix` of the repeating groups above it.
    ///
    /// Ascending and without repeats. Empty says no value names the group,
    /// which is not the same as one occurrence carrying no value: the builder
    /// tells the two apart, because a group the template requires still has
    /// to be built.
    #[must_use]
    pub fn group_occurrences(&self, group: &NodeKey, prefix: &[usize]) -> Vec<usize> {
        let depth = prefix.len();
        let mut found: Vec<usize> = self
            .entries
            .keys()
            .filter(|slot| slot.key.steps.starts_with(&group.steps))
            .filter(|slot| slot.group_path.starts_with(prefix))
            .filter_map(|slot| slot.group_path.get(depth).copied())
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    /// Forgets what was entered at one address, returning it.
    #[must_use = "the value that was forgotten; discard it deliberately"]
    pub fn remove_in(
        &mut self,
        key: &NodeKey,
        group_path: &[usize],
        occurrence: usize,
    ) -> Option<Entered> {
        self.entries.remove(&Slot {
            key: key.clone(),
            group_path: group_path.to_vec(),
            occurrence,
        })
    }

    /// Forgets everything entered inside one occurrence of one group.
    ///
    /// A field is inside it when its key descends from the group's and its
    /// occurrence path starts with the instance's, which is the same pair of
    /// tests [`FormValues::group_occurrences`] makes.
    pub fn remove_under(&mut self, group: &NodeKey, inside: &[usize]) {
        self.entries.retain(|slot, _| {
            !(slot.key.steps.starts_with(&group.steps) && slot.group_path.starts_with(inside))
        });
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

#[cfg(test)]
mod tests {
    use super::{Datum, Entered, FormValues};
    use crate::ids::{RmAttributeName, RmTypeName};
    use crate::key::{KeyStep, NodeKey};

    /// A key one step deep under `attribute`, which is all these tests need
    /// to tell two fields apart.
    fn key(attribute: &str) -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    /// A key two steps deep, so one key descends from another.
    fn nested(outer: &str, inner: &str) -> NodeKey {
        key(outer).child(KeyStep {
            rm_attribute: RmAttributeName::new(inner),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    fn text(what: &str) -> Entered {
        Entered::Value(Datum::Text(what.to_owned()))
    }

    #[test]
    fn a_removed_value_is_returned_and_gone() {
        let mut values = FormValues::new();
        values.set_in(key("items"), vec![1], 0, text("entered"));
        assert_eq!(
            values.remove_in(&key("items"), &[1], 0),
            Some(text("entered"))
        );
        assert_eq!(values.get_in(&key("items"), &[1], 0), None);
        assert!(values.is_empty());
    }

    #[test]
    fn removing_one_address_leaves_every_other_one_alone() {
        let mut values = FormValues::new();
        values.set_in(key("items"), vec![0], 0, text("first instance"));
        values.set_in(key("items"), vec![1], 0, text("second instance"));
        values.set_in(key("items"), vec![1], 1, text("second repeat"));
        assert_eq!(
            values.remove_in(&key("items"), &[1], 0),
            Some(text("second instance")),
            "the address removed is the one asked for"
        );
        assert_eq!(
            values.get_in(&key("items"), &[0], 0),
            Some(&text("first instance"))
        );
        assert_eq!(
            values.get_in(&key("items"), &[1], 1),
            Some(&text("second repeat"))
        );
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn removing_an_address_nothing_was_entered_at_is_not_an_error() {
        let mut values = FormValues::new();
        assert_eq!(values.remove_in(&key("items"), &[0], 0), None);
    }

    #[test]
    fn removing_one_instance_of_a_group_takes_everything_inside_it() {
        let mut values = FormValues::new();
        // Two occurrences of one group, each holding one field.
        values.set_in(nested("items", "value"), vec![0], 0, text("kept"));
        values.set_in(nested("items", "value"), vec![1], 0, text("discarded"));
        values.remove_under(&key("items"), &[1]);
        assert_eq!(
            values.get_in(&nested("items", "value"), &[0], 0),
            Some(&text("kept"))
        );
        assert_eq!(values.get_in(&nested("items", "value"), &[1], 0), None);
    }

    #[test]
    fn removing_one_instance_leaves_a_field_that_only_looks_nested() {
        let mut values = FormValues::new();
        // The same occurrence path under a DIFFERENT group. The path alone
        // would match, so the key test is what keeps this one.
        values.set_in(nested("other", "value"), vec![1], 0, text("elsewhere"));
        values.remove_under(&key("items"), &[1]);
        assert_eq!(
            values.get_in(&nested("other", "value"), &[1], 0),
            Some(&text("elsewhere"))
        );
    }

    #[test]
    fn removing_an_outer_instance_takes_every_instance_nested_inside_it() {
        let mut values = FormValues::new();
        for outer in 0..2 {
            for inner in 0..2 {
                values.set_in(
                    nested("items", "value"),
                    vec![outer, inner],
                    0,
                    text("entered"),
                );
            }
        }
        values.remove_under(&key("items"), &[1]);
        assert_eq!(values.len(), 2, "only the first outer instance survives");
        assert!(
            values
                .iter()
                .all(|(slot, _)| slot.group_path.first() == Some(&0))
        );
    }
}
