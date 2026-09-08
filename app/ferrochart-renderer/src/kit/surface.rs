// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Cards and wells: the two ways this app raises or sinks a region.
//!
//! Depth is `raised` against `sunken` plus a hairline and one soft shadow.
//! There is no elevation scale, and a second one is never added: a builder
//! screen with a canvas and an inspector has enough going on without
//! competing depths.

/// A raised panel. The default container for anything with a boundary.
pub(crate) const CARD: &str = "rounded-card border border-edge bg-raised shadow-card";

/// A raised panel with the standard inner padding.
pub(crate) const CARD_PAD: &str = "rounded-card border border-edge bg-raised shadow-card p-4";

/// A sunken region: a read-only pane, a summary strip, an inert block.
pub(crate) const WELL: &str = "rounded-card border border-edge bg-sunken p-3";

/// The heading inside a card.
pub(crate) const CARD_TITLE: &str = "mb-3 text-sm font-semibold text-ink";

/// A monospace run: an archetype id, a node path, a template id, an AQL
/// string. Every one of them is monospace, everywhere. A clinician reading a
/// path needs to see where one segment ends, and a proportional font hides
/// that.
pub(crate) const CODE: &str = "font-mono text-xs text-ink";
