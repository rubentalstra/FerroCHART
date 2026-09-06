// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The form renderer (`docs/architecture.md` section 10).
//!
//! It reads the form definition the server publishes and links no engine
//! crate. That boundary gets its committed check with the renderer itself.
