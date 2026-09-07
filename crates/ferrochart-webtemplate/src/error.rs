// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What reading or writing a web template refuses, and why.
//!
//! Every refusal names the node it happened at, by the node's `aqlPath` where
//! the document states one and by its `id` otherwise, so a person holding a
//! third party's document can find the node the reader stopped on.

use thiserror::Error;

/// Why a web template could not be read into a form definition.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The bytes are not JSON.
    #[error("the document is not JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    /// The document, or a node of it, is not a JSON object.
    #[error("{at} is not a JSON object, and a web template states one there")]
    NotAnObject {
        /// Where in the document the value sits.
        at: String,
    },
    /// A member the document has to state is missing.
    #[error("{at} states no {member}, which a web template node always carries")]
    MissingMember {
        /// Where in the document the object sits.
        at: String,
        /// The member that is missing.
        member: &'static str,
    },
    /// A member is stated as the wrong kind of JSON value.
    #[error("{at} states {member} as a {found}, and a web template states it as {expected}")]
    WrongType {
        /// Where in the document the object sits.
        at: String,
        /// The member with the wrong kind of value.
        member: &'static str,
        /// What a web template states there.
        expected: &'static str,
        /// What this document states there.
        found: &'static str,
    },
    /// The `min` and `max` a node states are not a count of occurrences.
    ///
    /// The two are the flattened integers a web template carries rather than
    /// an occurrences interval to re-derive, so they are read as they are and
    /// a pair no count can hold is refused rather than repaired.
    #[error("{at} states min {min} and max {max}, which is not a count of occurrences")]
    ImpossibleOccurrences {
        /// Where in the document the node sits.
        at: String,
        /// The `min` the node states.
        min: i64,
        /// The `max` the node states.
        max: i64,
    },
    /// A node states both children and inputs.
    #[error("{at} states both children and inputs, and a web template node states one or neither")]
    ChildrenAndInputs {
        /// Where in the document the node sits.
        at: String,
    },
    /// A node's inputs do not have the shape its Reference Model class gives
    /// them.
    #[error("{at} is a {rm_type} whose inputs cannot be read: {reason}")]
    UnreadableInputs {
        /// Where in the document the node sits.
        at: String,
        /// The Reference Model class the node states.
        rm_type: String,
        /// What is wrong with the inputs.
        reason: String,
    },
    /// An input states a type this format does not define.
    #[error("{at} states the input type {found}, which this format does not define")]
    UnknownInputType {
        /// Where in the document the node sits.
        at: String,
        /// The type the input states.
        found: String,
    },
    /// A node states a proportion type the Reference Model does not name.
    ///
    /// openEHR RM Release-1.1.0 `data_types.html` section 6.2.11 names five
    /// `PROPORTION_KIND` values and no others.
    #[error("{at} states the proportion type {found}, which PROPORTION_KIND does not name")]
    UnknownProportionType {
        /// Where in the document the node sits.
        at: String,
        /// The proportion type the node states.
        found: String,
    },
}

/// Why a form definition could not be written as a web template.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WriteError {
    /// A proportion field admits a kind outside the five the Reference Model
    /// names, and a web template spells a proportion type by name.
    ///
    /// openEHR RM Release-1.1.0 `data_types.html` section 6.2.11 assigns an
    /// integer to each of the five, and the form definition keeps any other
    /// integer rather than dropping it, so it reaches here with no name to
    /// write. It is refused rather than silently left out.
    #[error("{at} admits the proportion kind {kind}, which PROPORTION_KIND does not name")]
    UnnameableProportionKind {
        /// The key of the field.
        at: String,
        /// The kind, as the form definition holds it.
        kind: String,
    },
}
