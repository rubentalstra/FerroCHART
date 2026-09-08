// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What can go wrong in either direction.
//!
//! A COMPOSITION FerroCHART built and a CDR then refused is always a
//! FerroCHART bug (`CLAUDE.md`), so nothing here is a soft failure: a value
//! the builder cannot represent is refused by name rather than dropped, and a
//! node the reader cannot place is reported rather than skipped.

use ferrochart_form::key::NodeKey;
use thiserror::Error;

/// A failure while building a COMPOSITION from form values.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BuildError {
    /// A value was entered against a field the definition does not have.
    ///
    /// Building it anyway would put data at a path the template never
    /// permitted, which the CDR would refuse and which would be our bug.
    #[error("the form definition has no field at `{key}`")]
    UnknownField {
        /// The key the value was entered against.
        key: NodeKey,
    },

    /// The datum entered does not fit the field's kind.
    #[error("the field at `{key}` is {expected}, and the value entered is {found}")]
    WrongDatum {
        /// The field.
        key: NodeKey,
        /// What the field derives to.
        expected: &'static str,
        /// What was entered.
        found: &'static str,
    },

    /// A field the template requires carries neither a value nor a null
    /// flavour.
    ///
    /// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
    /// `ELEMENT` no legal state with both absent, so there is nothing to
    /// build.
    #[error("the field at `{key}` is required and carries neither a value nor a null flavour")]
    Missing {
        /// The field.
        key: NodeKey,
    },

    /// More or fewer occurrences than the template permits.
    #[error("the field at `{key}` permits {permitted} occurrences and carries {found}")]
    Occurrences {
        /// The field.
        key: NodeKey,
        /// What the template permits, as the template spells it.
        permitted: String,
        /// How many were entered.
        found: usize,
    },

    /// A failure inside one occurrence of a field.
    ///
    /// The builder walks a form with the occurrence of every repeating group
    /// above it, so at the point a value is refused it knows which repeat the
    /// value came from. The inner error says what is wrong; this one says
    /// where, so a renderer draws the refusal on the control the clinician
    /// typed into rather than on every repeat of that field (issue #152).
    #[error("{source}")]
    At {
        /// Which occurrence of each repeating ancestor group, outermost
        /// first.
        group_path: Vec<usize>,
        /// Which repeat of the field itself.
        occurrence: usize,
        /// What is wrong.
        #[source]
        source: Box<BuildError>,
    },

    /// A null flavour outside the openEHR `null flavours` group.
    ///
    /// `Inv_null_flavour_valid` tests membership of that group, which has
    /// four members: `253`, `271`, `272` and `273`.
    #[error("`{code}` is not a null flavour the openEHR terminology defines")]
    UnknownNullFlavour {
        /// The code that was given.
        code: String,
    },

    /// A `category` outside the openEHR `composition category` group.
    ///
    /// The group has four members in TERM Release-3.0.0, and `ehr.html`
    /// section 5.4.1 admits "any other code defined in" it, so this refuses
    /// only what the group does not carry.
    #[error("`{code}` is not a composition category the openEHR terminology defines")]
    UnknownCategory {
        /// The code that was given.
        code: String,
    },

    /// A value the Reference Model refuses.
    ///
    /// The invariant is named, because it is what a reader has to go and
    /// read.
    #[error("the value at `{key}` violates {invariant}: {detail}")]
    Invariant {
        /// The field.
        key: NodeKey,
        /// The Reference Model invariant, as the specification names it.
        invariant: &'static str,
        /// What was wrong.
        detail: String,
    },
}

/// A failure while reading a COMPOSITION back into form values.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The document names a different template from the definition.
    #[error("the composition was built from template `{found}` and the form is for `{expected}`")]
    WrongTemplate {
        /// The template the form definition compiles.
        expected: String,
        /// The template the composition names.
        found: String,
    },

    /// A data node matched no field of the definition.
    ///
    /// This is reported rather than dropped: content the form does not cover
    /// is a fact about the pair, and losing it silently would let an edit
    /// round trip delete a colleague's data.
    #[error("the composition carries content at `{path}` that the form definition does not cover")]
    Uncovered {
        /// The Reference Model path of the node.
        path: String,
    },

    /// A node carried a value shape the field does not derive to.
    #[error("the node at `{path}` is {found}, and the field there is {expected}")]
    WrongShape {
        /// The Reference Model path of the node.
        path: String,
        /// What the field derives to.
        expected: &'static str,
        /// What the node carries.
        found: String,
    },
}
