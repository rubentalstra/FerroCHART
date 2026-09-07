// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The gate: a composition judged against its operational template.
//!
//! All citations are openEHR RM Release-1.1.0 unless another document is
//! named.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::validation::ValidationReport;
use openehr_its::flat::webtemplate::model::WebTemplate;
use openehr_its::opt14::types::OperationalTemplate;
use openehr_rm::v1_2::composition::composition::Composition;
use serde_json::Value;

use crate::error::ValidateError;
use crate::index::KeyIndex;
use crate::path;
use crate::resolve;

/// One operational template, ready to judge compositions against.
///
/// Building it parses the template and flattens it once, so a server that
/// serves one form validates every submission against a value it already
/// holds.
#[derive(Debug)]
pub struct TemplateValidator {
    web_template: WebTemplate,
}

impl TemplateValidator {
    /// Reads an ADL 1.4 operational template from its canonical XML.
    ///
    /// openEHR ITS-REST Release-1.1.0 `definition.html` serves exactly this
    /// document from `GET /v1/definition/template/adl1.4/{template_id}` under
    /// `Accept: application/xml`, so a validator is buildable from a template
    /// a CDR holds as well as from a file on disk.
    ///
    /// # Errors
    /// [`ValidateError::Template`] when the XML does not parse, and
    /// [`ValidateError::WebTemplate`] when no web template can be built from
    /// it.
    pub fn from_opt14_xml(xml: &str) -> Result<Self, ValidateError> {
        let template = openehr_its::opt14::from_xml(xml)
            .map_err(|source| ValidateError::Template { source })?;
        Self::from_opt14(&template)
    }

    /// Flattens a parsed ADL 1.4 operational template.
    ///
    /// # Errors
    /// [`ValidateError::WebTemplate`] when no web template can be built from
    /// it.
    pub fn from_opt14(template: &OperationalTemplate) -> Result<Self, ValidateError> {
        let web_template = openehr_its::flat::webtemplate::builder::build_web_template(template)
            .map_err(|error| ValidateError::WebTemplate {
                detail: error.to_string(),
            })?;
        Ok(Self { web_template })
    }

    /// The identifier the operational template states for itself.
    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.web_template.template_id
    }

    /// The Reference Model class the template is rooted at.
    #[must_use]
    pub fn root_rm_type(&self) -> &str {
        &self.web_template.tree.rm_type
    }

    /// Judges `composition` against the template.
    ///
    /// An empty report is the only outcome that permits a write. A non-empty
    /// one is not an error: the composition was judged and found wanting, and
    /// every failure it carries is keyed onto `definition` where its path
    /// resolves.
    ///
    /// # Errors
    /// [`ValidateError::Canonical`] when the composition does not serialize,
    /// and [`ValidateError::TemplateRootAbsent`] when the document carries no
    /// node the template's root describes.
    pub fn validate(
        &self,
        definition: &FormDefinition,
        composition: &Composition,
    ) -> Result<ValidationReport, ValidateError> {
        let instance = serde_json::to_value(composition)
            .map_err(|source| ValidateError::Canonical { source })?;
        self.validate_json(definition, &instance)
    }

    /// [`TemplateValidator::validate`] over a composition already in canonical
    /// JSON.
    ///
    /// # Errors
    /// [`ValidateError::TemplateRootAbsent`] when the document carries no node
    /// the template's root describes.
    pub fn validate_json(
        &self,
        definition: &FormDefinition,
        instance: &Value,
    ) -> Result<ValidationReport, ValidateError> {
        let (subject, prefix) = self.subject_of(instance)?;
        let index = KeyIndex::of(definition, &prefix);

        // Passes 1 and 2 run over the whole document, because a Reference
        // Model invariant and an openEHR terminology code are properties of
        // the instance and hold on the envelope the template never describes.
        // Their paths are already absolute, so they take no prefix.
        let instance_passes = openehr_its::rm_instance::validate_rm_and_terminology(instance);

        // Pass 3 is the template's own, and it runs over the node the
        // template describes. Its paths are relative to that node, so they
        // carry the prefix that reaches it.
        let template_pass = openehr_its::flat::validation::validate_archetype_conformance(
            subject,
            &self.web_template,
        );

        let failures = instance_passes
            .iter()
            .map(|message| resolve::failure(message, &index, instance, ""))
            .chain(
                template_pass
                    .iter()
                    .map(|message| resolve::failure(message, &index, instance, &prefix)),
            )
            .collect();
        Ok(ValidationReport { failures })
    }

    /// Reads a CDR's own refusal of `composition` into the same report shape.
    ///
    /// A CDR refusing a COMPOSITION FerroCHART built and validated is a
    /// FerroCHART defect (`CLAUDE.md`), so this is diagnostic material for
    /// that defect rather than a second opinion. The reading is best-effort
    /// and vendor-specific, and every failure it produces is marked
    /// [`ferrochart_form::validation::FailureSource::Cdr`].
    ///
    /// This never fails. The CDR's own words are the point, and a document
    /// this reader cannot walk costs the keying rather than the words: every
    /// entry still reaches the report, unplaced.
    #[must_use]
    pub fn read_refusal(
        &self,
        definition: &FormDefinition,
        composition: &Composition,
        body: &str,
    ) -> ValidationReport {
        // NOTE: the document serialized once already, for the gate that let
        // it reach a CDR at all, so neither fallback here is reachable from
        // `ferrochart_server::commit`.
        let instance = serde_json::to_value(composition).unwrap_or(Value::Null);
        let prefix = self
            .subject_of(&instance)
            .map_or_else(|_| String::new(), |(_, prefix)| prefix);
        let index = KeyIndex::of(definition, &prefix);
        ValidationReport {
            failures: crate::vendor::failures(body, &index, &instance),
        }
    }

    /// The node the template describes, and the path that reaches it.
    ///
    /// A template rooted at COMPOSITION describes the whole document. One
    /// rooted below it describes a fragment, which
    /// `ferrochart_compose::build::composition` puts under `content`, so the
    /// conformance pass runs over that fragment rather than over the envelope
    /// the template says nothing about. 113 of the 123 committed templates
    /// root below COMPOSITION, so this is the ordinary case rather than the
    /// exception.
    fn subject_of<'i>(&self, instance: &'i Value) -> Result<(&'i Value, String), ValidateError> {
        if self.root_rm_type() == "COMPOSITION" {
            return Ok((instance, String::new()));
        }
        let wanted = self
            .web_template
            .tree
            .node_id
            .as_deref()
            .unwrap_or_default();
        let found = instance
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .find(|(_, item)| {
                item.get("archetype_node_id").and_then(Value::as_str) == Some(wanted)
            });
        let Some((at, subject)) = found else {
            return Err(ValidateError::TemplateRootAbsent {
                rm_type: self.root_rm_type().to_owned(),
                archetype_id: wanted.to_owned(),
            });
        };
        Ok((
            subject,
            path::against_instance(&format!("/content[{at}]"), instance),
        ))
    }
}
