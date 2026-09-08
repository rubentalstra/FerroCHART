// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One form, as the server compiled it.
//!
//! The screen draws the group tree the definition carries: a heading per
//! group, and the fields of that group under it. The controls a clinician
//! types into are issue #137 and slot in beneath these labels.
//!
//! Field order, grouping, labels and conditional visibility come from the
//! layout a person authored, where the server serves one
//! (`ferrochart_form::layout`). Where it serves none, order is template order
//! and grouping is the Reference Model tree (`docs/architecture.md` section
//! 4). The surface a person authors the layout ON is issue #27.
//!
//! A pure read, so a failure is drawn inline and nothing toasts
//! (`docs/architecture.md` section 10.1).

use ferrochart_form::definition::FormDefinition;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::control::group::FormBody;
use crate::kit::notice::Notice;
use crate::kit::page_header::{Crumb, PageHeader};
use crate::kit::surface::CARD_PAD;
use crate::kit::tone::Tone;
use crate::nav;
use crate::overlay::Laid;
use crate::screen::inline::{Failed, detail};
use crate::state::FormState;

/// The route parameter that names the template.
pub(crate) const TEMPLATE_PARAM: &str = "template_id";

/// One form.
#[component]
pub(crate) fn Form() -> impl IntoView {
    let params = use_params_map();
    let wanted = move || params.read().get(TEMPLATE_PARAM).unwrap_or_default();

    // The address with no template named is where the rail lands a reader who
    // clicks Forms, and there is nothing to ask for there. Asking anyway sends
    // `/api/templates//definition`, which the server answers 404, so the
    // guard is on the request and not only on what is drawn.
    let compiled = LocalResource::new(move || {
        let template_id = wanted();
        async move {
            if template_id.is_empty() {
                return None;
            }
            let definition = api::definition(&template_id).await;
            // The layout is fetched beside the definition rather than after
            // it, so a form never draws once in template order and then jumps
            // into the order a person authored. A server that cannot serve one
            // still serves the form: what a person authored is lost for that
            // reader, and the template is not.
            let laid = match api::layout(&template_id).await {
                Ok(layout) if layout.is_current_format() => Laid::of(layout),
                Ok(_) | Err(_) => Laid::none(),
            };
            Some((definition, laid))
        }
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
                Some((Ok(ref definition), ref laid)) => tree(definition, laid.clone()).into_any(),
                Some((Err(ref error), _)) => {
                    view! { <Failed title="The form could not be read." lines=detail(error) /> }
                        .into_any()
                }
                None => ().into_any(),
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

/// The whole form of one definition, drawn as controls a clinician fills.
fn tree(definition: &FormDefinition, laid: Laid) -> AnyView {
    let language = definition.default_language.clone();
    // An open slot is a place the template deliberately left for extra
    // content (openEHR AM Release-2.3.0 `AOM2.html` section 4.5.8), so it is
    // not counted here. Counting it made every form of the committed pack open
    // with a warning about itself, 116 of 121 of them (issue #180).
    let undetermined = definition
        .undetermined()
        .filter(|content| {
            !matches!(
                content.reason,
                ferrochart_form::group::UndeterminedReason::OpenSlot { .. }
            )
        })
        .count();
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
                {format!(
                    "{undetermined} {} recorded and drawn nowhere.",
                    if undetermined == 1 { "node is" } else { "nodes are" },
                )}
            </Notice>
        }
    });
    // The state is created from the definition, so a repeatable group opens
    // showing exactly the occurrences the template requires.
    let state = FormState::new(definition);
    view! {
        <div class="space-y-3">
            {unread} {hole}
            <FormBody definition=definition.clone() state=state language=language laid=laid />
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::TEMPLATE_PARAM;

    // The palette assertion this module used to make covered three class
    // constants that the controls replaced. The rule it checked is enforced
    // tree-wide by `scripts/checks/palette-utilities.sh`, over files nobody
    // has written yet as well as this one.

    #[test]
    fn the_route_parameter_is_one_segment() {
        assert!(!TEMPLATE_PARAM.contains('/'), "{TEMPLATE_PARAM}");
        assert!(!TEMPLATE_PARAM.is_empty());
    }
}
