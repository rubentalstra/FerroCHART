// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The empty state: what a region says when it holds nothing.
//!
//! One definition. It says what would be here and what to do about it, so a
//! reader can tell an empty list from a list that failed to load. A region
//! that draws nothing at all is the same pixel as a bug.

use leptos::prelude::*;

use crate::icon::{Decoration, Size};

/// A region that holds nothing yet.
#[component]
pub(crate) fn EmptyState(
    /// The glyph in the medallion.
    icon: &'static icondata_core::IconData,
    /// What would be here.
    #[prop(into)]
    title: String,
    /// Why it is not, and what to do about it.
    #[prop(optional, into)]
    detail: Option<String>,
    /// The action that fills the region, or nothing.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center gap-2 rounded-card border border-dashed border-edge \
        bg-sunken px-6 py-10 text-center">
            <span class="flex h-10 w-10 items-center justify-center rounded-full border border-edge \
            bg-raised text-ink-muted">
                <Decoration icon=icon size=Size::Plate />
            </span>
            <p class="text-sm font-medium text-ink">{title}</p>
            {detail
                .map(|line| {
                    view! { <p class="max-w-prose text-sm text-ink-muted">{line}</p> }
                })}
            {children.map(|action| view! { <div class="mt-1">{action()}</div> })}
        </div>
    }
}
