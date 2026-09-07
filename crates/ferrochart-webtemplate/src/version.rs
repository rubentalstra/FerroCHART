// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Telling one revision of a template apart from another, without `semVer`.
//!
//! # Why `semVer` is never consulted
//!
//! A web template states `semVer` beside `templateId`, and it reads like the
//! template's version. It is not usable as one. `semVer` is null for a
//! document built from an OPT 1.4 template, and a test over the whole
//! vendored CKM pack pins that: not one document of the pack states the
//! member. A reader that keyed on `semVer` would call every ADL 1.4 template
//! versionless, which is most of the templates a CDR holds.
//!
//! It is also not the format version. `version` is the web template format
//! version string, a fact about the document rather than about the template,
//! so it answers a different question again.
//!
//! # What is consulted instead
//!
//! [`SourceVersion`] is what the document does state about the artefact it
//! describes: the template identifier, and the root archetype identifier off
//! the root node's `nodeId`. An openEHR archetype identifier carries a version
//! part (openEHR BASE Release-1.2.0 `base_types.html` section 5,
//! `ARCHETYPE_ID`), so the pair moves when the root archetype is revised and
//! when the template is renamed.
//!
//! It is not a template version number, and this crate does not pretend it is:
//! a web template built from an OPT 1.4 template states none, and inventing
//! one would be a claim no source supports. `semVer` is kept verbatim and
//! readable through [`crate::document::WebTemplateDocument::stated_sem_ver`]
//! for a caller who wants what the document said, and nothing here branches on
//! it. No specification governs this: our own design.

use ferrochart_form::ids::{ArchetypeId, TemplateId};

/// What a web template states about the operational template it describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceVersion {
    /// The identifier the document states in `templateId`.
    pub template_id: TemplateId,
    /// The root archetype identifier, off the root node's `nodeId`.
    ///
    /// `None` where the root states no `nodeId`, which a document a hand-built
    /// form definition was written into may well not.
    pub root_archetype_id: Option<ArchetypeId>,
}
