// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Terminology resolution for coded fields (`docs/architecture.md`
//! section 7).
//!
//! Most value sets resolve out of the operational template. A server call
//! happens only for what the template does not carry, and it goes to any
//! conformant HL7 FHIR R4 4.0.1 terminology server: `ValueSet/$expand` to fill
//! a picker, `CodeSystem/$lookup` for the display text of an enumerated
//! external code, and `ValueSet/$validate-code` to confirm a chosen one.
//!
//! Nothing here is specific to one server, and no FHIR resource is
//! hand-written: the model and the operation request and response contracts
//! come from `fhir-types` (`docs/architecture.md` section 7.3).
//!
//! FerroCHART does not read the SNOMED CT expression constraint language. An
//! expression is passed through to the server as the implicit value set of
//! HL7 FHIR R4 4.0.1 `snomedct.html` section 4.3.1.0.9, whose result that page
//! defines as "the result of executing the given SNOMED CT Expression
//! Constraint": executing it is a terminology server's work, and FerroCHART is
//! a client of one.

pub mod cache;
pub mod client;
pub mod error;
pub mod expand;
pub mod local;
pub mod lookup;
pub mod resolution;
pub mod resolve;
pub mod system;
pub mod target;
pub mod validate;
