// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Why a composition could not be judged at all.
//!
//! A failure here is distinct from a composition that was judged and found
//! wanting, which is a
//! [`ferrochart_form::validation::ValidationReport`]. Nothing in this crate
//! turns one into the other: a validator that could not run reports that it
//! could not run, rather than an empty report a caller would read as a pass.

use thiserror::Error;

/// Why the validator could not judge a composition.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValidateError {
    /// The operational template XML did not parse.
    #[error("the operational template does not parse")]
    Template {
        /// Why it did not parse.
        #[source]
        source: openehr_its::xml::runtime::XmlError,
    },

    /// The operational template parsed and no web template could be built
    /// from it.
    ///
    /// The web template is the flattened form the archetype-conformance pass
    /// walks (`openehr_its::flat::webtemplate`). It is a compatibility target
    /// rather than a specification: openEHR ITS-REST Release-1.1.0
    /// `simplified_formats.html` section 2.2 puts "Web Template itself as a
    /// resource" under what the specification does not cover.
    #[error("no web template can be built from the operational template")]
    WebTemplate {
        /// What the builder said.
        detail: String,
    },

    /// The COMPOSITION did not serialize to canonical JSON.
    ///
    /// The instance passes read the wire value, so the document has to reach
    /// them as one.
    #[error("the COMPOSITION does not serialize to canonical JSON")]
    Canonical {
        /// Why it did not serialize.
        #[source]
        source: serde_json::Error,
    },

    /// The composition carries no node the template's root describes.
    ///
    /// A template rooted below COMPOSITION describes a fragment, and the
    /// document that carries it puts that fragment somewhere under `content`.
    /// A document that carries it nowhere cannot be judged against the
    /// template at all, and saying so is the honest answer.
    #[error("the composition carries no {rm_type} node for template root `{archetype_id}`")]
    TemplateRootAbsent {
        /// The Reference Model class the template is rooted at.
        rm_type: String,
        /// The archetype the template is rooted at.
        archetype_id: String,
    },
}
