// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A form definition and the web template spelling it came from.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::ids::{ArchetypeId, TemplateId};
use ferrochart_form::key::NodeKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ReadError, WriteError};
use crate::source::SourceSpelling;
use crate::version::SourceVersion;
use crate::write::WrittenDocument;

/// The member a web template states its own format version in.
const FORMAT_VERSION_MEMBER: &str = "version";

/// A web template, held as the form definition it maps onto and the spelling
/// it arrived in.
///
/// The web template is a compatibility target rather than a specified format,
/// so this type is FerroCHART's reading of it and not a conformance claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebTemplateDocument {
    form: FormDefinition,
    source: SourceSpelling,
    unmodelled: Vec<NodeKey>,
}

impl WebTemplateDocument {
    /// The document `json` states.
    ///
    /// # Errors
    /// [`ReadError`] when the bytes are not JSON, or when the document they
    /// hold is not one this crate can read.
    pub fn read(json: &str) -> Result<Self, ReadError> {
        let value: Value = serde_json::from_str(json).map_err(ReadError::NotJson)?;
        crate::read::document(&value)
    }

    /// The document `value` states.
    ///
    /// # Errors
    /// [`ReadError`] when the document is not one this crate can read.
    pub fn from_value(value: &Value) -> Result<Self, ReadError> {
        crate::read::document(value)
    }

    /// A document for `form`, to be written in web template format version
    /// `format_version`.
    ///
    /// The format version is the caller's to state. A web template carries one
    /// and no openEHR specification defines the sequence, so this crate has no
    /// version of its own to claim and never invents one.
    #[must_use]
    pub fn from_form(form: FormDefinition, format_version: impl Into<String>) -> Self {
        let mut source = SourceSpelling::none();
        let mut document = serde_json::Map::new();
        document.insert(
            FORMAT_VERSION_MEMBER.to_owned(),
            Value::String(format_version.into()),
        );
        source.set_document(document);
        Self {
            form,
            source,
            unmodelled: Vec::new(),
        }
    }

    /// The parts a read produced.
    pub(crate) fn from_parts(
        form: FormDefinition,
        source: SourceSpelling,
        unmodelled: Vec<NodeKey>,
    ) -> Self {
        Self {
            form,
            source,
            unmodelled,
        }
    }

    /// The form definition.
    #[must_use]
    pub fn form(&self) -> &FormDefinition {
        &self.form
    }

    /// The form definition, to change before writing the document back out.
    pub fn form_mut(&mut self) -> &mut FormDefinition {
        &mut self.form
    }

    /// The spelling the document arrived in, which is where a member the form
    /// definition does not model lives.
    #[must_use]
    pub fn source(&self) -> &SourceSpelling {
        &self.source
    }

    /// Every node whose content the form definition does not carry.
    ///
    /// A web template states inputs for classes the derivation table has no
    /// row for, the `PARTY_PROXY` family among them, and the form renders
    /// nothing for those. They are listed here so a caller sees them rather
    /// than finding them missing, and they still go back out verbatim.
    #[must_use]
    pub fn unmodelled_nodes(&self) -> &[NodeKey] {
        &self.unmodelled
    }

    /// The web template format version the document states.
    ///
    /// This is a fact about the document rather than about the template it
    /// describes; [`Self::source_version`] answers the other question.
    #[must_use]
    pub fn format_version(&self) -> Option<&str> {
        self.source
            .document()
            .get(FORMAT_VERSION_MEMBER)
            .and_then(Value::as_str)
    }

    /// The `semVer` the document states, recorded and never consulted.
    ///
    /// It is null for every document built from an OPT 1.4 template, so
    /// nothing here branches on it. [`crate::version`] says what is consulted
    /// instead.
    #[must_use]
    pub fn stated_sem_ver(&self) -> Option<&str> {
        self.source.document().get("semVer").and_then(Value::as_str)
    }

    /// What the document states about the operational template it describes.
    ///
    /// Read off `templateId` and the root node's `nodeId`, never off `semVer`.
    /// [`crate::version`] carries the reason.
    #[must_use]
    pub fn source_version(&self) -> SourceVersion {
        SourceVersion {
            template_id: TemplateId::new(self.form.template_id.as_str()),
            root_archetype_id: self
                .form
                .root
                .archetype_id
                .as_ref()
                .map(|id| ArchetypeId::new(id.as_str())),
        }
    }

    /// The document, written back out as a web template.
    ///
    /// # What survives
    ///
    /// Every member the form definition does not model survives verbatim: the
    /// node's `id`, `aqlPath`, `inContext`, `annotations`, `termBindings`,
    /// `cardinalities` and `dependsOn`, the document's `semVer`, `version` and
    /// `otherDetails`, and any member a tool this crate has never seen wrote.
    /// A node the form definition still states as it was read goes back out
    /// untouched, so members nested inside `inputs` survive too, a per-option
    /// terminology binding among them.
    ///
    /// # What cannot
    ///
    /// A form definition holds facts a web template has no member for: the
    /// null-flavour affordance beside a field, the reference bands shown
    /// beside a value, the shape a group's Reference Model class gives it,
    /// whether a repeat is ordered or unique, whether the archetype marks a
    /// node deprecated, and the state machine of a `DV_STATE`. Those are lost
    /// uniformly. The ones that belong to a single item are listed in
    /// [`WrittenDocument::not_carried`] rather than dropped in silence.
    ///
    /// # Errors
    /// [`WriteError`] when a field states something the format has a member
    /// for but no spelling of.
    pub fn write(&self) -> Result<WrittenDocument, WriteError> {
        crate::write::document(self)
    }
}
