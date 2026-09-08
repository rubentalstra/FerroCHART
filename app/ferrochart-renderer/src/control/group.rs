// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One group of a form, and the add and remove controls a repeatable one
//! carries.
//!
//! `C_OBJECT.occurrences` (openEHR AM Release-2.3.0 `AOM1.4.html` section
//! 4.3.6) is the ceiling and the floor. At either one the button is disabled
//! rather than offering an action that does nothing, because a button that
//! looks live and does nothing is how a clinician loses confidence in a form.
//!
//! Removing an occurrence forgets everything inside it. An occurrence a
//! clinician removed is one they said is not there, and leaving its entries
//! behind would commit a document they cannot see.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use leptos::prelude::*;

use crate::control::field::FieldView;
use crate::control::localized;
use crate::control::repeats::Repeats;
use crate::control::undetermined::UndeterminedView;
use crate::kit::badge::BADGE;
use crate::kit::field::HINT;
use crate::kit::notice::Notice;
use crate::kit::surface::{CARD_PAD, CARD_TITLE};
use crate::kit::tone::Tone;
use crate::state::{FormState, Path, descend, root_path};

/// The whole form, rooted in the group the template root became.
#[component]
pub(crate) fn FormBody(
    /// The form, as the compiler derived it.
    definition: FormDefinition,
    /// What the clinician has entered so far.
    state: FormState,
    /// The language labels are shown in.
    language: LanguageTag,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <GroupView group=definition.root state=state path=root_path() language=language />
        </div>
    }
}

/// One group of a form, drawn once per occurrence the form is showing.
///
/// The return type is erased rather than opaque, because a group holds
/// groups: an opaque type would be an argument to itself.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn GroupView(
    /// The group, as the compiler derived it.
    group: FormGroup,
    /// The form the group writes into.
    state: FormState,
    /// Which occurrence of each repeating group above this one.
    path: Path,
    /// The language a label is shown in.
    language: LanguageTag,
) -> AnyView {
    let repeatable = group.occurrences.is_repeatable();
    let key = group.key.clone();
    let minimum = group.occurrences.minimum;
    let maximum = group.occurrences.maximum;
    let label = localized(&group.label, &language);
    let help = localized(&group.help, &language);

    let occurrences = {
        let key = key.clone();
        let path = path.clone();
        move || {
            if repeatable {
                (0..state.shown(&key, &path)).collect::<Vec<_>>()
            } else {
                vec![0]
            }
        }
    };

    let bodies = {
        let group = group.clone();
        let language = language.clone();
        let path = path.clone();
        move || {
            occurrences()
                .into_iter()
                .map(|occurrence| {
                    let inside = if repeatable {
                        descend(&path, occurrence)
                    } else {
                        path.clone()
                    };
                    view! {
                        <div class="flex flex-col gap-3">
                            <Show when=move || repeatable>
                                <p class="text-xs font-medium text-ink-muted">
                                    {format!("Occurrence {}", occurrence.saturating_add(1))}
                                </p>
                            </Show>
                            {contents(&group, state, &inside, &language)}
                        </div>
                    }
                })
                .collect::<Vec<_>>()
        }
    };

    let add = {
        let key = key.clone();
        let path = path.clone();
        Callback::new(move |()| {
            state.add_occurrence(&key, &path, maximum);
        })
    };
    let remove = {
        let key = key.clone();
        let path = path.clone();
        Callback::new(move |()| {
            state.remove_group_occurrence(&key, &path, minimum);
        })
    };
    let at_ceiling = {
        let key = key.clone();
        let path = path.clone();
        Signal::derive(move || !state.can_add(&key, &path, maximum))
    };
    let at_floor = Signal::derive(move || !state.can_remove(&key, &path, minimum));
    let bound = group.occurrences.to_string();

    view! {
        <section class=format!("{CARD_PAD} flex flex-col gap-3")>
            <div class="flex flex-wrap items-center gap-2">
                <h3 class=CARD_TITLE>{label}</h3>
                <Show when=move || repeatable>
                    <span class=format!("{BADGE} {}", Tone::Neutral.subtle())>{bound.clone()}</span>
                </Show>
            </div>
            <Show when={
                let help = help.clone();
                move || !help.is_empty()
            }>
                <p class=HINT>{help.clone()}</p>
            </Show>
            {bodies}
            <Show when=move || repeatable>
                <Repeats
                    add=add
                    remove=remove
                    at_ceiling=at_ceiling
                    at_floor=at_floor
                    add_label="Add one"
                    remove_label="Remove the last"
                />
            </Show>
        </section>
    }
    .into_any()
}

/// Everything one occurrence of a group holds: its items, then the holes the
/// template left in it.
fn contents(
    group: &FormGroup,
    state: FormState,
    inside: &Path,
    language: &LanguageTag,
) -> Vec<AnyView> {
    let mut drawn: Vec<AnyView> = group
        .items
        .iter()
        .map(|item| match *item {
            FormItem::Group(ref nested) => view! {
                <GroupView
                    group=(**nested).clone()
                    state=state
                    path=inside.clone()
                    language=language.clone()
                />
            }
            .into_any(),
            FormItem::Field(ref field) => view! {
                <FieldView
                    field=(**field).clone()
                    state=state
                    path=inside.clone()
                    language=language.clone()
                />
            }
            .into_any(),
            // `FormItem` is `#[non_exhaustive]`, so the compiler asks for
            // this arm.
            _ => view! { <Notice tone=Tone::Warn title="This renderer does not know this kind of item." /> }
            .into_any(),
        })
        .collect();
    drawn.extend(group.undetermined.iter().map(|content| {
        view! { <UndeterminedView content=content.clone() language=language.clone() /> }.into_any()
    }));
    drawn
}
