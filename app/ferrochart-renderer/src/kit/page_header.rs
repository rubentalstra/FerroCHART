// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The block every screen opens with: a breadcrumb, a title, and the screen's
//! own actions.
//!
//! One definition. A screen that writes its own heading is how two pages
//! start disagreeing about where the title sits.

use leptos::prelude::*;

/// One step in the breadcrumb.
#[derive(Clone, Debug)]
pub(crate) struct Crumb {
    /// What the step is called.
    pub(crate) label: String,
    /// Where it leads, or `None` for the step the reader is already on.
    pub(crate) href: Option<String>,
}

impl Crumb {
    /// A step that leads somewhere.
    pub(crate) fn link(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: Some(href.into()),
        }
    }

    /// The last step, which is the screen the reader is on.
    pub(crate) fn here(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
        }
    }
}

/// The page header.
#[component]
pub(crate) fn PageHeader(
    /// The screen's name.
    title: Signal<String>,
    /// One line under the title, or nothing.
    #[prop(optional, into)]
    subtitle: Option<Signal<String>>,
    /// The trail above the title. Fewer than two entries draws no breadcrumb
    /// at all: a trail of one names the screen its own heading names.
    #[prop(optional)]
    crumbs: Vec<Crumb>,
    /// Whether the title is an identifier and therefore monospace.
    #[prop(optional)]
    mono: bool,
    /// The screen's actions, laid out to the right of the title.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    // A trail of one entry is not a trail: it names the screen the heading
    // under it already names, so Layout drew "Layout" twice (issue #193).
    let trail = (crumbs.len() > 1).then(|| {
        let steps: Vec<_> = crumbs
            .into_iter()
            .map(|crumb| match crumb.href {
                Some(href) => view! {
                    <li class="flex items-center gap-1">
                        <a href=href class="hover:text-ink">
                            {crumb.label}
                        </a>
                        <span aria-hidden="true">"/"</span>
                    </li>
                }
                .into_any(),
                None => view! {
                    <li aria-current="page" class="text-ink">
                        {crumb.label}
                    </li>
                }
                .into_any(),
            })
            .collect();
        view! {
            <nav aria-label="Breadcrumb" class="mb-1">
                <ol class="flex flex-wrap items-center gap-1 text-xs text-ink-muted">{steps}</ol>
            </nav>
        }
    });

    view! {
        <div class="mb-6 flex flex-wrap items-end justify-between gap-3">
            <div class="min-w-0">
                {trail}
                <h1 class=move || {
                    if mono {
                        "truncate font-mono text-xl font-semibold text-ink"
                    } else {
                        "truncate text-xl font-semibold text-ink"
                    }
                }>{move || title.get()}</h1>
                {subtitle
                    .map(|line| {
                        view! { <p class="mt-1 text-sm text-ink-muted">{move || line.get()}</p> }
                    })}
            </div>
            {children
                .map(|actions| view! { <div class="flex items-center gap-2">{actions()}</div> })}
        </div>
    }
}
