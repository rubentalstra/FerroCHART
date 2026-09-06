// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The layout overlay: storage, key normalization, replay, and the
//! differential report a template revision produces
//! (`docs/architecture.md` section 6).
//!
//! The key is a step chain carrying the RM attribute, the node id, the
//! archetype id, the RM type and the pinned name, with a sibling ordinal only
//! where all five tie.
