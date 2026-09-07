// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The composition builder and the read-back, tested as inverses.
//!
//! One integration binary per crate (`.claude/rules/testing.md`), so every
//! topic here is a module rather than a top-level test file.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test assertions"
)]

mod corpus;
mod datum;
mod filler;
