// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The inline notice: one block that tells a reader something about the
//! region it sits in.
//!
//! One definition for the whole family. The tone picks the colours and the
//! glyph ([`crate::kit::tone::Tone`]), and nothing else changes, so an
//! informational note and a refusal cannot drift into two different shapes.
//!
//! A notice is announced. A refusal carries `role="alert"` and interrupts;
//! every other tone carries `role="status"` and waits for the reader to be
//! idle.

use leptos::prelude::*;

use crate::icon::{Decoration, Size};
use crate::kit::tone::Tone;

/// A block of text about the region it sits in.
#[component]
pub(crate) fn Notice(
    /// What the notice says about the region.
    tone: Tone,
    /// The one line a reader takes away.
    #[prop(into)]
    title: String,
    /// The detail under it, or nothing.
    #[prop(optional, into)]
    detail: Option<String>,
    /// Anything the notice offers to do about it.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div
            role=tone.live_role()
            class=format!(
                "flex items-start gap-2.5 rounded-card border px-3 py-2.5 text-sm {}",
                tone.subtle(),
            )
        >
            <span class="mt-0.5">
                <Decoration icon=tone.icon() size=Size::Inline />
            </span>
            <div class="min-w-0 flex-1">
                <p class="font-medium">{title}</p>
                {detail.map(|line| view! { <p class="mt-0.5 text-ink-muted">{line}</p> })}
                {children.map(|actions| view! { <div class="mt-2 flex gap-2">{actions()}</div> })}
            </div>
        </div>
    }
}
