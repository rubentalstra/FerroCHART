// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The identifier newtypes the internal constraint model is keyed by.
//!
//! Every one of these is a bare string on the wire, and swapping two of them
//! is the kind of mistake that binds layout to the wrong clinical concept, so
//! each gets its own type (`.claude/rules/reliability.md`).

use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Wraps `value` verbatim. The text is never normalized, because
            /// the template states it and the model repeats it.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The identifier as it appears in the template.
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
    /// The identifier an operational template states for itself.
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
    ///
    /// The three ADL 2 code prefixes and the single ADL 1.4 one share this
    /// type on purpose: see [`LocalCode::kind`].
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

/// The role a local code plays, read from its prefix.
///
/// ADL 1.4 spends `at`-codes on both node identity and coded values
/// (openEHR AM Release-2.3.0, `ADL1.4.html` section 7.2). ADL 2 splits the two
/// and adds a third: `id`-codes identify nodes, `at`-codes identify values, and
/// `ac`-codes name value sets (openEHR AM Release-2.3.0, `ADL2.html`
/// section 7.13.5.1).
///
/// The mapping the two readers agree on: whatever occupies a node's identity
/// slot is that node's [`LocalCode`], so an ADL 1.4 `at`-code and an ADL 2
/// `id`-code both land in [`NodeIdentity::node_id`]; an `at`-code inside a
/// coded value's permitted set lands in the constraint payload in both
/// generations; and an `ac`-code lands in the payload as the name of a set the
/// terminology resolves.
///
/// [`NodeIdentity::node_id`]: crate::model::node::NodeIdentity::node_id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CodeKind {
    /// An `id`-code: node identity in ADL 2.
    Node,
    /// An `at`-code: node identity in ADL 1.4, a coded value in both.
    Term,
    /// An `ac`-code: the name of a value set.
    ValueSet,
    /// A code outside the three reserved prefixes, including the empty code a
    /// leaf with no siblings may carry (openEHR AM Release-2.3.0,
    /// `AOM1.4.html` section 4.2.3.1).
    Other,
}

impl LocalCode {
    /// The role this code plays, read from its prefix.
    #[must_use]
    pub fn kind(&self) -> CodeKind {
        // A prefix is only a prefix when a digit follows it, so a term whose
        // text happens to start with "at" is not mistaken for a code.
        let digit_after = |prefix: &str| {
            self.0
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        };
        if digit_after("id") {
            CodeKind::Node
        } else if digit_after("at") {
            CodeKind::Term
        } else if digit_after("ac") {
            CodeKind::ValueSet
        } else {
            CodeKind::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeKind, LocalCode};

    #[test]
    fn the_three_adl2_prefixes_are_told_apart() {
        assert_eq!(LocalCode::new("id2").kind(), CodeKind::Node);
        assert_eq!(LocalCode::new("at0004").kind(), CodeKind::Term);
        assert_eq!(LocalCode::new("ac1").kind(), CodeKind::ValueSet);
    }

    #[test]
    fn a_prefix_without_a_digit_is_not_a_code() {
        assert_eq!(LocalCode::new("atrial").kind(), CodeKind::Other);
        assert_eq!(LocalCode::new("").kind(), CodeKind::Other);
    }
}
