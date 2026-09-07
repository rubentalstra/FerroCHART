// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The form definition and the layout overlay, with their serialisations.
//!
//! This crate is the published contract a third party writes a renderer
//! against, so it performs no I/O and depends on nothing else in the tree.
//! The format is described in `docs/architecture.md` sections 4 and 6.
//!
//! A form definition is a tree of groups rooted at
//! [`definition::FormDefinition::root`]. A group holds items, an item is
//! either a nested group or a field, and a field carries one
//! [`field::FieldKind`], which is what a renderer chooses its control from.
//! Content the operational template left undetermined is recorded on the group
//! that would have held it and is never rendered.
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
pub mod occurrences;
pub mod range;
pub mod text;
pub mod value;
