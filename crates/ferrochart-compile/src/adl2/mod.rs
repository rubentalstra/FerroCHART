// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The ADL 2 reader.
//!
//! FerroCHART does not read a published OPT 2 file, because there is nothing
//! to read: openEHR AM Release-2.3.0 `OPT2.html` section 5.2 "Concrete
//! Formats" and its ADL, JSON/YAML and XML subsections are empty headings. The
//! ADL 2 path therefore starts from source archetypes and a source template
//! and produces the operational form itself, with `openehr_adl`'s
//! `flatten::flat_form` and `opt::create_opt`.
//!
//! The result fills the same [`crate::model`] the ADL 1.4 reader fills. Where
//! the two generations spell one clinical constraint differently, this reader
//! normalizes: a `DV_QUANTITY` complex object whose `magnitude` and `units`
//! are tied by a `C_PRIMITIVE_TUPLE` becomes the same
//! [`ConstraintPayload::Quantity`] the ADL 1.4 `C_DV_QUANTITY` becomes, and a
//! `DV_ORDINAL` or `DV_SCALE` tuple over `symbol` and `value` becomes the same
//! [`ConstraintPayload::Ordinal`]. openEHR AM Release-2.3.0 `AOM2.html`
//! section 4.3 §Tuple Constraints says why: the tuple constraint type replaces
//! all domain-specific constraint types defined in ADL/AOM 1.4, including
//! `C_DV_QUANTITY` and `C_DV_ORDINAL`.
//!
//! [`ConstraintPayload::Quantity`]: crate::model::payload::ConstraintPayload::Quantity
//! [`ConstraintPayload::Ordinal`]: crate::model::payload::ConstraintPayload::Ordinal

mod payload;
mod terminology;
mod walk;

use openehr_adl::artefact::ArchetypeRepository;
use openehr_adl::parse::Dialect;
use openehr_am::v2_4::aom2::archetype::archetype::Archetype;
use openehr_am::v2_4::aom2::archetype::operational_template::OperationalTemplate;

use crate::error::ReadError;
use crate::model::node::ConstraintTemplate;

/// Parses one ADL 2 artefact from its source text.
///
/// # Errors
/// [`ReadError::AdlSyntax`] carrying every error the parser reported.
pub fn parse(source: &str) -> Result<Archetype, ReadError> {
    openehr_adl::assemble::parse_artefact(source, Dialect::Adl2).map_err(ReadError::from_syntax)
}

/// Reads an ADL 2 template from its source, with `constituents` supplying every
/// archetype it specialises or fills a slot with.
///
/// The constituents are parsed first and registered in a repository, then the
/// template's lineage is flattened and its operational form generated, then
/// that form is walked into the internal constraint model.
///
/// # Errors
/// [`ReadError::AdlSyntax`] when a source does not parse, [`ReadError::Opt`]
/// when a reference does not resolve or the fillers form a cycle,
/// [`ReadError::Flatten`] when a specialisation lineage cannot be flattened,
/// and anything [`read`] refuses.
pub fn from_source(template: &str, constituents: &[&str]) -> Result<ConstraintTemplate, ReadError> {
    let mut repository = ArchetypeRepository::new();
    for source in constituents {
        repository.insert(parse(source)?);
    }
    let root = parse(template)?;
    let operational = openehr_adl::opt::create_opt(&root, &repository)?;
    read(&operational)
}

/// Reads a generated ADL 2 operational template into the internal constraint
/// model.
///
/// A node whose `occurrences matches {0}` is absent from the result, per
/// openEHR AM Release-2.3.0 `OPT2.html` section 3.3.
///
/// # Errors
/// [`ReadError::RequiredSlotUnfilled`] when the template requires content at a
/// slot it never filled; [`ReadError::UnresolvedInternalReference`] when a
/// `use_node` proxy survived the flattener and its target is not there;
/// [`ReadError::NegativeMultiplicity`] when a count interval carries a bound
/// that is not a count; [`ReadError::UninterpretableConstraint`] when a node
/// constrains a value in a way this reader cannot place; [`ReadError::TooDeep`]
/// past [`crate::error::MAX_DEPTH`].
pub fn read(template: &OperationalTemplate) -> Result<ConstraintTemplate, ReadError> {
    walk::read(template)
}
