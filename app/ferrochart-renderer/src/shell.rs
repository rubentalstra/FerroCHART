// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The frame every screen renders inside.
//!
//! No specification governs this: our own design, following the proportions
//! of the `FerroEHR` viewer so the two products read as one family: a full-width
//! 56px topbar above everything, a 208px rail, a 40px footer.
//!
//! **The third column is FerroCHART's own.** The sibling's right-hand panel
//! is a transient drawer, because a viewer only ever reads. A form builder
//! selects a node and edits its layout, so the inspector is persistent.
//!
//! Below the breakpoint the sibling hides its rail with no scrim and no focus
//! management. This one turns it into an overlay: the scrim covers the page,
//! Escape closes it, Tab cannot leave it, and focus returns to the control
//! that opened it. A builder is a keyboard tool
//! (`docs/architecture.md` section 10 commits to WCAG 2.2 AA and to
//! keyboard-only entry), and a trap that leaks is how a keyboard reader ends
//! up typing into a page they cannot see.

use leptos::ev::KeyboardEvent;
use leptos::html;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;

use crate::focus::{focus, focus_first, trap};
use crate::icon::{Decoration, Size};
use crate::kit::field::BTN_QUIET;
use crate::nav::{self, Slot};
use crate::theme::Theme;

/// The id the skip link jumps to.
const MAIN_ID: &str = "main";

/// The application frame.
#[component]
pub(crate) fn Shell(
    /// The theme, so the topbar can switch it.
    theme: RwSignal<Theme>,
) -> impl IntoView {
    let rail_open = RwSignal::new(false);
    let toggle: NodeRef<html::Button> = NodeRef::new();
    let rail: NodeRef<html::Nav> = NodeRef::new();

    // The overlay is a region a keyboard reader is inside, so opening it
    // moves focus in and closing it puts focus back where it came from.
    Effect::new(move |_| {
        if rail_open.get() {
            focus_first(rail.get());
        }
    });

    let close_rail = move || {
        rail_open.set(false);
        if let Some(button) = toggle.get() {
            focus(&button.into());
        }
    };

    let on_key = move |event: KeyboardEvent| {
        if !rail_open.get() {
            return;
        }
        match event.key().as_str() {
            "Escape" => {
                event.prevent_default();
                close_rail();
            }
            "Tab" => trap(&event, rail.get()),
            _ => {}
        }
    };

    view! {
        <div class="flex min-h-screen flex-col bg-surface text-ink">
            <a
                href=format!("#{MAIN_ID}")
                class="sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 \
                focus:z-50 focus:rounded-control focus:bg-raised focus:px-3 focus:py-2 \
                focus:text-sm focus:font-medium focus:text-ink"
            >
                "Skip to content"
            </a>

            <header class="flex h-14 shrink-0 items-center gap-3 border-b border-edge bg-raised px-4">
                <button
                    node_ref=toggle
                    type="button"
                    class=format!("{BTN_QUIET} md:hidden")
                    aria-controls="rail"
                    aria-label="Sections"
                    aria-expanded=move || if rail_open.get() { "true" } else { "false" }
                    on:click=move |_| rail_open.update(|open| *open = !*open)
                >
                    <Decoration icon=icondata_lu::LuMenu size=Size::Topbar />
                </button>
                <Brand />
                <div class="flex-1"></div>
                <button
                    type="button"
                    class=BTN_QUIET
                    aria-label=move || {
                        format!("Switch to the {} theme", theme.get().toggled().label())
                    }
                    on:click=move |_| theme.update(|current| *current = current.toggled())
                >
                    <Decoration icon=icondata_lu::LuSunMoon size=Size::Topbar />
                </button>
            </header>

            <div class="flex min-h-0 flex-1" on:keydown=on_key>
                <div
                    class="fixed inset-0 z-30 bg-scrim md:hidden"
                    class:hidden=move || !rail_open.get()
                    aria-hidden="true"
                    on:click=move |_| close_rail()
                ></div>
                <Rail rail_open=rail_open node=rail />
                <main id=MAIN_ID tabindex="-1" class="min-w-0 flex-1 overflow-auto p-6">
                    <Outlet />
                </main>
                <aside class="hidden w-inspector shrink-0 overflow-auto border-l border-edge bg-raised lg:block">
                    <div class="border-b border-edge px-4 py-3 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                        "Inspector"
                    </div>
                    <div class="p-4 text-sm text-ink-muted">
                        "Select a node to edit its layout."
                    </div>
                </aside>
            </div>

            <footer class="flex h-10 shrink-0 items-center gap-2 border-t border-edge bg-raised px-4 text-xs text-ink-muted">
                <span>"FerroCHART"</span>
                <span aria-hidden="true">"·"</span>
                <span class="font-mono">{env!("CARGO_PKG_VERSION")}</span>
            </footer>
        </div>
    }
}

/// The brand lockup, drawn inline.
///
/// The mark is inline SVG rather than an asset fetch: the served page's
/// content security policy allows no remote image, and one round trip for a
/// 24px drawing is a round trip the first paint waits on.
#[component]
fn Brand() -> impl IntoView {
    view! {
        <a href=nav::href(nav::FORMS) class="flex items-center gap-2">
            <svg
                viewBox="0 0 64 64"
                width="24"
                height="24"
                aria-hidden="true"
                fill="none"
                stroke="currentColor"
                stroke-width="4"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="shrink-0 text-accent"
            >
                <rect x="10" y="6" width="44" height="52" rx="6" />
                <path d="M20 22h24" />
                <path d="M20 34h14" />
                <path d="M20 46l6 6 12-14" />
            </svg>
            <span class="text-sm font-semibold tracking-tight text-ink">
                "Ferro"<span class="text-accent">"CHART"</span>
            </span>
        </a>
    }
}

/// The rail: a fixed column from the breakpoint up, an overlay below it.
#[component]
fn Rail(
    /// Whether the overlay form is showing.
    rail_open: RwSignal<bool>,
    /// The element the focus trap works over.
    node: NodeRef<html::Nav>,
) -> impl IntoView {
    let location = use_location();
    let active = move || nav::section(&location.pathname.get());

    let entries: Vec<_> = nav::SLOTS
        .iter()
        .map(|slot| match slot {
            Slot::Divider => view! { <li aria-hidden="true" class="my-2 border-t border-edge"></li> }
                .into_any(),
            Slot::Item(section, label, icon) => {
                let section = *section;
                let is_active = move || active() == Some(section);
                view! {
                    <li>
                        <a
                            href=nav::href(section)
                            aria-current=move || is_active().then_some("page")
                            class="flex items-center gap-2.5 rounded-control px-3 py-2 text-sm font-medium"
                            class=(["bg-accent-subtle", "text-accent-ink"], is_active)
                            class=(
                                ["text-ink-muted", "hover:bg-sunken", "hover:text-ink"],
                                move || !is_active(),
                            )
                        >
                            <Decoration icon=*icon size=Size::Rail />
                            {*label}
                        </a>
                    </li>
                }
                .into_any()
            }
        })
        .collect();

    view! {
        <nav
            node_ref=node
            id="rail"
            aria-label="Sections"
            class="w-rail shrink-0 overflow-auto border-r border-edge bg-raised p-2 \
            max-md:fixed max-md:inset-y-0 max-md:left-0 max-md:z-40"
            class:max-md:hidden=move || !rail_open.get()
        >
            <ul class="flex flex-col gap-0.5">{entries}</ul>
        </nav>
    }
}
