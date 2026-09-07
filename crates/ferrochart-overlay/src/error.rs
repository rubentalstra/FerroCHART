// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the overlay store refuses, and why.
//!
//! Every refusal names the entry, the section or the node it is about, so a
//! form author reads a sentence rather than a stack trace. Nothing here is
//! absorbed into a default: an overlay that does not describe a form the
//! definition has is refused rather than silently trimmed.

use thiserror::Error;

/// What the overlay store refuses.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OverlayError {
    /// The document states a format version this crate does not read.
    #[error("the overlay states format version {stated}, and this build reads version {supported}")]
    UnsupportedFormatVersion {
        /// The version the document states.
        stated: u32,
        /// The version this build reads.
        supported: u32,
    },
    /// The key names no group and no field of the form definition.
    #[error("no group and no field of template {template} is at {key}")]
    NoSuchNode {
        /// The template the definition was compiled from.
        template: String,
        /// The key, as a person reads it.
        key: String,
    },
    /// The definition was compiled from a different template than the overlay
    /// was keyed against.
    ///
    /// A revision of the same template keeps its identifier, so a differing
    /// identifier is a different form rather than a newer one.
    #[error("the overlay is keyed against template {overlay}, and the definition is {definition}")]
    TemplateMismatch {
        /// The identifier the overlay was keyed against.
        overlay: String,
        /// The identifier the definition carries.
        definition: String,
    },
    /// Two sections of the document share one identifier.
    #[error("two sections share the identifier {section}")]
    DuplicateSection {
        /// The identifier both sections carry.
        section: String,
    },
    /// A section or an entry names a section the document does not define.
    #[error("{referrer} names section {section}, which the overlay does not define")]
    UnknownSection {
        /// What names the missing section.
        referrer: String,
        /// The identifier that resolves to nothing.
        section: String,
    },
    /// A chain of section parents leads back to where it started.
    #[error("section {section} is its own ancestor")]
    SectionCycle {
        /// One section on the cycle.
        section: String,
    },
    /// Two entries of the document decorate one node.
    #[error("two entries decorate the node at {key}")]
    DuplicateEntry {
        /// The key both entries carry.
        key: String,
    },
    /// An entry disagrees with itself about whether it is positionally keyed.
    ///
    /// A positionally keyed entry carries the anchor its replay needs, and an
    /// entry that is not positionally keyed carries none. An entry that
    /// carries one without the other cannot be replayed as either.
    #[error(
        "the entry at {key} is marked positionally keyed = {is_positional} and carries an anchor = {has_anchor}"
    )]
    AnchorMismatch {
        /// The key of the entry that disagrees with itself.
        key: String,
        /// Whether the entry's key is marked positionally keyed.
        is_positional: bool,
        /// Whether the entry carries an anchor.
        has_anchor: bool,
    },
    /// The document is not the JSON this format is written in.
    #[error("the overlay document does not read as this format")]
    Malformed {
        /// What the JSON reader refused.
        #[source]
        source: serde_json::Error,
    },
    /// The overlay could not be written as JSON.
    #[error("the overlay does not write as JSON")]
    NotSerializable {
        /// What the JSON writer refused.
        #[source]
        source: serde_json::Error,
    },
}
