// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The web template compatibility surface: reading one into a form
//! definition, and writing a form definition back out as one.
//!
//! # The web template is a compatibility target, not a specification
//!
//! No openEHR specification defines the web template document. openEHR
//! ITS-REST Release-1.1.0 `simplified_formats.html` section 2.2 lists "Web
//! Template itself as a resource" under what that specification does not
//! cover, and section 4.1 adds that specification of web template metadata is
//! separate from the data serialisation format it describes. ITS-REST
//! registers the media type `application/openehr.wt+json` and an endpoint that
//! delivers one (`definition.html`), and stops there. So the format here is
//! established by observation of the published implementations, the openEHR
//! Reference Model is the authority wherever the two disagree, and a
//! behaviour that exists only to match an implementation carries a note
//! saying so.
//!
//! The FLAT and structured formats are a different case: `simplified_formats.html`
//! is STABLE and specifies both, so nothing in this crate applies to them.
//!
//! # This sits beside the compiler, never under it
//!
//! `ferrochart-compile` derives a form definition from the internal
//! constraint model, which is what lets one derivation serve both ADL
//! generations (`docs/architecture.md` sections 3 and 4; no specification
//! governs this: our own design). This crate never takes part in that path. It
//! reads a web template another tool produced, and writes one another tool can
//! consume, and it links `ferrochart-form` and nothing else of this tree.
//!
//! # What a round trip preserves
//!
//! A form definition is not a web template, so reading one into a form
//! definition drops every member the definition has no place for. Those
//! members are kept verbatim in a [`source::SourceSpelling`] beside the
//! definition, and writing puts them back, so a third party's annotations
//! survive passing through this tool. [`document::WebTemplateDocument`] holds
//! the pair. What survives, and what cannot, is spelled out on
//! [`document::WebTemplateDocument::write`].

mod aql;
pub mod document;
pub mod error;
mod json;
mod kind;
pub mod member;
pub mod read;
pub mod source;
pub mod version;
pub mod write;
