// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a template reader refuses, and why.
//!
//! Every variant names the node it refused, by the path a person can find in
//! the template. An upstream failure is never flattened into an empty model.

use crate::model::multiplicity::Multiplicity;

/// The deepest node tree either reader composes before it refuses.
///
/// No specification governs this bound; it is FerroCHART's own design, and it
/// exists so a template whose internal references chase each other fails with
/// a typed error rather than exhausting the stack.
pub const MAX_DEPTH: usize = 256;

/// A failure while reading an operational template into the internal
/// constraint model.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The operational template XML did not parse.
    #[error("the operational template XML did not parse")]
    Xml(#[source] Box<openehr_its::xml::runtime::XmlError>),

    /// The ADL 2 source did not parse.
    #[error("the ADL 2 source did not parse: {} error(s), the first is: {first}", .all.len())]
    AdlSyntax {
        /// The first error, which is this error's cause.
        #[source]
        first: Box<openehr_adl::error::SyntaxError>,
        /// Every error the parser reported, in the order it found them.
        all: Vec<openehr_adl::error::SyntaxError>,
    },

    /// The ADL 2 specialisation lineage could not be flattened.
    #[error("the ADL 2 specialisation lineage could not be flattened")]
    Flatten(#[source] Box<openehr_adl::flatten::FlattenError>),

    /// The operational template could not be generated from the ADL 2 source.
    #[error("the operational template could not be generated from the ADL 2 source")]
    Opt(#[source] Box<openehr_adl::opt::OptError>),

    /// The template requires content at a slot it never filled.
    ///
    /// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 fills a slot by
    /// inlining the archetype that fills it. A slot the template still leaves
    /// open holds undetermined content, so a form cannot render a field for
    /// it. Where the occurrences make that content mandatory, no form this
    /// compiler produces can satisfy the template, and the template is
    /// refused rather than compiled into something that a CDR will reject.
    #[error(
        "{path} requires an unfilled {rm_type} archetype slot (node {node_id}, occurrences {occurrences}); \
         a form cannot render undetermined content, and omitting it would not satisfy the template"
    )]
    RequiredSlotUnfilled {
        /// Where the slot sits in the template.
        path: String,
        /// The slot's node identifier.
        node_id: String,
        /// The Reference Model type the slot constrains.
        rm_type: String,
        /// How many the template requires.
        occurrences: Multiplicity,
    },

    /// An internal reference names a node the template does not define.
    ///
    /// openEHR AM Release-2.3.0 `OPT2.html` section 3.3 replaces every
    /// `use_node` internal reference with an inline copy of its target. The
    /// ADL 1.4 flattener is a vendor tool and does not always do so, so the
    /// reader resolves any it still meets against the same template. A target
    /// that is not there means the template is defective.
    #[error(
        "the internal reference at {path} targets {target}, which the template does not define"
    )]
    UnresolvedInternalReference {
        /// Where the reference sits in the template.
        path: String,
        /// The path the reference targets.
        target: String,
    },

    /// A default value names a node the template does not define.
    #[error("the default value at {path} names a node the template does not define")]
    UnresolvedDefaultPath {
        /// The differential path the template states.
        path: String,
    },

    /// A count interval carries a bound no count can take.
    #[error("{path} states the count interval bound {bound}, which is not a count")]
    NegativeMultiplicity {
        /// Where the interval sits in the template.
        path: String,
        /// The bound the template states.
        bound: i32,
    },

    /// The template nests deeper than [`MAX_DEPTH`].
    #[error("the template nests deeper than the limit of {MAX_DEPTH} levels, at {path}")]
    TooDeep {
        /// Where the walk gave up.
        path: String,
    },

    /// A node carries a constraint the reader cannot place under its Reference
    /// Model type.
    ///
    /// This is never absorbed into a permissive field: a form that admits what
    /// the reader did not understand is a form that admits what the template
    /// refuses.
    #[error("{path} constrains {rm_type} with {constrainer}, which this reader cannot interpret")]
    UninterpretableConstraint {
        /// Where the node sits in the template.
        path: String,
        /// The Reference Model type the node constrains.
        rm_type: String,
        /// The constrainer class the template uses.
        constrainer: String,
    },
}

impl From<openehr_its::xml::runtime::XmlError> for ReadError {
    fn from(source: openehr_its::xml::runtime::XmlError) -> Self {
        Self::Xml(Box::new(source))
    }
}

impl From<openehr_adl::flatten::FlattenError> for ReadError {
    fn from(source: openehr_adl::flatten::FlattenError) -> Self {
        Self::Flatten(Box::new(source))
    }
}

impl From<openehr_adl::opt::OptError> for ReadError {
    fn from(source: openehr_adl::opt::OptError) -> Self {
        Self::Opt(Box::new(source))
    }
}

impl ReadError {
    /// Builds the syntax-error variant from what the ADL 2 parser reported.
    ///
    /// An empty list cannot happen: `openehr_adl` returns `Err` only with at
    /// least one error. Should one arrive anyway, it becomes an error that
    /// says so rather than a panic.
    #[must_use]
    pub fn from_syntax(all: Vec<openehr_adl::error::SyntaxError>) -> Self {
        let first = all
            .first()
            .cloned()
            .unwrap_or_else(|| openehr_adl::error::SyntaxError {
                code: openehr_adl::error::SyntaxErrorCode::Sunk,
                message: "the parser refused the source without reporting an error".to_owned(),
                line: 1,
                column: 1,
                span: 0..0,
            });
        Self::AdlSyntax {
            first: Box::new(first),
            all,
        }
    }
}
