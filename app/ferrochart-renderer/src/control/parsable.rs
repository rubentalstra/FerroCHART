// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_PARSABLE`: text in a named formalism.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 9.2.3 gives the class a
//! `value` that "may validly be empty in some syntaxes" and a `formalism`
//! with the invariant `Formalism_valid: not formalism.is_empty`, so the two
//! parts refuse different things.

use ferrochart_form::field::ParsableField;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, text_enumeration_admits};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{INPUT, LABEL, SELECT, TEXTAREA};

/// The value the two parts record, where the constraint admits them.
pub(crate) fn admit(field: &ParsableField, formalism: &str, value: &str) -> Result<Datum, Refusal> {
    if formalism.is_empty() {
        return Err(Refusal::Empty);
    }
    if !text_enumeration_admits(&field.formalism, formalism) {
        return Err(Refusal::NotEnumerated);
    }
    if !text_enumeration_admits(&field.value, value) {
        return Err(Refusal::NotEnumerated);
    }
    Ok(Datum::Parsable {
        value: value.to_owned(),
        formalism: formalism.to_owned(),
    })
}

/// The two parts in the slot, as the inputs show them.
fn shown(slot: &Slot) -> (String, String) {
    match slot.datum() {
        Some(Datum::Parsable { formalism, value }) => (formalism, value),
        _ => (String::new(), String::new()),
    }
}

/// A control over text in a named formalism.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn ParsableControl(
    /// What the template admits.
    field: ParsableField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let held = shown(&slot);
    let formalism = RwSignal::new(held.0);
    let value = RwSignal::new(held.1);
    let formalism_id = slot.part("formalism");

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            slot.apply(
                admit(&field, &formalism.get_untracked(), &value.get_untracked()),
                refused,
            );
        }
    };

    let listed = field.formalism.options_closed && !field.formalism.options.is_empty();
    let choices: Vec<_> = field
        .formalism
        .options
        .iter()
        .map(|option| view! { <option value=option.clone()>{option.clone()}</option> })
        .collect();

    let on_formalism = {
        let record = record.clone();
        move |event: leptos::ev::Event| {
            formalism.set(event_target_value(&event));
            record();
        }
    };
    let on_value = move |event: leptos::ev::Event| {
        value.set(event_target_value(&event));
        record();
    };

    view! {
        <div class="flex flex-col gap-2">
            <div>
                <label class=LABEL for=formalism_id.clone()>
                    "Formalism"
                </label>
                <Show
                    when=move || listed
                    fallback={
                        let formalism_id = formalism_id.clone();
                        let on_formalism = on_formalism.clone();
                        move || {
                            view! {
                                <input
                                    id=formalism_id.clone()
                                    class=INPUT
                                    type="text"
                                    prop:value=move || formalism.get()
                                    on:input=on_formalism.clone()
                                />
                            }
                        }
                    }
                >
                    <select
                        id=formalism_id.clone()
                        class=SELECT
                        prop:value=move || formalism.get()
                        on:change=on_formalism.clone()
                    >
                        <option value="">"Not entered"</option>
                        {choices.clone()}
                    </select>
                </Show>
            </div>
            <div>
                <label class=LABEL for=slot.id.clone()>
                    "Text"
                </label>
                <textarea
                    id=slot.id.clone()
                    class=TEXTAREA
                    prop:value=move || value.get()
                    on:input=on_value
                ></textarea>
            </div>
        </div>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{ParsableField, TextField};
    use ferrochart_form::values::Datum;

    use super::admit;
    use crate::control::admit::Refusal;

    #[test]
    fn an_empty_formalism_is_refused_and_an_empty_text_is_not() {
        let field = ParsableField::default();
        assert_eq!(admit(&field, "", "anything"), Err(Refusal::Empty));
        assert_eq!(
            admit(&field, "text/plain", ""),
            Ok(Datum::Parsable {
                value: String::new(),
                formalism: "text/plain".to_owned(),
            })
        );
    }

    #[test]
    fn a_formalism_outside_a_closed_list_is_refused() {
        let field = ParsableField {
            formalism: TextField {
                patterns: Vec::new(),
                options: vec!["text/plain".to_owned()],
                options_closed: true,
            },
            value: TextField::default(),
        };
        assert_eq!(
            admit(&field, "text/plain", "x"),
            Ok(Datum::Parsable {
                value: "x".to_owned(),
                formalism: "text/plain".to_owned(),
            })
        );
        assert_eq!(
            admit(&field, "application/xml", "x"),
            Err(Refusal::NotEnumerated)
        );
    }
}
