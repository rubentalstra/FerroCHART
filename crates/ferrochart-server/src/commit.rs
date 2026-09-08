// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Building a COMPOSITION, judging it, and only then committing it.
//!
//! # Why the gate lives here
//!
//! `docs/architecture.md` section 9 requires a composition to be validated
//! against its operational template before any request is made. Nothing below
//! this crate can enforce that: `ferrochart-cdr` links `ferrochart-form` and
//! nothing else of the tree, and `scripts/checks/crate-closure.sh` fails the
//! build if it ever links more, so the client cannot hold a validator.
//! `ferrochart-validate` cannot hold a client for the mirror reason, and it
//! should not: a validator that can post is a validator that can be talked
//! into posting.
//!
//! So the gate is a composition of the three, and section 11 already names
//! the crate that composes them: the server surface, which "serves
//! definitions, validates, builds and commits compositions".
//! [`Commit::create`] and [`Commit::update`] are the only paths in this
//! workspace from a clinician's entries to a CDR write, and neither one can
//! reach the client without passing through the validator first.

use ferrochart_cdr::client::CdrClient;
use ferrochart_cdr::composition::Written;
use ferrochart_cdr::error::CdrError;
use ferrochart_cdr::header::Prefer;
use ferrochart_cdr::ids::{EhrId, VersionUid, VersionedObjectUid};
use ferrochart_compose::build;
use ferrochart_compose::envelope::Envelope;
use ferrochart_compose::error::BuildError;
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::validation::ValidationReport;
use ferrochart_form::values::FormValues;
use ferrochart_validate::error::ValidateError;
use ferrochart_validate::template::TemplateValidator;
use openehr_rm::v1_2::composition::composition::Composition;

/// Everything one commit needs, gathered once per form.
///
/// The validator flattens its operational template when it is built, so a
/// server serving one form judges every submission against a value it already
/// holds.
#[derive(Debug)]
pub struct Commit<'a> {
    /// The form the entries were made against.
    pub definition: &'a FormDefinition,
    /// The gate for the operational template that form came from.
    pub validator: &'a TemplateValidator,
    /// The Reference Model values a form never shows.
    pub envelope: &'a Envelope,
}

/// Why a commit did not happen.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommitError {
    /// The entries do not build a COMPOSITION at all.
    #[error("the entered values do not build a COMPOSITION")]
    Build {
        /// What the builder refused.
        #[source]
        source: BuildError,
    },

    /// The composition could not be judged.
    ///
    /// Distinct from [`CommitError::Refused`]: nothing was decided about the
    /// document, so nothing may be inferred about it either.
    #[error("the COMPOSITION could not be judged against its template")]
    Judge {
        /// Why the validator could not run.
        #[source]
        source: ValidateError,
    },

    /// The composition does not conform to its operational template, so no
    /// request was made.
    ///
    /// The report is keyed by [`ferrochart_form::key::NodeKey`], so a renderer
    /// holding the same form definition puts each failure on its field.
    #[error("the COMPOSITION does not conform to its template ({} failures)", report.failures.len())]
    Refused {
        /// Every failure, keyed onto the form where its path resolved.
        report: Box<ValidationReport>,
    },

    /// The CDR refused a composition FerroCHART built and validated.
    ///
    /// That is a FerroCHART defect (`CLAUDE.md`), so the CDR's own error body
    /// is read into `diagnosis` as material for it. The reading is
    /// best-effort and vendor-specific
    /// (`ferrochart_validate::vendor`), and it is empty where the CDR sent
    /// nothing to read.
    #[error("the CDR refused a COMPOSITION FerroCHART validated")]
    Rejected {
        /// What the CDR said.
        #[source]
        source: Box<CdrError>,
        /// The CDR's error body, read onto the form where it could be.
        diagnosis: Box<ValidationReport>,
    },

    /// The request itself failed, before or after the CDR judged anything.
    #[error("the commit did not reach the CDR")]
    Wire {
        /// What went wrong.
        #[source]
        source: Box<CdrError>,
    },
}

impl Commit<'_> {
    /// Builds the COMPOSITION `values` describe and judges it.
    ///
    /// The public half of the gate, for a caller that wants the document
    /// without committing it: a preview, a download, or a second commit of
    /// the same entries.
    ///
    /// # Errors
    /// [`CommitError::Build`] when the entries build no document,
    /// [`CommitError::Judge`] when it cannot be judged, and
    /// [`CommitError::Refused`] when it does not conform.
    pub fn validated(&self, values: &FormValues) -> Result<Composition, CommitError> {
        let composition = build::composition(self.definition, values, self.envelope)
            .map_err(|source| CommitError::Build { source })?;
        let report = self
            .validator
            .validate(self.definition, &composition)
            .map_err(|source| CommitError::Judge { source })?;
        if report.is_valid() {
            return Ok(composition);
        }
        Err(CommitError::Refused {
            report: Box::new(report),
        })
    }

    /// Commits the entries into `ehr` as a new COMPOSITION.
    ///
    /// `Prefer: return=identifier`, which yields the version uid the next
    /// update sends in `If-Match` and nothing more
    /// (`docs/architecture.md` section 8).
    ///
    /// # Errors
    /// Anything [`Commit::validated`] refuses, [`CommitError::Rejected`] when
    /// the CDR refuses a document that passed the gate, and
    /// [`CommitError::Wire`] for any other failure of the request.
    pub async fn create(
        &self,
        client: &CdrClient,
        ehr: &EhrId,
        values: &FormValues,
    ) -> Result<Written, CommitError> {
        let composition = self.validated(values)?;
        match client
            .create_composition(ehr, &composition, Prefer::Identifier)
            .await
        {
            Ok(written) => Ok(written),
            Err(failure) => Err(self.rejected(&composition, failure)),
        }
    }

    /// Commits the entries into `ehr` as a new version of `id`, replacing
    /// `preceding`.
    ///
    /// # Errors
    /// Anything [`Commit::validated`] refuses, [`CommitError::Rejected`] when
    /// the CDR refuses a document that passed the gate, and
    /// [`CommitError::Wire`] for any other failure of the request, a
    /// concurrent edit among them.
    pub async fn update(
        &self,
        client: &CdrClient,
        ehr: &EhrId,
        id: &VersionedObjectUid,
        preceding: &VersionUid,
        values: &FormValues,
    ) -> Result<Written, CommitError> {
        let composition = self.validated(values)?;
        match client
            .update_composition(ehr, id, preceding, &composition, Prefer::Identifier)
            .await
        {
            Ok(written) => Ok(written),
            Err(failure) => Err(self.rejected(&composition, failure)),
        }
    }

    /// A CDR failure, with its body read onto the form where the failure is a
    /// refusal of the document.
    ///
    /// Only 400 and 422 are refusals of what FerroCHART sent. Every other
    /// status is about the request, the EHR, the version or the server, and
    /// reading a field-level diagnosis out of one would invent a finding.
    fn rejected(&self, composition: &Composition, failure: CdrError) -> CommitError {
        let body = match failure {
            CdrError::BadRequest { ref body } | CdrError::Unprocessable { ref body } => body,
            other => {
                return CommitError::Wire {
                    source: Box::new(other),
                };
            }
        };
        let diagnosis = self
            .validator
            .read_refusal(self.definition, composition, body);
        CommitError::Rejected {
            source: Box::new(failure),
            diagnosis: Box::new(diagnosis),
        }
    }
}
