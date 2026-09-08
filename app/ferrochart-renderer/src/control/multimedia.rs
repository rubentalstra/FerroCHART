// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_MULTIMEDIA`: a file, given by reference.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 9.2.2. The template's
//! media types are the reference's filter, and its `uri` constraint is the
//! constraint on the reference itself.
//!
//! Only the reference is collected. The entered-value document carries no
//! shape for the media type, the size, or inline data, so a control that
//! offered a file upload would collect something it cannot record.

use ferrochart_form::field::MultimediaField;
use ferrochart_form::value::ValueSet;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, pattern_attribute, text_enumeration_admits};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{HINT, INPUT};

/// The media types the template accepts, as one line a reader can read.
///
/// Empty where the template names none, which accepts any media type.
pub(crate) fn accepted(field: &MultimediaField) -> Vec<String> {
    match field.media_types {
        ValueSet::Enumerated(ref set) => set
            .options
            .iter()
            .map(|option| option.code.code.clone())
            .collect(),
        _ => Vec::new(),
    }
}

/// The value the reference records, where the constraint admits it.
///
/// `pattern_matched` is the browser's answer about the `pattern` attribute,
/// for the reason [`crate::control::text::admit`] gives.
pub(crate) fn admit(
    field: &MultimediaField,
    typed: &str,
    pattern_matched: bool,
) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    if let Some(constraint) = field.uri.as_ref()
        && !text_enumeration_admits(constraint, typed)
    {
        return Err(Refusal::NotEnumerated);
    }
    if !pattern_matched {
        return Err(Refusal::Malformed);
    }
    // NOTE: no specification governs this: our own design, and #149 carries
    // the adjudication of what a DV_MULTIMEDIA value document should hold.
    Ok(Datum::Uri(typed.to_owned()))
}

/// The reference in the slot, as the input shows it.
fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Uri(value)) => value,
        _ => String::new(),
    }
}

/// A control over a file given by reference.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn MultimediaControl(
    /// What the template admits.
    field: MultimediaField,
    /// Where the reference goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let types = accepted(&field);
    let pattern = field
        .uri
        .as_ref()
        .and_then(|constraint| pattern_attribute(&constraint.patterns));
    let held = {
        let slot = slot.clone();
        move || shown(&slot)
    };
    let on_input = {
        let slot = slot.clone();
        let field = field.clone();
        move |event: leptos::ev::Event| {
            let Some(input) = crate::control::input_of(&event) else {
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
            type="url"
            pattern=pattern
            placeholder="https://example.invalid/scan.png"
            prop:value=held
            on:input=on_input
        />
        <Show when={
            let types = types.clone();
            move || !types.is_empty()
        }>
            <span class=HINT>{format!("The template accepts {}.", types.join(", "))}</span>
        </Show>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{MultimediaField, TextField};
    use ferrochart_form::ids::TerminologyName;
    use ferrochart_form::text::Localized;
    use ferrochart_form::value::{Code, CodedOption, EnumeratedSet, ValueSet};
    use ferrochart_form::values::Datum;

    use super::{accepted, admit};
    use crate::control::admit::Refusal;

    fn media(types: &[&str]) -> MultimediaField {
        MultimediaField {
            media_types: ValueSet::Enumerated(EnumeratedSet {
                terminology: TerminologyName::new("IANA_media-types"),
                options: types
                    .iter()
                    .map(|kind| CodedOption {
                        code: Code::new(TerminologyName::new("IANA_media-types"), *kind),
                        label: Localized::empty(),
                        description: Localized::empty(),
                    })
                    .collect(),
                needs_display_lookup: false,
            }),
            uri: None,
        }
    }

    #[test]
    fn the_accepted_media_types_come_from_the_enumerated_set() {
        assert_eq!(accepted(&media(&["image/png"])), ["image/png"]);
        assert_eq!(
            accepted(&MultimediaField {
                media_types: ValueSet::Unconstrained,
                uri: None,
            }),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_reference_is_recorded_and_an_empty_one_is_not() {
        let field = media(&["image/png"]);
        assert_eq!(
            admit(&field, "https://example.invalid/scan.png", true),
            Ok(Datum::Uri("https://example.invalid/scan.png".to_owned()))
        );
        assert_eq!(admit(&field, "", true), Err(Refusal::Empty));
    }

    #[test]
    fn a_reference_outside_the_uri_constraint_is_refused() {
        let field = MultimediaField {
            media_types: ValueSet::Unconstrained,
            uri: Some(TextField {
                patterns: Vec::new(),
                options: vec!["https://example.invalid/scan.png".to_owned()],
                options_closed: true,
            }),
        };
        assert_eq!(
            admit(&field, "https://elsewhere.invalid/x.png", true),
            Err(Refusal::NotEnumerated)
        );
        assert_eq!(
            admit(&field, "https://example.invalid/scan.png", false),
            Err(Refusal::Malformed)
        );
    }
}
