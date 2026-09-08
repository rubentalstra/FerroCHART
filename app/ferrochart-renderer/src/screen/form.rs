// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One form, as the server compiled it.
//!
//! The screen draws the group tree the definition carries: a heading per
//! group, and the fields of that group under it. The controls a clinician
//! types into are issue #137 and slot in beneath these labels.
//!
//! Field order is template order and grouping is the Reference Model tree
//! (`docs/architecture.md` section 4), so this screen decides no layout of
//! its own. Everything a person would want to decide lives in the overlay,
//! which is issue #27.
//!
//! A pure read, so a failure is drawn inline and nothing toasts
//! (`docs/architecture.md` section 10.1).

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::kit::notice::Notice;
use crate::kit::page_header::{Crumb, PageHeader};
use crate::kit::surface::{CARD_PAD, CODE};
use crate::kit::tone::Tone;
use crate::label;
use crate::nav;
use crate::screen::inline::{Failed, detail};

/// The route parameter that names the template.
pub(crate) const TEMPLATE_PARAM: &str = "template_id";

/// How far one level of the tree is indented.
const NESTED: &str = "ml-4 border-l border-edge pl-4";

/// The heading of a group.
const GROUP_HEADING: &str = "text-sm font-semibold text-ink";

/// One field's line.
const FIELD_ROW: &str = "flex flex-wrap items-baseline gap-2 py-1";

/// One form.
#[component]
pub(crate) fn Form() -> impl IntoView {
    let params = use_params_map();
    let wanted = move || params.read().get(TEMPLATE_PARAM).unwrap_or_default();

    let compiled = LocalResource::new(move || {
        let template_id = wanted();
        async move { api::definition(&template_id).await }
    });

    let body = move || {
        if wanted().is_empty() {
            return view! {
                <Notice tone=Tone::Neutral title="No template is named.">
                    <a href=nav::href(nav::TEMPLATES) class="text-accent hover:underline">
                        "Choose one from the template library."
                    </a>
                </Notice>
            }
            .into_any();
        }
        compiled
            .map(|answer| match *answer {
                Ok(ref definition) => tree(definition).into_any(),
                Err(ref error) => {
                    view! { <Failed title="The form could not be read." lines=detail(error) /> }
                        .into_any()
                }
            })
            .unwrap_or_else(|| {
                view! { <p class="text-sm text-ink-muted">"Reading the form…"</p> }.into_any()
            })
    };

    let title = Signal::derive(move || {
        let named = wanted();
        if named.is_empty() {
            "Forms".to_owned()
        } else {
            named
        }
    });

    view! {
        <PageHeader
            title=title
            mono=true
            crumbs=vec![Crumb::link("Templates", nav::href(nav::TEMPLATES)), Crumb::here("Form")]
        />
        <div class=CARD_PAD>{body}</div>
    }
}

/// The whole tree of one definition.
fn tree(definition: &FormDefinition) -> AnyView {
    let language = definition.default_language.clone();
    let undetermined = definition.undetermined().count();
    let unread = (!definition.is_current_format()).then(|| {
        view! {
            <Notice tone=Tone::Warn title="This form states another format version.">
                {format!(
                    "It is written in version {}, and this renderer reads version {}.",
                    definition.format_version,
                    ferrochart_form::definition::FORMAT_VERSION,
                )}
            </Notice>
        }
    });
    let hole = (undetermined > 0).then(|| {
        view! {
            <Notice tone=Tone::Warn title="The template left content undetermined.">
                {format!("{undetermined} nodes are recorded and drawn nowhere.")}
            </Notice>
        }
    });
    view! { <div class="space-y-3">{unread} {hole} {group_view(&definition.root, &language)}</div> }
        .into_any()
}

/// One group, and everything under it.
fn group_view(group: &FormGroup, language: &LanguageTag) -> AnyView {
    let heading = label::of(&group.label, language, &group.key);
    let help = label::text(&group.help, language).map(str::to_owned);
    let members: Vec<_> = group
        .items
        .iter()
        .map(|item| match *item {
            FormItem::Group(ref nested) => {
                view! { <div class=NESTED>{group_view(nested, language)}</div> }.into_any()
            }
            FormItem::Field(ref field) => {
                let name = label::of(&field.label, language, &field.key);
                let rm_type = field.rm_type.as_str().to_owned();
                view! {
                    <div class=FIELD_ROW>
                        <span class="text-sm text-ink">{name}</span>
                        <span class=CODE>{rm_type}</span>
                    </div>
                }
                .into_any()
            }
            // NOTE: `FormItem` is `#[non_exhaustive]`, so a member kind added
            // to the published contract is recorded on the screen rather than
            // dropped from the tree.
            _ => view! { <p class="text-xs text-warn">"This form holds an item this renderer cannot draw."</p> }
            .into_any(),
        })
        .collect();
    let empty = group.items.is_empty().then(|| {
        view! { <p class="text-xs text-ink-faint">"This group holds nothing."</p> }
    });
    view! {
        <section class="space-y-1">
            <h2 class=GROUP_HEADING>{heading}</h2>
            {help.map(|line| view! { <p class="text-xs text-ink-muted">{line}</p> })}
            {empty}
            <div>{members}</div>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::{FIELD_ROW, GROUP_HEADING, NESTED, TEMPLATE_PARAM};

    /// Every class constant this screen owns.
    const ALL: [&str; 3] = [NESTED, GROUP_HEADING, FIELD_ROW];

    #[test]
    fn no_constant_reaches_past_the_semantic_tokens() {
        for class in ALL {
            assert!(!class.contains("dark:"), "dark mode is the tokens: {class}");
            for raw in ["slate-", "rose-", "gray-", "zinc-", "red-"] {
                assert!(!class.contains(raw), "raw palette `{raw}` in: {class}");
            }
        }
    }

    #[test]
    fn the_route_parameter_is_one_segment() {
        assert!(!TEMPLATE_PARAM.contains('/'), "{TEMPLATE_PARAM}");
        assert!(!TEMPLATE_PARAM.is_empty());
    }
}
