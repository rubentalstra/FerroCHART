// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The template library: what this server holds, and the form each compiles
//! to.
//!
//! A pure read, so a failure is drawn inline and nothing toasts
//! (`docs/architecture.md` section 10.1).

use leptos::prelude::*;

use crate::api;
use crate::kit::notice::{Notice, Tone};
use crate::kit::page_header::{Crumb, PageHeader};
use crate::kit::surface::{CARD_PAD, CODE};
use crate::nav;
use crate::screen::inline::{Failed, detail};

/// The row one template gets.
const ROW: &str =
    "flex items-center justify-between gap-3 rounded-control px-2 py-2 hover:bg-sunken";

/// The template library.
#[component]
pub(crate) fn Templates() -> impl IntoView {
    let held = LocalResource::new(api::templates);

    let body = move || {
        held.map(|answer| match *answer {
            Ok(ref list) => listed(list).into_any(),
            Err(ref error) => {
                view! { <Failed title="The templates could not be read." lines=detail(error) /> }
                    .into_any()
            }
        })
        .unwrap_or_else(|| {
            view! { <p class="text-sm text-ink-muted">"Reading the template library…"</p> }
                .into_any()
        })
    };

    view! {
        <PageHeader
            title=Signal::derive(|| "Templates".to_owned())
            subtitle=Signal::derive(|| {
                "The operational templates this server has compiled.".to_owned()
            })
            crumbs=vec![Crumb::here("Templates")]
        />
        <div class=CARD_PAD>{body}</div>
    }
}

/// The list itself, or the notice that says the library is empty.
fn listed(list: &api::TemplateList) -> AnyView {
    if list.templates.is_empty() {
        return view! {
            <Notice tone=Tone::Info title="This server holds no template.">
                "Compile one and it appears here."
            </Notice>
        }
        .into_any();
    }
    let rows: Vec<_> = list
        .templates
        .iter()
        .map(|template_id| {
            let identifier = template_id.as_str().to_owned();
            let address = nav::form_href(&identifier);
            view! {
                <li>
                    <a href=address class=ROW>
                        <span class=CODE>{identifier}</span>
                        <span class="text-xs text-accent">"Open the form"</span>
                    </a>
                </li>
            }
        })
        .collect();
    view! { <ul class="-mx-2 divide-y divide-edge">{rows}</ul> }.into_any()
}

#[cfg(test)]
mod tests {
    use super::ROW;

    #[test]
    fn the_row_reaches_past_no_semantic_token() {
        assert!(!ROW.contains("dark:"), "dark mode is the tokens: {ROW}");
        for raw in ["slate-", "rose-", "gray-", "zinc-"] {
            assert!(!ROW.contains(raw), "raw palette `{raw}` in: {ROW}");
        }
    }
}
