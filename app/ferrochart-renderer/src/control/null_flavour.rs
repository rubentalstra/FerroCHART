// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The null-flavour affordance beside a field.
//!
//! openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
//! `ELEMENT` the invariants `Inv_is_null_valid`, `Inv_null_flavour_indicated`
//! and `Inv_null_reason_valid`, which together say a field carries exactly one
//! of a value and a null flavour, and that a reason is legal only beside a
//! flavour. Choosing a flavour therefore clears the value, entering a value
//! clears the flavour, and the reason input is offered only while a flavour is
//! set.

use ferrochart_form::field::NullFlavour;
use ferrochart_form::ids::LocalCode;
use ferrochart_form::values::Entered;
use leptos::prelude::*;

use crate::control::Slot;
use crate::kit::field::{INPUT, LABEL, SELECT};

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

/// The affordance that records a reason instead of a value.
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

    view! {
        <div class="mt-2 grid gap-2 sm:grid-cols-2">
            <div>
                <label class=LABEL for=flavour_id.clone()>
                    "No value, because"
                </label>
                <select
                    id=flavour_id
                    class=SELECT
                    prop:value=move || flavour.get()
                    on:change=on_flavour
                >
                    <option value="">"A value is entered"</option>
                    {choices}
                </select>
            </div>
            <Show when=move || accepts_reason && !flavour.get().is_empty()>
                <div>
                    <label class=LABEL for=reason_id.clone()>
                        "The specific reason"
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

    use super::offers;

    #[test]
    fn the_form_offers_exactly_the_flavours_the_spec_names() {
        let affordance = NullFlavour::offered();
        for code in ["253", "271", "272", "273"] {
            assert!(offers(&affordance, code), "the form drops flavour {code}");
        }
        assert!(!offers(&affordance, "999"));
    }

    #[test]
    fn a_field_with_no_null_mechanism_offers_no_flavour_and_no_reason() {
        let affordance = NullFlavour::absent();
        assert!(!affordance.is_offered);
        assert!(!affordance.accepts_reason);
        assert!(!offers(&affordance, "253"));
    }
}
