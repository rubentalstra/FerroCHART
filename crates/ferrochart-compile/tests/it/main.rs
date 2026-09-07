// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The integration tests of `ferrochart-compile`, in one binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::panic_in_result_fn,
    reason = "test assertions"
)]

mod adl14;
mod adl2;
mod adl2_corpus;
mod corpus;
mod derive;
mod matched_pair;
mod reference_ranges;
mod snapshot;
mod support;
mod terminology;
