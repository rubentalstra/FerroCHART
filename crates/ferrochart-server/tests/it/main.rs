// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The gate of `docs/architecture.md` section 9, on the bench and against a
//! real CDR.
//!
//! One integration binary per crate (`.claude/rules/testing.md`), so every
//! topic here is a module rather than a top-level test file.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test assertions"
)]

mod api;
mod filler;
mod gate;
mod live;
mod store;
mod support;
