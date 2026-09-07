// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Operational template to form definition.
//!
//! Owns the internal constraint model that both template generations
//! normalize into, so the field derivation is written once.
//!
//! FerroCHART reads both operational template generations. The ADL 1.4 path
//! reads a `.opt` XML document ([`adl14`]); the ADL 2 path reads source
//! archetypes and a source template and produces the operational form itself
//! ([`adl2`]), because openEHR AM Release-2.3.0 `OPT2.html` section 5.2
//! publishes no concrete serialisation to read. Both fill [`model`], and
//! nothing above that point can tell which reader produced a node.
//!
//! [`mod@derive`] is what sits above that point: the field derivation table,
//! written once, from the internal constraint model to the published form
//! definition of `ferrochart-form`.

pub mod adl14;
pub mod adl2;
pub mod derive;
pub mod error;
pub mod model;
