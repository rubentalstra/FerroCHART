// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_STATE`: one state of a state machine.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 4.2.3. The template
//! carries the states and the transitions out of each one, and the
//! transitions are what a state can move to, so a control offers every state
//! while none is chosen and only the reachable ones once one is.

use ferrochart_form::field::{StateField, StateOption};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::control::{RefusalNote, Slot};
use crate::kit::badge::StatusPill;
use crate::kit::field::SELECT;
use crate::kit::tone::Tone;

/// The states reachable from where the value is now.
///
/// Every state while nothing is chosen. Once a state is chosen, the state
/// itself and whatever its transitions name, because those are the only ways
/// out of it.
pub(crate) fn reachable<'a>(field: &'a StateField, from: Option<&str>) -> Vec<&'a StateOption> {
    let Some(current) = from else {
        return field.states.iter().collect();
    };
    let Some(state) = field.states.iter().find(|state| state.name == current) else {
        return field.states.iter().collect();
    };
    field
        .states
        .iter()
        .filter(|candidate| {
            candidate.name == current
                || state.transitions.iter().any(|transition| {
                    transition.next_state.as_deref() == Some(candidate.name.as_str())
                })
        })
        .collect()
}

/// The value the chosen state records, where it is reachable from the one the
/// form is in.
pub(crate) fn admit(
    field: &StateField,
    from: Option<&str>,
    chosen: &str,
) -> Result<Datum, Refusal> {
    if chosen.is_empty() {
        return Err(Refusal::Empty);
    }
    if !reachable(field, from)
        .iter()
        .any(|state| state.name == chosen)
    {
        return Err(Refusal::NotEnumerated);
    }
    // NOTE: no specification governs this: our own design, and #149 carries
    // the adjudication of what a DV_STATE value document should hold.
    Ok(Datum::Text(chosen.to_owned()))
}

/// The state in the slot, as the control shows it.
fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Text(value)) => value,
        _ => String::new(),
    }
}

/// A control over a state of a state machine.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn StateControl(
    /// What the template admits.
    field: StateField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let current = RwSignal::new(shown(&slot));

    let choices = {
        let field = field.clone();
        move || {
            let held = current.get();
            let from = (!held.is_empty()).then_some(held.as_str());
            reachable(&field, from)
                .into_iter()
                .map(|state| {
                    view! { <option value=state.name.clone()>{state.name.clone()}</option> }
                })
                .collect::<Vec<_>>()
        }
    };

    let terminal = {
        let field = field.clone();
        move || {
            let held = current.get();
            field
                .states
                .iter()
                .find(|state| state.name == held)
                .is_some_and(|state| state.is_terminal)
        }
    };

    let on_change = {
        let slot = slot.clone();
        let field = field.clone();
        move |event: leptos::ev::Event| {
            let chosen = event_target_value(&event);
            let held = current.get_untracked();
            let from = (!held.is_empty()).then_some(held.as_str());
            let outcome = admit(&field, from, &chosen);
            if outcome.is_ok() {
                current.set(chosen);
            }
            slot.apply(outcome, refused);
        }
    };

    view! {
        <select
            id=slot.id.clone()
            class=SELECT
            prop:value=move || current.get()
            on:change=on_change
        >
            <option value="">"Not entered"</option>
            {choices}
        </select>
        <Show when=terminal>
            <div class="mt-1">
                <StatusPill tone=Tone::Ok label="The state machine ends here" />
            </div>
        </Show>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{StateField, StateOption, StateTransitionOption};
    use ferrochart_form::values::Datum;

    use super::{admit, reachable};
    use crate::control::admit::Refusal;

    fn machine() -> StateField {
        StateField {
            states: vec![
                StateOption {
                    name: "planned".to_owned(),
                    is_terminal: false,
                    transitions: vec![StateTransitionOption {
                        event: "start".to_owned(),
                        action: None,
                        guard: None,
                        next_state: Some("active".to_owned()),
                    }],
                },
                StateOption {
                    name: "active".to_owned(),
                    is_terminal: false,
                    transitions: vec![StateTransitionOption {
                        event: "finish".to_owned(),
                        action: None,
                        guard: None,
                        next_state: Some("completed".to_owned()),
                    }],
                },
                StateOption {
                    name: "completed".to_owned(),
                    is_terminal: true,
                    transitions: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn every_state_is_offered_while_none_is_chosen() {
        assert_eq!(reachable(&machine(), None).len(), 3);
        assert_eq!(
            admit(&machine(), None, "completed"),
            Ok(Datum::Text("completed".to_owned()))
        );
    }

    #[test]
    fn only_a_transition_out_of_the_current_state_is_offered() {
        let states = machine();
        let names: Vec<&str> = reachable(&states, Some("planned"))
            .into_iter()
            .map(|state| state.name.as_str())
            .collect();
        assert_eq!(names, ["planned", "active"]);
        assert_eq!(
            admit(&machine(), Some("planned"), "active"),
            Ok(Datum::Text("active".to_owned()))
        );
        assert_eq!(
            admit(&machine(), Some("planned"), "completed"),
            Err(Refusal::NotEnumerated)
        );
    }

    #[test]
    fn a_terminal_state_leads_nowhere_but_itself() {
        let states = machine();
        let names: Vec<&str> = reachable(&states, Some("completed"))
            .into_iter()
            .map(|state| state.name.as_str())
            .collect();
        assert_eq!(names, ["completed"]);
        assert_eq!(
            admit(&machine(), Some("completed"), "planned"),
            Err(Refusal::NotEnumerated)
        );
    }

    #[test]
    fn nothing_chosen_is_nothing_entered() {
        assert_eq!(admit(&machine(), None, ""), Err(Refusal::Empty));
    }
}
