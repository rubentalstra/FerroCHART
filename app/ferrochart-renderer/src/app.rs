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

#[cfg(feature = "design")]
use crate::design::DesignSystem;
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
            <Routes fallback=OffTheMap>
                <ParentRoute path=path!("") view=move || view! { <Shell theme=theme /> }>
                    <Route path=path!("") view=Form />
                    <Route path=path!("forms") view=Form />
                    <Route path=path!("forms/:template_id") view=Form />
                    <Route path=path!("templates") view=Templates />
                    <Route path=path!("layout") view=Layout />
                    <Route path=path!("commits") view=Commits />
                    <Route path=path!("settings") view=Settings />
                    <Route path=path!("design") view=DesignSystem />
                    // An address under the base that no route above owns.
                    // It sits INSIDE the shell, so a reader who mistypes one
                    // keeps the rail, the theme control and every way out.
                    // The outer fallback stays for an address outside the
                    // base, where there is no application to stay inside.
                    <Route path=path!("*any") view=NotFound />
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// The style guide, in a build that does not carry it.
///
/// The route stays so the router has one shape rather than two, and a build
/// without the guide answers its address the way it answers any address it
/// does not serve.
#[cfg(not(feature = "design"))]
#[component]
fn DesignSystem() -> impl IntoView {
    view! { <NotFound /> }
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
    view! {
        <Pending
            title="Layout"
            note="Where a form is laid out: the order of the fields, what they are \
            called, and which of them a person is asked for. Not built yet."
        />
    }
}

/// The commit log.
#[component]
fn Commits() -> impl IntoView {
    view! {
        <Pending
            title="Commits"
            note="What this server posted to the CDR, and what came back. Not built \
            yet."
        />
    }
}

/// The settings screen.
#[component]
fn Settings() -> impl IntoView {
    view! {
        <Pending
            title="Settings"
            note="The CDR and terminology endpoints this server uses. Not built yet."
        />
    }
}

/// The screen for an address no route owns.
///
/// Drawn as ordinary content, because it is reached from inside the shell and
/// keeps the frame around it. [`OffTheMap`] is the same content for the one
/// address that has no shell to sit in.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class=CARD_PAD>
            <p class="text-sm font-semibold text-ink">"That address does not name a screen."</p>
            <a
                href=crate::nav::href(crate::nav::TEMPLATES)
                class="mt-2 inline-block text-sm text-accent hover:underline"
            >
                "Go to the template library"
            </a>
        </div>
    }
}

/// An address outside the application entirely.
///
/// The router is based at `/ui`, so this is reached only by an address the
/// server handed to the bundle and the bundle does not own. There is no shell
/// to sit inside, so the content is centred on a bare ground.
#[component]
fn OffTheMap() -> impl IntoView {
    view! {
        <div class="flex min-h-screen flex-col items-center justify-center bg-surface p-6 text-center">
            <NotFound />
        </div>
    }
}
