// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The identifier newtypes the form definition is keyed by.
//!
//! Every one of these is a bare string on the wire, and swapping two of them
//! binds layout or a value to the wrong clinical concept, so each gets its own
//! type (`.claude/rules/reliability.md`).

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps `value` verbatim.
            ///
            /// The text is never normalized, because the operational template
            /// states it and the form definition repeats it.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The identifier as the template states it.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the identifier, returning the text it wraps.
            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id! {
    /// The identifier the operational template states for itself.
    ///
    /// openEHR BASE Release-1.2.0 `base_types.html` section 5, `TEMPLATE_ID`.
    TemplateId
}

string_id! {
    /// An openEHR archetype identifier, including its version part.
    ///
    /// openEHR BASE Release-1.2.0 `base_types.html` section 5, `ARCHETYPE_ID`.
    ArchetypeId
}

string_id! {
    /// A local code defined by one archetype's own terminology.
    LocalCode
}

string_id! {
    /// A Reference Model class name, as the template spells it.
    RmTypeName
}

string_id! {
    /// A Reference Model attribute name, as the template spells it.
    RmAttributeName
}

string_id! {
    /// The identifier of a terminology a code is drawn from.
    ///
    /// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3,
    /// `CODE_PHRASE.terminology_id`. The value `local` names the archetype's
    /// own terminology.
    TerminologyName
}

string_id! {
    /// An IETF language tag, as the template states it.
    LanguageTag
}

/// The terminology every openEHR null flavour is drawn from.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 4.1 draws the null
/// flavours from the openEHR terminology's `null flavours` group.
#[must_use]
pub fn openehr_terminology() -> TerminologyName {
    TerminologyName::new("openehr")
}

/// The terminology name an archetype's own codes carry.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3 reserves `local`
/// for codes the archetype itself defines.
#[must_use]
pub fn local_terminology() -> TerminologyName {
    TerminologyName::new("local")
}

#[cfg(test)]
mod tests {
    use super::{LanguageTag, local_terminology, openehr_terminology};

    #[test]
    fn an_identifier_round_trips_through_json_as_a_bare_string() {
        let tag = LanguageTag::new("en");
        let json = serde_json::to_string(&tag).expect("a language tag serializes");
        assert_eq!(json, r#""en""#);
        let back: LanguageTag = serde_json::from_str(&json).expect("a language tag deserializes");
        assert_eq!(back, tag);
    }

    #[test]
    fn the_two_reserved_terminology_names_are_the_ones_the_spec_reserves() {
        assert_eq!(local_terminology().as_str(), "local");
        assert_eq!(openehr_terminology().as_str(), "openehr");
    }
}
