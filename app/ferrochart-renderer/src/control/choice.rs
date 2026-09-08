// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A field the clinician fills through exactly one of several alternatives.
//!
//! openEHR AM Release-2.3.0 `AOM2.html` section 4.2.8.2 prescribes two sibling
//! nodes, one coded and one text-only, for a value that may be either; the
//! alternatives under any single-valued attribute are the general case.
//!
//! Each alternative is a node of its own with its own key, so a filled
//! alternative is written under that key. The attribute holds one value, so
//! choosing an alternative clears every other one.

use ferrochart_form::field::ChoiceField;
use leptos::prelude::*;

use crate::control::{Slot, localized};
use crate::kit::field::{LABEL, SELECT};

/// Which alternative the values name.
///
/// More than one filled is a state the attribute cannot hold, so the first
/// filled one is the answer and a control clears the rest.
pub(crate) fn chosen(filled: &[bool]) -> Option<usize> {
    filled.iter().position(|held| *held)
}

/// A control over several alternatives, of which exactly one is filled.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn ChoiceControl(
    /// What the template admits.
    field: ChoiceField,
    /// Where the choice sits, which is what the alternatives hang off.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let language = slot.language.clone();
    let slots: Vec<Slot> = field
        .alternatives
        .iter()
        .map(|alternative| {
            slot.sibling(
                alternative.key.clone(),
                localized(&alternative.label, &language),
            )
        })
        .collect();
    let filled: Vec<bool> = slots
        .iter()
        .map(|alternative| alternative.entered().is_some())
        .collect();
    let picked = RwSignal::new(chosen(&filled).unwrap_or(0));

    let choices: Vec<_> = field
        .alternatives
        .iter()
        .enumerate()
        .map(|(at, alternative)| {
            let label = localized(&alternative.label, &language);
            let shown = if label.is_empty() {
                alternative.rm_type.as_str().to_owned()
            } else {
                label
            };
            view! { <option value=at.to_string()>{shown}</option> }
        })
        .collect();

    let bodies: Vec<_> = field
        .alternatives
        .iter()
        .zip(slots.iter())
        .enumerate()
        .map(|(at, (alternative, alternative_slot))| {
            let body = crate::control::field::control(
                &alternative.kind,
                &alternative.rm_type,
                alternative_slot,
            );
            view! { <div class:hidden=move || picked.get() != at>{body}</div> }
        })
        .collect();

    let picker_id = slot.part("alternative");
    let held = StoredValue::new(slots);
    let on_pick = move |event: leptos::ev::Event| {
        let at: usize = event_target_value(&event).parse().unwrap_or(0);
        // The attribute holds one value, so every other alternative is
        // forgotten the moment one is chosen.
        for (index, alternative) in held.get_value().iter().enumerate() {
            if index != at {
                alternative.clear();
            }
        }
        picked.set(at);
    };

    view! {
        <div class="flex flex-col gap-2">
            <div>
                <label class=LABEL for=picker_id.clone()>
                    "Which of these"
                </label>
                <select
                    id=picker_id
                    class=SELECT
                    prop:value=move || picked.get().to_string()
                    on:change=on_pick
                >
                    {choices}
                </select>
            </div>
            {bodies}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::chosen;

    #[test]
    fn nothing_filled_names_no_alternative() {
        assert_eq!(chosen(&[false, false]), None);
    }

    #[test]
    fn the_one_filled_alternative_is_the_one_the_control_shows() {
        assert_eq!(chosen(&[false, true, false]), Some(1));
    }

    #[test]
    fn two_filled_alternatives_collapse_to_the_first() {
        assert_eq!(
            chosen(&[false, true, true]),
            Some(1),
            "the attribute holds one value, so the control clears the rest"
        );
    }
}
