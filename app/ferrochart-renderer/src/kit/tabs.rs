// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Tab pills: one row of pills, of which one is on.
//!
//! One definition. The pills are real buttons in a `role="tablist"`, each one
//! reachable by Tab and operable by Enter or Space, because that is what a
//! button already does and a hand-rolled roving index is one more thing to
//! get wrong.
//!
//! Each pill owns the panel named by [`Tab::panel_id`], so `aria-controls`
//! and `aria-selected` say which panel a reader is about to land in.

use leptos::prelude::*;

/// One pill.
#[derive(Clone, Debug)]
pub(crate) struct Tab {
    /// What the pill is called.
    pub(crate) label: String,
    /// The id of the panel the pill shows, which is what `aria-controls`
    /// names.
    pub(crate) panel_id: String,
}

impl Tab {
    /// A pill over the panel with this id.
    pub(crate) fn new(label: impl Into<String>, panel_id: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            panel_id: panel_id.into(),
        }
    }
}

/// The row of pills.
#[component]
pub(crate) fn TabPills(
    /// What the row as a whole is called, for a reader who lands on it.
    #[prop(into)]
    label: String,
    /// The pills, in the order they are drawn.
    tabs: Vec<Tab>,
    /// Which pill is on, by its position in `tabs`.
    selected: RwSignal<usize>,
) -> impl IntoView {
    let pills: Vec<_> = tabs
        .into_iter()
        .enumerate()
        .map(|(at, tab)| {
            let is_on = move || selected.get() == at;
            view! {
                <button
                    type="button"
                    role="tab"
                    id=format!("{}-tab", tab.panel_id)
                    aria-controls=tab.panel_id
                    aria-selected=move || if is_on() { "true" } else { "false" }
                    class="rounded-control px-3 py-1.5 text-sm font-medium"
                    class=(["bg-accent-subtle", "text-accent-ink"], is_on)
                    class=(
                        ["text-ink-muted", "hover:bg-sunken", "hover:text-ink"],
                        move || !is_on(),
                    )
                    on:click=move |_| selected.set(at)
                >
                    {tab.label}
                </button>
            }
        })
        .collect();

    view! {
        <div role="tablist" aria-label=label class="flex flex-wrap gap-1 border-b border-edge pb-2">
            {pills}
        </div>
    }
}
