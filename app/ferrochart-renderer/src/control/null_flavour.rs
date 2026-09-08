// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The affordance that records a reason instead of a value.
//!
//! openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
//! `ELEMENT` the invariants `Inv_is_null_valid`, `Inv_null_flavour_indicated`
//! and `Inv_null_reason_valid`, which together say a field carries exactly one
//! of a value and a null flavour, and that a reason is legal only beside a
//! flavour.
//!
//! So the screen shows one of them at a time. A field with a value draws its
//! control and a quiet way to say there is no value; a field with a flavour
//! draws the flavour and a way back to entering one. Drawing both at once
//! showed a state the Reference Model forbids, and gave a rare exception the
//! same width and weight as the value a person came to type (issue #190).

use ferrochart_form::field::NullFlavour;
use ferrochart_form::ids::LocalCode;
use ferrochart_form::values::Entered;
use leptos::prelude::*;

use crate::control::Slot;
use crate::kit::field::{BTN_SECONDARY, INPUT, LABEL, SELECT};

/// The flavour and reason the slot holds, where it holds a null.
pub(crate) fn shown(slot: &Slot) -> (String, String) {
    match slot.entered() {
        Some(Entered::Null { code, reason }) => {
            (code.as_str().to_owned(), reason.unwrap_or_default())
        }
        Some(Entered::Value(_)) | None => (String::new(), String::new()),
    }
}

/// Whether the code names one of the flavours the form offers.
pub(crate) fn offers(affordance: &NullFlavour, code: &str) -> bool {
    affordance
        .options
        .iter()
        .any(|option| option.code.code == code)
}

/// The rubric the form prints for `code`, or the code where it offers none.
fn rubric(affordance: &NullFlavour, code: &str) -> String {
    affordance
        .options
        .iter()
        .find(|option| option.code.code == code)
        .map_or_else(|| code.to_owned(), |option| option.rubric.clone())
}

/// The quiet control that starts recording a reason instead of a value.
///
/// It sits at the end of the value row rather than under it, so a field that
/// is being filled normally costs one button and not a second select.
#[component]
pub(crate) fn NoValueButton(
    /// What starting looks like to the caller.
    on_start: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="mt-1.5 shrink-0 whitespace-nowrap text-xs text-ink-muted underline underline-offset-2 hover:text-ink"
            on:click=move |_| on_start.run(())
        >
            "No value"
        </button>
    }
}

/// The flavour a field carries instead of a value, and the way back.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn NullFlavourControl(
    /// What the form offers.
    affordance: NullFlavour,
    /// The field the flavour stands in for.
    at: Slot,
    /// What abandoning looks like to the caller, for the case where nothing
    /// is chosen yet and the reader changes their mind.
    on_cancel: Callback<()>,
) -> impl IntoView {
    let slot = at;
    let held = shown(&slot);
    // A flavour outside what this form offers is not shown as chosen: the
    // select would show a blank and the reader would think nothing is set.
    let flavour = RwSignal::new(if offers(&affordance, &held.0) {
        held.0
    } else {
        String::new()
    });
    let reason = RwSignal::new(held.1);
    let flavour_id = slot.part("null-flavour");
    let reason_id = slot.part("null-reason");
    let accepts_reason = affordance.accepts_reason;

    let record = {
        let slot = slot.clone();
        move || {
            let code = flavour.get_untracked();
            if code.is_empty() {
                slot.clear();
                return;
            }
            let reason = reason.get_untracked();
            slot.set_null(LocalCode::new(code), (!reason.is_empty()).then_some(reason));
        }
    };

    let choices: Vec<_> = affordance
        .options
        .iter()
        .map(|option| {
            view! { <option value=option.code.code.clone()>{option.rubric.clone()}</option> }
        })
        .collect();

    let on_flavour = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            flavour.set(event_target_value(&event));
            record();
        }
    };
    let on_reason = move |event: leptos::ev::Event| {
        reason.set(event_target_value(&event));
        record();
    };
    let back = {
        let slot = slot.clone();
        move |_| {
            slot.clear();
            flavour.set(String::new());
            reason.set(String::new());
            on_cancel.run(());
        }
    };
    let named = {
        let affordance = affordance.clone();
        move || {
            let code = flavour.get();
            if code.is_empty() {
                String::new()
            } else {
                rubric(&affordance, &code)
            }
        }
    };

    view! {
        <div class="flex flex-col gap-2">
            <div class="flex items-center gap-2">
                <label class="sr-only" for=flavour_id.clone()>
                    "No value, because"
                </label>
                <select
                    id=flavour_id
                    class=SELECT
                    prop:value=move || flavour.get()
                    on:change=on_flavour
                >
                    <option value="">"Choose why there is no value"</option>
                    {choices}
                </select>
                <button type="button" class=BTN_SECONDARY on:click=back>
                    "Enter a value"
                </button>
            </div>
            <Show when=move || accepts_reason && !flavour.get().is_empty()>
                <div>
                    <label class=LABEL for=reason_id.clone()>
                        {
                            let named = named.clone();
                            move || format!("Why it is {}", named().to_lowercase())
                        }
                    </label>
                    <input
                        id=reason_id.clone()
                        class=INPUT
                        type="text"
                        prop:value=move || reason.get()
                        on:input=on_reason.clone()
                    />
                </div>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::NullFlavour;

    use super::{offers, rubric};

    #[test]
    fn the_form_offers_exactly_the_flavours_the_spec_names() {
        let affordance = NullFlavour::offered();
        for code in ["253", "271", "272", "273"] {
            assert!(offers(&affordance, code), "{code} is not offered");
        }
    }

    #[test]
    fn a_code_the_form_does_not_offer_is_not_offered() {
        let affordance = NullFlavour::offered();
        assert!(!offers(&affordance, "999"));
        assert!(!offers(&affordance, ""));
    }

    #[test]
    fn a_flavour_is_named_by_its_rubric_and_never_by_its_code() {
        // The audience came to build a form, so `253` is not what the screen
        // says back to them (#178).
        let affordance = NullFlavour::offered();
        let said = rubric(&affordance, "253");
        assert_ne!(said, "253");
        assert!(!said.is_empty());
    }

    #[test]
    fn a_code_with_no_rubric_keeps_its_own_text() {
        // Inventing a rubric for a code this build does not know would be
        // worse than showing the code.
        let affordance = NullFlavour::offered();
        assert_eq!(rubric(&affordance, "999"), "999");
    }
}
