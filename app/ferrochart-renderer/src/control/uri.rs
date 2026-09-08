// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_URI` and `DV_EHR_URI`: a reference, optionally pinned to one scheme.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 10.3.2 gives
//! `DV_EHR_URI` the invariant `Scheme_valid`, which requires the `ehr` scheme;
//! a plain `DV_URI` takes any scheme.

use ferrochart_form::field::UriField;
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, pattern_attribute, uri_scheme};
use crate::control::{RefusalNote, Slot, input_of};
use crate::kit::field::{HINT, INPUT};

/// The value `typed` records, where the constraint admits it.
///
/// `pattern_matched` is the browser's answer about the `pattern` attribute,
/// for the reason [`crate::control::text::admit`] gives.
pub(crate) fn admit(
    field: &UriField,
    typed: &str,
    pattern_matched: bool,
) -> Result<Datum, Refusal> {
    if typed.is_empty() {
        return Err(Refusal::Empty);
    }
    if let Some(required) = field.required_scheme.as_deref()
        && uri_scheme(typed).as_deref() != Some(&required.to_ascii_lowercase())
    {
        return Err(Refusal::WrongScheme);
    }
    if !pattern_matched {
        return Err(Refusal::Malformed);
    }
    Ok(Datum::Uri(typed.to_owned()))
}

/// What is in the slot, as the input shows it.
fn shown(slot: &Slot) -> String {
    match slot.datum() {
        Some(Datum::Uri(value)) => value,
        _ => String::new(),
    }
}

/// A control over a URI.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn UriControl(
    /// What the template admits.
    field: UriField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let pattern = pattern_attribute(&field.patterns);
    let scheme = field.required_scheme.clone();
    let held = {
        let slot = slot.clone();
        move || shown(&slot)
    };
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
            type="url"
            pattern=pattern
            prop:value=held
            on:input=on_input
        />
        {scheme
            .map(|required| {
                view! { <span class=HINT>{format!("The scheme has to be {required}.")}</span> }
            })}
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::UriField;
    use ferrochart_form::values::Datum;

    use super::admit;
    use crate::control::admit::Refusal;

    #[test]
    fn an_ehr_uri_refuses_every_other_scheme() {
        let field = UriField {
            patterns: Vec::new(),
            required_scheme: Some("ehr".to_owned()),
        };
        assert_eq!(
            admit(&field, "ehr://node/path", true),
            Ok(Datum::Uri("ehr://node/path".to_owned()))
        );
        assert_eq!(
            admit(&field, "https://example.invalid", true),
            Err(Refusal::WrongScheme)
        );
    }

    #[test]
    fn a_plain_uri_takes_any_scheme() {
        let field = UriField {
            patterns: Vec::new(),
            required_scheme: None,
        };
        assert_eq!(
            admit(&field, "https://example.invalid", true),
            Ok(Datum::Uri("https://example.invalid".to_owned()))
        );
        assert_eq!(admit(&field, "", true), Err(Refusal::Empty));
    }

    #[test]
    fn a_uri_the_pattern_refuses_never_becomes_a_datum() {
        let field = UriField {
            patterns: vec!["https://.*".to_owned()],
            required_scheme: None,
        };
        assert_eq!(
            admit(&field, "ftp://host/x", false),
            Err(Refusal::Malformed)
        );
    }
}
