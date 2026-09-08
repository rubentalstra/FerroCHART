// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The router, and the screens hanging off it.
//!
//! The template library and one form are real screens and live in
//! [`crate::screen`]. The rest carry their frame and not yet their content:
//! the overlay authoring surface is issue #27, and the controls a clinician
//! types into are issue #137.

use leptos::prelude::*;
use leptos_meta::{Meta, Title, provide_meta_context};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::kit::page_header::{Crumb, PageHeader};
use crate::kit::surface::CARD_PAD;
use crate::nav;
use crate::screen::form::Form;
use crate::screen::templates::Templates;
use crate::shell::Shell;
use crate::theme;

/// The application.
#[component]
pub(crate) fn App() -> impl IntoView {
    provide_meta_context();
    let theme = theme::install();

    view! {
        <Title text="FerroCHART" />
        <Meta name="color-scheme" content="light dark" />
        <Router base=nav::BASE>
            <Routes fallback=NotFound>
                <ParentRoute path=path!("") view=move || view! { <Shell theme=theme /> }>
                    <Route path=path!("") view=Form />
                    <Route path=path!("forms") view=Form />
                    <Route path=path!("forms/:template_id") view=Form />
                    <Route path=path!("templates") view=Templates />
                    <Route path=path!("layout") view=Layout />
                    <Route path=path!("commits") view=Commits />
                    <Route path=path!("settings") view=Settings />
                    <Route path=path!("design") view=crate::design::DesignSystem />
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// A screen that has its frame and not yet its content.
#[component]
fn Pending(
    /// The screen's name.
    title: &'static str,
    /// What it will do, and the issue that builds it.
    note: &'static str,
) -> impl IntoView {
    view! {
        <PageHeader
            title=Signal::derive(move || title.to_owned())
            crumbs=vec![Crumb::here(title)]
        />
        <div class=CARD_PAD>
            <p class="text-sm text-ink-muted">{note}</p>
        </div>
    }
}

/// The layout overlay authoring surface.
#[component]
fn Layout() -> impl IntoView {
    view! { <Pending title="Layout" note="The overlay authoring surface lands with issue #27." /> }
}

/// The commit log.
#[component]
fn Commits() -> impl IntoView {
    view! { <Pending title="Commits" note="What this server posted to the CDR, and what came back." /> }
}

/// The settings screen.
#[component]
fn Settings() -> impl IntoView {
    view! { <Pending title="Settings" note="The CDR and terminology endpoints this server uses." /> }
}

/// The screen for an address no route owns.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="flex min-h-screen flex-col items-center justify-center gap-3 bg-surface p-6 text-center">
            <p class="text-sm font-semibold text-ink">"That address does not name a screen."</p>
            <a href=crate::nav::href(crate::nav::FORMS) class="text-sm text-accent hover:underline">
                "Go to Forms"
            </a>
        </div>
    }
}
