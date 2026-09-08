// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The screens, one module per screen.
//!
//! A screen reads through [`crate::api`] and never opens a request of its
//! own. It renders a failed read inline through [`inline`], because
//! `docs/architecture.md` section 10.1 reserves the toast for a mutation.

pub(crate) mod form;
pub(crate) mod inline;
pub(crate) mod templates;
