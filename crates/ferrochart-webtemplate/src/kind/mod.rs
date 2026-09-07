// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A node's `inputs` and what a clinician fills, in both directions.
//!
//! A web template describes a leaf by an `inputs` array whose entries carry a
//! FLAT key suffix and a value type, and the form definition describes the
//! same leaf by one [`ferrochart_form::field::FieldKind`]. This module is the
//! mapping between them, written once so that reading and writing cannot
//! drift apart.
//!
//! The web template is a compatibility target, so every row of the mapping is
//! an observation of the published implementations. Where the two disagree the
//! openEHR Reference Model wins, and the class each row belongs to is the
//! Reference Model class the node states in `rmType`.

pub(crate) mod range;
pub(crate) mod read;
pub(crate) mod temporal;
pub(crate) mod write;

use serde_json::{Map, Value};

/// The value type a web template input states.
///
/// The names are the `inputs[].type` values the published implementations
/// emit. No openEHR specification defines the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputType {
    Text,
    CodedText,
    Date,
    Time,
    Datetime,
    Boolean,
    Integer,
    Decimal,
    Duration,
    Quantity,
    Count,
    Proportion,
}

impl InputType {
    /// The type `name` states, or `None` where it is not one of them.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "TEXT" => Some(Self::Text),
            "CODED_TEXT" => Some(Self::CodedText),
            "DATE" => Some(Self::Date),
            "TIME" => Some(Self::Time),
            "DATETIME" => Some(Self::Datetime),
            "BOOLEAN" => Some(Self::Boolean),
            "INTEGER" => Some(Self::Integer),
            "DECIMAL" => Some(Self::Decimal),
            "DURATION" => Some(Self::Duration),
            "QUANTITY" => Some(Self::Quantity),
            "COUNT" => Some(Self::Count),
            "PROPORTION" => Some(Self::Proportion),
            _ => None,
        }
    }

    /// How a web template spells this type.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::CodedText => "CODED_TEXT",
            Self::Date => "DATE",
            Self::Time => "TIME",
            Self::Datetime => "DATETIME",
            Self::Boolean => "BOOLEAN",
            Self::Integer => "INTEGER",
            Self::Decimal => "DECIMAL",
            Self::Duration => "DURATION",
            Self::Quantity => "QUANTITY",
            Self::Count => "COUNT",
            Self::Proportion => "PROPORTION",
        }
    }
}

/// One entry of a node's `inputs`, read.
#[derive(Debug)]
pub(crate) struct ParsedInput<'a> {
    /// The FLAT key suffix this input fills, or `None` for the node's bare
    /// value.
    pub(crate) suffix: Option<&'a str>,
    /// The value type.
    pub(crate) input_type: InputType,
    /// The coded options the input enumerates.
    pub(crate) list: Vec<ParsedOption<'a>>,
    /// Whether the list admits a value outside itself.
    pub(crate) list_open: Option<bool>,
    /// The `validation` object, unread.
    pub(crate) validation: Option<&'a Map<String, Value>>,
    /// The terminology the input names.
    pub(crate) terminology: Option<&'a str>,
}

/// One entry of an input's `list`, read.
#[derive(Debug)]
pub(crate) struct ParsedOption<'a> {
    /// The code the option stores.
    pub(crate) value: &'a str,
    /// The display label in the document's default language.
    pub(crate) label: Option<&'a str>,
    /// The `localizedLabels` object.
    pub(crate) localized_labels: Option<&'a Map<String, Value>>,
    /// The `localizedDescriptions` object.
    pub(crate) localized_descriptions: Option<&'a Map<String, Value>>,
    /// The `validation` object, unread.
    pub(crate) validation: Option<&'a Map<String, Value>>,
    /// The `DV_ORDINAL` score.
    pub(crate) ordinal: Option<i64>,
    /// The `DV_SCALE` score.
    pub(crate) scale: Option<f64>,
}

/// The Reference Model class a node states, with any generic argument
/// stripped.
///
/// A web template spells an interval's class with its element class,
/// `DV_INTERVAL<DV_COUNT>`, and the row a node maps to is decided by the class
/// itself.
pub(crate) fn base_rm_type(rm_type: &str) -> &str {
    rm_type.split('<').next().unwrap_or(rm_type)
}

#[cfg(test)]
mod tests {
    use super::{InputType, base_rm_type};

    #[test]
    fn an_input_type_reads_back_as_the_name_it_writes() {
        for name in [
            "TEXT",
            "CODED_TEXT",
            "DATE",
            "TIME",
            "DATETIME",
            "BOOLEAN",
            "INTEGER",
            "DECIMAL",
            "DURATION",
            "QUANTITY",
            "COUNT",
            "PROPORTION",
        ] {
            let parsed = InputType::from_name(name).expect("a name this format states");
            assert_eq!(parsed.name(), name);
        }
        assert_eq!(InputType::from_name("SLIDER"), None);
    }

    #[test]
    fn a_generic_argument_does_not_change_the_class() {
        assert_eq!(base_rm_type("DV_INTERVAL<DV_COUNT>"), "DV_INTERVAL");
        assert_eq!(base_rm_type("DV_QUANTITY"), "DV_QUANTITY");
    }
}
