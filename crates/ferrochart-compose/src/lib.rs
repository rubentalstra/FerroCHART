// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Turns what a clinician entered into a COMPOSITION, and a COMPOSITION back
//! into what a clinician entered (`docs/architecture.md` sections 5 and 12).
//!
//! # The two directions are one crate
//!
//! One form definition serves entry, review and editing, so the builder and
//! the reader are inverses of each other and are tested as a pair. Splitting
//! them would let one drift.
//!
//! # It works from the form definition alone
//!
//! This crate links [`ferrochart_form`] and the Reference Model, and nothing
//! else of this tree. It never reads an operational template: every
//! [`ferrochart_form::key::KeyStep`] already carries the Reference Model
//! attribute, the node id, the archetype id, the Reference Model type and the
//! pinned name, and every group carries its shape, so the whole Reference
//! Model tree is reconstructible from the published contract.
//!
//! That is deliberate. `docs/architecture.md` section 11 makes the form
//! definition a contract a third party can write a renderer against; keeping
//! the builder on the same contract makes it one they can commit against too.
//! A builder that needed the template would demote the definition to an
//! intermediate representation.
//!
//! # What the specification does not guarantee
//!
//! No openEHR specification guarantees that a COMPOSITION committed to a CDR
//! comes back unchanged, and none says what a CDR may normalise. The one
//! thing it sanctions a server authoring is version identity. So the property
//! this crate holds itself to is the **form-level inverse**: reading a
//! committed COMPOSITION back into the same form definition yields the same
//! field values. That survives every normalisation the Reference Model leaves
//! open, and it is the only claim the specifications support.

pub mod build;
pub mod datum;
pub mod envelope;
pub mod error;
pub mod tree;
pub mod values;
