// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The form definition and the layout types, with their serialisations.
//!
//! This crate is the published contract a third party writes a renderer
//! against, so it performs no I/O and depends on nothing else in the tree.
//! `scripts/checks/crate-closure.sh` walks the resolved dependency graph and
//! fails when a first-party crate appears in it. The format is described in
//! `docs/architecture.md` sections 4 and 6.
//!
//! A form definition is a tree of groups rooted at
//! [`definition::FormDefinition::root`]. A group holds items, an item is
//! either a nested group or a field, and a field carries one
//! [`field::FieldKind`], which is what a renderer chooses its control from.
//! Content the operational template left undetermined is recorded on the group
//! that would have held it and is never rendered.
//!
//! # What this crate owns, and what it does not
//!
//! It owns the two things a renderer reads: the definition above, and the
//! layout types a person authored against it ([`mod@layout`]), the column grid
//! of `docs/architecture.md` section 6.3 among them. A renderer applies a
//! layout, so the types are here beside the definition they decorate.
//!
//! It owns none of the machinery around them. Overlay storage, the key
//! normalization, the authoring session, the replay and the report read and
//! write files and compare two definitions, none of which a renderer does, and
//! all of it lives in `ferrochart-overlay`. That is what keeps a renderer on
//! this one dependency from the tree, and it is what makes the form definition
//! a contract a third party implements rather than a crate graph they adopt
//! (`docs/architecture.md` section 11).
//!
//! # The serialisation is byte-deterministic
//!
//! Every map in the format is a [`std::collections::BTreeMap`] and every list
//! keeps the order the operational template stated, so serializing the same
//! form twice, or recompiling an unchanged template, produces an identical
//! document. That is what makes a diff between two compilations meaningful
//! (`.claude/rules/reliability.md`).

pub mod definition;
pub mod field;
pub mod group;
pub mod ids;
pub mod key;
pub mod layout;
pub mod occurrences;
pub mod range;
pub mod text;
pub mod value;
