// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The one way this app draws a glyph.
//!
//! No specification governs this: our own design, over `leptos_icons` and the
//! single Lucide family crate the workspace pins. The `icondata` umbrella
//! pulls every pack and is never a dependency here.
//!
//! `leptos_icons` writes `role="graphics-symbol"` on everything it draws and
//! adds neither an accessible name nor `aria-hidden`, which leaves an unnamed
//! node in the accessibility tree for every decorative glyph. Both components
//! below close that: [`Decoration`] hides the glyph, and [`Glyph`] names it.
//! Nothing else in this crate calls `leptos_icons::Icon` directly, so the
//! compensation cannot be forgotten one screen at a time.

use icondata_core::IconData;
use leptos::prelude::*;

/// The size ladder, in pixels.
///
/// Explicit pixels rather than `1em`: a glyph beside 14px text and a glyph in
/// an empty-state medallion want different sizes, and `1em` gives them the
/// same one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Size {
    /// 12px, for a chip.
    Chip,
    /// 14px, beside the text of a button or a link. The workhorse.
    Inline,
    /// 16px, for a rail entry.
    Rail,
    /// 18px, for a topbar control.
    Topbar,
    /// 20px, for a stat plate or an empty-state medallion.
    Plate,
}

impl Size {
    /// The pixel count, as the attribute value both dimensions take.
    const fn px(self) -> &'static str {
        match self {
            Self::Chip => "12",
            Self::Inline => "14",
            Self::Rail => "16",
            Self::Topbar => "18",
            Self::Plate => "20",
        }
    }
}

/// A glyph that repeats what its surroundings already say.
///
/// It is hidden from assistive technology, because a screen reader announcing
/// "graphics symbol" beside a labelled control adds noise and no information.
/// Colour is inherited, always.
#[component]
pub(crate) fn Decoration(
    /// The Lucide glyph to draw.
    icon: &'static IconData,
    /// Where on the size ladder it sits.
    size: Size,
) -> impl IntoView {
    view! {
        <span aria-hidden="true" class="inline-flex shrink-0">
            <leptos_icons::Icon icon=icon width=size.px() height=size.px() />
        </span>
    }
}

/// A glyph that is the only thing carrying its meaning.
///
/// Its label is required, so an icon-only control cannot ship nameless. The
/// label is exposed on a wrapper the component owns rather than on the SVG,
/// because `leptos_icons` insists on its own `role` and takes no attribute
/// spread.
#[component]
pub(crate) fn Glyph(
    /// The Lucide glyph to draw.
    icon: &'static IconData,
    /// Where on the size ladder it sits.
    size: Size,
    /// What the glyph means, read aloud in place of the drawing.
    label: &'static str,
) -> impl IntoView {
    view! {
        <span role="img" aria-label=label class="inline-flex shrink-0">
            <leptos_icons::Icon icon=icon width=size.px() height=size.px() />
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::Size;

    #[test]
    fn the_size_ladder_climbs_and_never_repeats_a_step() {
        let ladder = [
            Size::Chip,
            Size::Inline,
            Size::Rail,
            Size::Topbar,
            Size::Plate,
        ];
        let pixels: Vec<u32> = ladder
            .iter()
            .map(|size| size.px().parse().unwrap_or_default())
            .collect();
        assert_eq!(pixels, vec![12, 14, 16, 18, 20]);
        assert!(
            pixels.windows(2).all(|pair| pair.first() < pair.last()),
            "the ladder has to climb, so a caller can read a step off it"
        );
    }
}
