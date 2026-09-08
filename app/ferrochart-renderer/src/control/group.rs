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
use ferrochart_form::group::{FormGroup, FormItem, GroupShape};
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
    let label = crate::label::of(&group.label, &language, &group.key, "a group of fields");
    let help = localized(&group.help, &language);

    let occurrences = {
        let key = key.clone();
        let path = path.clone();
        move || {
            if repeatable {
                (0..state.shown(&key, &path, minimum as usize)).collect::<Vec<_>>()
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
                                    {format!("Entry {}", occurrence.saturating_add(1))}
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
            state.add_occurrence(&key, &path, minimum, maximum);
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
        Signal::derive(move || !state.can_add(&key, &path, minimum, maximum))
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

/// Whether the group is the Reference Model's own container rather than a
/// grouping a person would recognize.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 4.3 makes
/// `ITEM_SINGLE`, `ITEM_LIST` and `ITEM_TREE` the `ITEM_STRUCTURE` an ENTRY
/// attribute holds its content in. They exist because the Reference Model
/// needs a container there, not because anybody grouped anything, and the CKM
/// archetypes label them accordingly: the one on the family history form is
/// headed "Tree" and described "@ internal @".
///
/// So the group's contents are drawn where the group would have been. Nothing
/// is invented and nothing is dropped: the node keeps its key, its values and
/// its place in the composition, and only the card and the heading go. An
/// `ITEM_TABLE` keeps both, because a grid is a shape a person reads, and a
/// `CLUSTER` keeps both, because a cluster IS the clinical grouping.
///
/// A structure node that repeats keeps its card too, because the add and
/// remove pair has to hang on something.
fn is_plumbing(group: &FormGroup) -> bool {
    matches!(
        group.shape,
        GroupShape::Single | GroupShape::List | GroupShape::Tree
    ) && group.archetype_id.is_none()
        && !group.occurrences.is_repeatable()
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
            FormItem::Group(ref nested) if is_plumbing(nested) => {
                contents(nested, state, inside, language).into_any()
            }
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

#[cfg(test)]
mod tests {
    use ferrochart_form::group::{FormGroup, GroupShape};
    use ferrochart_form::ids::{ArchetypeId, RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};
    use ferrochart_form::occurrences::Occurrences;
    use ferrochart_form::text::Localized;

    use super::is_plumbing;

    /// A group of `shape`, holding nothing.
    fn group(shape: GroupShape) -> FormGroup {
        FormGroup {
            key: NodeKey::root().child(KeyStep {
                rm_attribute: RmAttributeName::new("data"),
                node_id: None,
                archetype_id: None,
                rm_type: RmTypeName::new("ITEM_TREE"),
                pinned_name: None,
                sibling_ordinal: 0,
            }),
            rm_type: RmTypeName::new("ITEM_TREE"),
            archetype_id: None,
            label: Localized::empty(),
            help: Localized::empty(),
            occurrences: Occurrences::bounded(1, 1),
            shape,
            name_constraint: None,
            is_ordered: false,
            is_unique: false,
            is_deprecated: false,
            items: Vec::new(),
            undetermined: Vec::new(),
        }
    }

    #[test]
    fn the_reference_models_own_container_is_plumbing() {
        // openEHR RM Release-1.1.0 `data_structures.html` section 4.3: these
        // hold an ENTRY's content because the model needs a container there,
        // and the CKM archetypes head them "Tree" and describe them
        // "@ internal @".
        for shape in [GroupShape::Single, GroupShape::List, GroupShape::Tree] {
            assert!(is_plumbing(&group(shape)), "{shape:?}");
        }
    }

    #[test]
    fn a_cluster_and_a_table_are_not_plumbing() {
        // A cluster IS the clinical grouping, and a table is a shape a person
        // reads.
        assert!(!is_plumbing(&group(GroupShape::Cluster)));
        assert!(!is_plumbing(&group(GroupShape::Table)));
        assert!(!is_plumbing(&group(GroupShape::Plain)));
    }

    #[test]
    fn a_container_that_roots_an_archetype_keeps_its_heading() {
        let mut rooted = group(GroupShape::Tree);
        rooted.archetype_id = Some(ArchetypeId::new("openEHR-EHR-CLUSTER.x.v1"));
        assert!(!is_plumbing(&rooted));
    }

    #[test]
    fn a_container_that_repeats_keeps_its_heading() {
        // The add and remove pair has to hang on something.
        let mut repeats = group(GroupShape::Tree);
        repeats.occurrences = Occurrences::unbounded_from(0);
        assert!(!is_plumbing(&repeats));
    }
}
