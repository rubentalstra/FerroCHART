// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The ADL 1.4 operational template reader.
//!
//! `openehr_its::opt14::from_xml` parses the `.opt`; this module walks what it
//! produced and fills the internal constraint model, so everything downstream
//! is generation-blind.
//!
//! # The oracle for this layer is the XSD, not AOM 1.4 prose
//!
//! openEHR ITS-XML 2.0.0 `components/AM/Release-1.4/` (`Archetype.xsd`,
//! `OpenehrProfile.xsd`, `Template.xsd`, `Resource.xsd`) is what a real `.opt`
//! validates against and what a CDR accepts on
//! `POST /v1/definition/template/adl1.4`. openEHR AM Release-2.3.0
//! `AOM1.4.html` names a different class set for the same layer. Every
//! divergence this reader depends on is recorded here.
//!
//! NOTE: `Archetype.xsd` derives `ARCHETYPE_SLOT`, `CONSTRAINT_REF` and
//! `ARCHETYPE_INTERNAL_REF` straight from `C_OBJECT` and declares no
//! `C_REFERENCE_OBJECT`, while `AOM1.4.html` section 4.2 puts all three under
//! `C_REFERENCE_OBJECT` and gives `ARCHETYPE_SLOT` the invariant
//! `any_allowed xor (includes /= Void or excludes /= Void)`; the reader tests
//! a slot's occurrences rather than an `any_allowed` the XSD has no field for.
//!
//! NOTE: `OpenehrProfile.xsd` declares `C_DV_STATE`, `STATE_MACHINE`,
//! `NON_TERMINAL_STATE`, `TERMINAL_STATE` and `TRANSITION`, none of which
//! `AOM1.4.html` defines in prose; the reader reads the XSD shape, which is
//! the only definition there is.
//!
//! NOTE: `Template.xsd` declares `C_CODE_REFERENCE` (a `C_CODE_PHRASE` with a
//! `referenceSetUri`) and `T_COMPLEX_OBJECT` (a `C_COMPLEX_OBJECT` with a
//! `default_value`), and `AOM1.4.html` defines neither; both are read as the
//! XSD declares them.
//!
//! NOTE: `Archetype.xsd` declares `C_OBJECT.occurrences` without
//! `minOccurs="0"`, so an ADL 1.4 node always states its occurrences, unlike
//! the optional `C_OBJECT.occurrences` of openEHR AM Release-2.3.0
//! `AOM2.html` section 4.5.2.

mod defaults;
mod name;
mod path;
mod payload;
mod slot;
mod terminology;
mod walk;

use openehr_its::opt14::types as opt;

use crate::error::ReadError;
use crate::model::node::ConstraintTemplate;

/// Reads an ADL 1.4 operational template from its canonical XML.
///
/// # Errors
/// [`ReadError::Xml`] when the document does not parse, and anything
/// [`read`] refuses.
pub fn from_xml(xml: &str) -> Result<ConstraintTemplate, ReadError> {
    let template = openehr_its::opt14::from_xml(xml)?;
    read(&template)
}

/// Reads a parsed ADL 1.4 operational template into the internal constraint
/// model.
///
/// A node whose `occurrences matches {0}` is absent from the result: openEHR
/// AM Release-2.3.0 `OPT2.html` section 3.3 removes such a node when it
/// flattens, so the template has taken it away.
///
/// # Errors
/// [`ReadError::RequiredSlotUnfilled`] when the template requires content at a
/// slot it never filled; [`ReadError::UnresolvedInternalReference`] when a
/// `use_node` reference names a node the template does not define;
/// [`ReadError::UnresolvedDefaultPath`] when a default value names a node the
/// definition does not carry; [`ReadError::NegativeMultiplicity`] when a count
/// interval carries a bound that is not a count;
/// [`ReadError::UninterpretableConstraint`] when a node constrains a value in
/// a way this reader cannot place; [`ReadError::TooDeep`] past
/// [`crate::error::MAX_DEPTH`].
pub fn read(template: &opt::OperationalTemplate) -> Result<ConstraintTemplate, ReadError> {
    walk::read(template)
}
