// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the field derivation refuses, and why.
//!
//! A constraint the derivation does not understand is refused by name. It is
//! never absorbed into a permissive field: a form that admits what the
//! derivation failed to read is a form that admits what the template refuses,
//! and the CDR rejection that follows would be this compiler's bug.

/// A failure while deriving a form definition from the internal constraint
/// model.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DeriveError {
    /// A node collects a Reference Model class the derivation has no field
    /// for.
    #[error("{path} collects {rm_type}, which the field derivation does not model")]
    UnmodelledDataValue {
        /// Where the node sits in the template.
        path: String,
        /// The Reference Model class the node constrains.
        rm_type: String,
    },

    /// A data value carries a constrained attribute the derivation has no
    /// field for.
    ///
    /// Dropping the constraint would widen the field past what the template
    /// admits, so the template is refused instead.
    #[error(
        "{path} constrains {rm_type}.{attribute}, which the field derivation does not model; \
         dropping it would let the form admit what the template refuses"
    )]
    UnmodelledAttribute {
        /// Where the node sits in the template.
        path: String,
        /// The Reference Model class that carries the attribute.
        rm_type: String,
        /// The attribute the template constrains.
        attribute: String,
    },

    /// A node carries a constraint that cannot sit under its Reference Model
    /// type.
    #[error(
        "{path} constrains {rm_type} with a {payload} constraint, \
         which cannot sit under that Reference Model type"
    )]
    PayloadMismatch {
        /// Where the node sits in the template.
        path: String,
        /// The Reference Model class the node constrains.
        rm_type: String,
        /// The constraint the node carries, by the internal model's name for
        /// it.
        payload: &'static str,
    },

    /// A data value ties its attributes into more combinations than one.
    ///
    /// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.25 gives a
    /// `C_ATTRIBUTE_TUPLE` one row per admitted combination. A single row is
    /// the same thing as a constraint per attribute, and the derivation reads
    /// it as one; more than one row is a constraint across attributes that a
    /// form definition cannot express, and widening the field to the union of
    /// the rows would admit combinations the template refuses.
    #[error(
        "{path} ties the attributes of {rm_type} into {rows} admitted combinations, \
         and a form field carries a constraint per attribute rather than across them"
    )]
    UnfoldedTuple {
        /// Where the node sits in the template.
        path: String,
        /// The Reference Model class the node constrains.
        rm_type: String,
        /// How many combinations the template admits.
        rows: usize,
    },

    /// The template root is not something a form can be rooted in.
    #[error("the template root at {path} constrains {rm_type}, which is not a form group")]
    RootIsNotAGroup {
        /// Where the root sits.
        path: String,
        /// The Reference Model class the root constrains.
        rm_type: String,
    },
}
