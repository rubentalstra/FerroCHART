// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The badge and the status pill: the two small labels this app draws beside
//! a thing.
//!
//! One definition each. A badge is a static label, so it is a class constant
//! a caller composes; a status pill carries a state, so it is a component
//! that draws the tone's glyph and names it.

use leptos::prelude::*;

use crate::icon::{Glyph, Size};
use crate::kit::tone::Tone;

/// A small label beside a thing: a count, a category, a template id.
///
/// The tone's own classes are appended by the caller, so one shape carries
/// every colour.
pub(crate) const BADGE: &str = "inline-flex items-center gap-1 rounded-control border \
                                px-1.5 py-0.5 text-xs font-medium";

/// A label that carries a state.
///
/// The glyph is named rather than decorative, so a reader who cannot see the
/// colour still hears which state this is.
#[component]
pub(crate) fn StatusPill(
    /// The state the pill reports.
    tone: Tone,
    /// What the state is called.
    #[prop(into)]
    label: String,
) -> impl IntoView {
    view! {
        <span class=format!("{BADGE} {}", tone.subtle())>
            <Glyph icon=tone.icon() size=Size::Chip label=tone.name() />
            {label}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::BADGE;

    #[test]
    fn the_badge_carries_no_colour_of_its_own() {
        for coloured in ["bg-", "text-ink", "border-edge"] {
            assert!(
                !BADGE.contains(coloured),
                "the tone owns the colour, so `{coloured}` cannot be in the shape"
            );
        }
        assert!(
            BADGE.contains("border"),
            "the shape owns the hairline width"
        );
    }

    #[test]
    fn the_badge_draws_no_focus_ring_of_its_own() {
        assert!(
            !BADGE.contains("focus:") && !BADGE.contains("focus-visible:"),
            "the stylesheet's base layer owns the focus indicator"
        );
    }
}
