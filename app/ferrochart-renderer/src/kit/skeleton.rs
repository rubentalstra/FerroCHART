// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The loading skeleton: the shape of what is coming, while it comes.
//!
//! One definition, and one only. A second skeleton would be a second guess at
//! what the pending region looks like, and the two would disagree the moment
//! either region was restyled.
//!
//! The bars are hidden from assistive technology and one `role="status"` node
//! carries the word, because a screen reader announcing eight grey rectangles
//! is worse than silence. The pulse is a CSS animation, which the
//! `prefers-reduced-motion` rule in the stylesheet already stops.

use leptos::prelude::*;

/// A region whose content has not arrived.
#[component]
pub(crate) fn Skeleton(
    /// How many bars to draw.
    #[prop(default = 3)]
    rows: usize,
    /// What is loading, read aloud in place of the bars.
    #[prop(into)]
    label: String,
) -> impl IntoView {
    let bars: Vec<_> = (0..rows)
        .map(|at| {
            // The last bar is short, so the block reads as text rather than
            // as a table that lost its borders.
            let width = if at + 1 == rows { "w-2/3" } else { "w-full" };
            view! { <span class=format!("block h-3 rounded-control bg-sunken {width}")></span> }
        })
        .collect();

    view! {
        <div class="flex flex-col gap-2">
            <span role="status" class="sr-only">
                {label}
            </span>
            <div aria-hidden="true" class="flex animate-pulse flex-col gap-2">
                {bars}
            </div>
        </div>
    }
}
