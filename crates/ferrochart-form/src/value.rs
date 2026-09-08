// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Codes, value sets, and the values a template prefills a field with.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{LocalCode, TerminologyName};
use crate::text::Localized;

/// A code and the terminology that defines it.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3, `CODE_PHRASE`.
/// This is the pair every selection emits; `preferred_term` is display only,
/// so it is not part of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Code {
    /// The terminology the code is drawn from.
    pub terminology: TerminologyName,
    /// The code string, verbatim.
    pub code: String,
}

impl Code {
    /// The pair a selection emits.
    #[must_use]
    pub fn new(terminology: TerminologyName, code: impl Into<String>) -> Self {
        Self {
            terminology,
            code: code.into(),
        }
    }
}

/// One code a clinician may choose, with the display text the template
/// carries for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodedOption {
    /// What the composition stores.
    pub code: Code,
    /// The rubric the option is shown under.
    ///
    /// Empty when the template carries no rubric for the code, which is the
    /// case for a code from a terminology other than the archetype's own. A
    /// terminology server supplies the display text for those.
    pub label: Localized,
    /// The longer rubric, where the template carries one.
    pub description: Localized,
}

/// Where a coded field's permitted codes come from.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.14 (`CONSTRAINT_REF`)
/// and `ADL2.html` section 7.13.5.1 give the shapes a template names a set by;
/// which one a node carries decides whether the form needs a terminology
/// server at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValueSet {
    /// Every code the template lists, all from one terminology.
    Enumerated(EnumeratedSet),
    /// Any code from the named terminology; the template lists none.
    OpenTerminology {
        /// The terminology every value must come from.
        terminology: TerminologyName,
    },
    /// A set the template names but does not enumerate, which a terminology
    /// server expands at render time.
    Expansion(ExpansionSource),
    /// Neither a terminology nor a list: any code at all.
    Unconstrained,
}

/// A value set the template enumerates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumeratedSet {
    /// The terminology the codes are drawn from.
    pub terminology: TerminologyName,
    /// The codes, in the order the template lists them.
    pub options: Vec<CodedOption>,
    /// Whether any option is still missing its display text.
    ///
    /// Membership and display text are separate questions: a code from the
    /// archetype's own terminology carries its rubric in the template
    /// (openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2, where
    /// `ARCHETYPE_TERM.text` is mandatory), and an enumerated code from
    /// another terminology does not, so its display text comes from a
    /// terminology server rather than from here.
    pub needs_display_lookup: bool,
}

/// A value set a terminology server has to expand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExpansionSource {
    /// A set named by a local `ac`-code.
    ///
    /// ADL 1.4 spells this as a `CONSTRAINT_REF` whose reference names an
    /// entry in the archetype's `constraint_definitions`, resolved through
    /// `constraint_bindings` (openEHR AM Release-2.3.0 `AOM1.4.html`
    /// section 4.3.14). ADL 2 spells it as a `C_TERMINOLOGY_CODE` whose
    /// constraint is that `ac`-code (`ADL2.html` section 7.13.5.1).
    ConstraintCode {
        /// The `ac`-code naming the set.
        code: LocalCode,
        /// Every target the archetype binds the set to, keyed by terminology.
        bindings: BTreeMap<TerminologyName, String>,
        /// The terminology an ADL 2 operational binding pins the set to,
        /// where the template states one.
        pinned_terminology: Option<TerminologyName>,
    },
    /// A set named by URI.
    ///
    /// `C_CODE_REFERENCE.referenceSetUri` is declared only in the openEHR
    /// ITS-XML 2.0.0 `Template.xsd`; no AOM prose in either generation
    /// defines the class.
    ReferenceSet {
        /// The URI the template states.
        uri: String,
    },
}

/// How strictly a value set binds.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.10,
/// `C_PRIMITIVE_OBJECT.constraint_status`. ADL 1.4 has no way to say this, so
/// a field derived from an ADL 1.4 template leaves it unstated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BindingStrictness {
    /// The value must come from the set.
    Required,
    /// The set may be extended.
    Extensible,
    /// The set is preferred but not enforced.
    Preferred,
    /// The set is illustrative.
    Example,
    /// A status outside the four the specification names, kept as the integer
    /// the template states rather than dropped.
    Other(i32),
}

/// A value the template prefills a field with.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.3.2 keeps this distinct
/// from an assumed value: "default values do appear in data, while assumed
/// values don't". Only the former reaches a form, so a form definition carries
/// no assumed values at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Prefill {
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
        code: Code,
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
        symbol: Code,
    },
    /// An ISO 8601 date, time, date and time, or duration, as the template
    /// spells it.
    Temporal(String),
    /// A prefilled value in a shape this format does not model, kept as the
    /// text the template states so nothing is dropped.
    Opaque(String),
}

#[cfg(test)]
mod tests {
    use super::{Code, EnumeratedSet, Prefill, ValueSet};
    use crate::ids::{TerminologyName, local_terminology};

    #[test]
    fn a_value_set_serializes_under_the_name_of_its_source() {
        let set = ValueSet::OpenTerminology {
            terminology: TerminologyName::new("SNOMED-CT"),
        };
        let json = serde_json::to_string(&set).expect("a value set serializes");
        assert_eq!(json, r#"{"open_terminology":{"terminology":"SNOMED-CT"}}"#);
    }

    #[test]
    fn an_enumerated_local_set_round_trips() {
        let set = EnumeratedSet {
            terminology: local_terminology(),
            options: Vec::new(),
            needs_display_lookup: false,
        };
        let round: EnumeratedSet =
            serde_json::from_str(&serde_json::to_string(&set).expect("serializes"))
                .expect("deserializes");
        assert_eq!(round, set);
    }

    #[test]
    fn a_prefill_nests_under_the_name_of_its_variant() {
        let prefill = Prefill::Coded {
            code: Code::new(local_terminology(), "at0004"),
            rubric: Some("Sitting".to_owned()),
        };
        let json = serde_json::to_string(&prefill).expect("a prefill serializes");
        assert!(json.starts_with(r#"{"coded":{"#), "{json}");
    }
}
