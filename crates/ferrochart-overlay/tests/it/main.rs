// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The integration tests of `ferrochart-overlay`, in one binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::panic_in_result_fn,
    reason = "test assertions"
)]

mod corpus;
mod geometry;
mod replay;
mod report;
mod store;
mod support;
