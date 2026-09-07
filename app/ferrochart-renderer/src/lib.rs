// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The form renderer (`docs/architecture.md` section 10).
//!
//! It reads the form definition the server publishes and links no engine
//! crate. `scripts/checks/crate-closure.sh` enforces that from the resolved
//! dependency graph, so the boundary is checked rather than intended.
