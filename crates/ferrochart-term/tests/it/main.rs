// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The terminology client, driven against a mock FHIR terminology server.
//!
//! One integration binary per crate (`.claude/rules/testing.md`), so every
//! topic here is a module rather than a top-level test file.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test assertions"
)]

mod cache;
mod ecl;
mod expand;
mod failure;
mod fhir_model;
mod live;
mod local;
mod lookup;
mod support;
mod validate;
