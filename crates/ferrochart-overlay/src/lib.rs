// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The layout overlay: storage, key normalization, replay, and the
//! differential report a template revision produces
//! (`docs/architecture.md` section 6).
//!
//! # What the overlay is for
//!
//! A form definition is a projection of an operational template, so the
//! compiler decides everything the specifications govern and nothing else.
//! What is left is the half a person does by hand: field order, grouping,
//! labels, help text, defaults, conditional visibility, widget choice, and
//! where an item sits on its section's column grid. openEHR publishes no form
//! artefact and defines the semantics of no place a template could carry that
//! work, so all of it is FerroCHART's own design.
//!
//! # What this crate owns, and what it does not
//!
//! It owns the machinery: the stored document, the authoring session that keys
//! an entry against one definition, the replay, and the report a template
//! revision produces. All of it reads and writes files or compares two
//! definitions.
//!
//! It owns none of the layout types. [`ferrochart_form::layout`] holds
//! [`ferrochart_form::layout::Layout`], [`ferrochart_form::layout::Section`]
//! and the column grid of `docs/architecture.md` section 6.3, because a
//! renderer applies a layout and has no business with anything in the
//! paragraph above. That keeps a renderer on `ferrochart-form` alone, which is
//! what makes the form definition a contract a third party implements rather
//! than a crate graph they adopt (`docs/architecture.md` section 11). Nothing
//! here is re-exported: a caller that needs a layout type names the crate that
//! defines it.
//!
//! The overlay is stored apart from the definition and is never merged into
//! it. That is what makes a recompile survivable: the template is revised, the
//! definition is derived again, and the overlay is replayed against the new
//! definition rather than discarded.
//!
//! # The key
//!
//! An entry is keyed by [`ferrochart_form::key::NodeKey`], whose every step
//! carries the Reference Model attribute, the node id, the archetype id, the
//! Reference Model type and the pinned name, with a sibling ordinal only where
//! all five tie. Each part earns its place in a measurement over real
//! templates, and a key that drops one loses real templates
//! (`docs/architecture.md` section 6.2).
//!
//! [`store::Author`] normalizes a key against the definition rather than
//! trusting what it is handed: an entry that decorates no node of the form is
//! refused, and an entry the definition tells from its siblings by nothing but
//! its position is reported as such and anchored to the siblings it was
//! measured against.
//!
//! # The replay
//!
//! [`replay::replay`] classifies every entry against a recompiled definition
//! and reports it. Nothing is discarded, nothing is rebound, and a move is a
//! suggestion a person accepts through [`store::Author::accept_move`]. The
//! report is the product: no implementation surveyed for this project says
//! what a recompile did to hand-authored layout.

pub mod advice;
pub mod entry;
pub mod error;
mod index;
pub mod replay;
pub mod store;
