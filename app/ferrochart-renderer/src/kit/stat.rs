// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The stat tile: one number a reader scans, with the word that says what it
//! counts.
//!
//! One definition. The number is large and the word is small, and the pair is
//! read as one thing by assistive technology, because "412" on its own is not
//! a fact.

use leptos::prelude::*;

use crate::icon::{Decoration, Size};
use crate::kit::surface::CARD_PAD;
use crate::kit::tone::Tone;

/// One number, and what it counts.
#[component]
pub(crate) fn StatTile(
    /// What the number counts.
    #[prop(into)]
    label: String,
    /// The number itself, already formatted.
    #[prop(into)]
    value: Signal<String>,
    /// One line under the number, or nothing.
    #[prop(optional, into)]
    detail: Option<String>,
    /// The glyph in the plate, or nothing.
    #[prop(optional)]
    icon: Option<&'static icondata_core::IconData>,
    /// What the number says about the thing it counts.
    #[prop(optional)]
    tone: Option<Tone>,
) -> impl IntoView {
    let plate = tone.unwrap_or(Tone::Neutral);
    view! {
        <div class=format!(
            "{CARD_PAD} flex items-start gap-3",
        )>
            {icon
                .map(|glyph| {
                    view! {
                        <span class=format!(
                            "flex h-9 w-9 shrink-0 items-center justify-center rounded-control border {}",
                            plate.subtle(),
                        )>
                            <Decoration icon=glyph size=Size::Plate />
                        </span>
                    }
                })} <div class="min-w-0">
                <p class="text-xs font-medium uppercase tracking-wide text-ink-muted">{label}</p>
                <p class="mt-0.5 text-2xl font-semibold tabular-nums text-ink">
                    {move || value.get()}
                </p>
                {detail.map(|line| view! { <p class="mt-0.5 text-xs text-ink-muted">{line}</p> })}
            </div>
        </div>
    }
}
