// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The openEHR ITS-REST client (`docs/architecture.md` section 8).
//!
//! It speaks to any conformant CDR. Nothing here is specific to one
//! implementation.

pub mod client;
pub mod composition;
pub mod ehr;
pub mod error;
pub mod header;
pub mod ids;
pub mod template;
