// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_TEXT`: free text, a mask, or a pick list.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.3 defines
//! `C_STRING.list_open` as "True if the list is being used to specify the
//! constraint but is not considered exhaustive", so a closed list is the whole
//! permitted set and an open one is a suggestion. The patterns are regular
//! expressions with ADL 2's delimiters already stripped.

use ferrochart_form::field::TextField;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, pattern_attribute, text_enumeration_admits};
use crate::control::{RefusalNote, Slot, input_of};
use crate::kit::field::{INPUT, SELECT};

/// Which control the constraint calls for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Shape {
    /// A closed list: a selection over the whole permitted set.
    Selection,
    /// An open list: typing, with the listed values offered.
    Suggested,
    /// No list at all: typing.
    Free,
}

/// The control the constraint calls for.
///
/// No specification governs the choice of widget: our own design. What the
/// specification decides is the SET, and a closed set is what makes a
/// selection honest rather than a hint.
pub(crate) fn shape(field: &TextField) -> Shape {
    match (field.options.is_empty(), field.options_closed) {
        (true, _) => Shape::Free,
        (false, true) => Shape::Selection,
        (false, false) => Shape::Suggested,
    }
}

/// The value `typed` records, where the constraint admits it.
///
/// `pattern_matched` is what the browser's own constraint validation says
/// about the `pattern` attribute this module puts on the input. The HTML
/// Standard compiles that attribute as a JavaScript regular expression, and
/// no openEHR specification states the dialect an ADL pattern is written in,
/// so the browser is where a pattern is judged and the answer is passed in.
pub(crate) fn admit(
    field: &TextField,
    typed: &str,
    pattern_matched: bool,
) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    if !text_enumeration_admits(field, typed) {
        return Err(Refusal::NotEnumerated);
    }
    if !pattern_matched {
        return Err(Refusal::Malformed);
    }
    Ok(Datum::Text(typed.to_owned()))
}

/// What is in the slot, as the input shows it.
pub(crate) fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Text(value)) => value,
        _ => String::new(),
    }
}

/// A control over `DV_TEXT.value`.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn TextControl(
    /// What the template admits.
    field: TextField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let pattern = pattern_attribute(&field.patterns);
    let list_id = slot.part("options");
    let held = {
        let slot = slot.clone();
        move || shown(&slot)
    };

    let control = match shape(&field) {
        Shape::Selection => {
            let options: Vec<_> = field
                .options
                .iter()
                .map(|option| view! { <option value=option.clone()>{option.clone()}</option> })
                .collect();
            let on_change = {
                let slot = slot.clone();
                let field = field.clone();
                move |event: leptos::ev::Event| {
                    let typed = event_target_value(&event);
                    slot.apply(admit(&field, &typed, true), refused);
                }
            };
            view! {
                <select id=slot.id.clone() class=SELECT prop:value=held on:change=on_change>
                    <option value="">"Not entered"</option>
                    {options}
                </select>
            }
            .into_any()
        }
        Shape::Suggested | Shape::Free => {
            let suggestions: Vec<_> = field
                .options
                .iter()
                .map(|option| view! { <option value=option.clone()></option> })
                .collect();
            let has_suggestions = !field.options.is_empty();
            let on_input = {
                let slot = slot.clone();
                let field = field.clone();
                move |event: leptos::ev::Event| {
                    let Some(input) = input_of(&event) else {
                        return;
                    };
                    slot.apply(
                        admit(&field, &input.value(), input.check_validity()),
                        refused,
                    );
                }
            };
            view! {
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type="text"
                    list=has_suggestions.then(|| list_id.clone())
                    pattern=pattern.clone()
                    prop:value=held
                    on:input=on_input
                />
                <Show when=move || has_suggestions>
                    <datalist id=list_id.clone()>{suggestions.clone()}</datalist>
                </Show>
            }
            .into_any()
        }
    };

    view! {
        {control}
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::TextField;
    use ferrochart_form::values::Datum;

    use super::{Shape, admit, shape};
    use crate::control::admit::Refusal;

    fn listed(options: &[&str], closed: bool) -> TextField {
        TextField {
            patterns: Vec::new(),
            options: options.iter().map(|option| (*option).to_owned()).collect(),
            options_closed: closed,
        }
    }

    #[test]
    fn a_closed_list_refuses_a_value_outside_it() {
        let field = listed(&["sitting", "standing"], true);
        assert_eq!(shape(&field), Shape::Selection);
        assert_eq!(
            admit(&field, "sitting", true),
            Ok(Datum::Text("sitting".to_owned()))
        );
        assert_eq!(admit(&field, "lying", true), Err(Refusal::NotEnumerated));
    }

    #[test]
    fn an_open_list_admits_a_value_outside_it() {
        let field = listed(&["sitting"], false);
        assert_eq!(shape(&field), Shape::Suggested);
        assert_eq!(
            admit(&field, "lying", true),
            Ok(Datum::Text("lying".to_owned()))
        );
    }

    #[test]
    fn a_constraint_with_no_list_takes_any_text() {
        let field = listed(&[], true);
        assert_eq!(shape(&field), Shape::Free);
        assert_eq!(
            admit(&field, "anything", true),
            Ok(Datum::Text("anything".to_owned()))
        );
    }

    #[test]
    fn a_value_the_pattern_refuses_never_becomes_a_datum() {
        let field = TextField {
            patterns: vec!["[0-9]{4}".to_owned()],
            options: Vec::new(),
            options_closed: false,
        };
        assert_eq!(
            admit(&field, "2026", true),
            Ok(Datum::Text("2026".to_owned()))
        );
        assert_eq!(admit(&field, "no", false), Err(Refusal::Malformed));
    }

    #[test]
    fn nothing_typed_is_nothing_entered_rather_than_a_refusal() {
        assert_eq!(admit(&listed(&[], false), "", true), Err(Refusal::Empty));
    }
}
