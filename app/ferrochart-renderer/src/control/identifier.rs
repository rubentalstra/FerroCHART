// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_IDENTIFIER`: four parts, of which one is mandatory.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 4.2.4 gives the class
//! `issuer`, `assigner`, `id` and `type`, and the invariant `Id_valid: not
//! id.is_empty`. Each of the four carries its own text constraint, so each
//! input honours its own enumeration.

use ferrochart_form::field::{IdentifierField, TextField};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, pattern_attribute, text_enumeration_admits};
use crate::control::{RefusalNote, Slot};
use crate::kit::field::{INPUT, LABEL};

/// The optional part, or nothing where it was left blank.
fn part(constraint: &TextField, typed: &str) -> Result<Option<String>, Refusal> {
    if typed.is_empty() {
        return Ok(None);
    }
    if text_enumeration_admits(constraint, typed) {
        return Ok(Some(typed.to_owned()));
    }
    Err(Refusal::NotEnumerated)
}

/// The value the four parts record, where the constraint admits them.
pub(crate) fn admit(
    field: &IdentifierField,
    id: &str,
    issuer: &str,
    assigner: &str,
    identifier_type: &str,
) -> Result<Datum, Refusal> {
    if id.is_empty() {
        return Err(Refusal::Empty);
    }
    if !text_enumeration_admits(&field.id, id) {
        return Err(Refusal::NotEnumerated);
    }
    Ok(Datum::Identifier {
        id: id.to_owned(),
        issuer: part(&field.issuer, issuer)?,
        assigner: part(&field.assigner, assigner)?,
        identifier_type: part(&field.identifier_type, identifier_type)?,
    })
}

/// The four parts in the slot, as the inputs show them.
fn shown(slot: &Slot) -> (String, String, String, String) {
    match slot.datum() {
        Some(Datum::Identifier {
            id,
            issuer,
            assigner,
            identifier_type,
        }) => (
            id,
            issuer.unwrap_or_default(),
            assigner.unwrap_or_default(),
            identifier_type.unwrap_or_default(),
        ),
        _ => (String::new(), String::new(), String::new(), String::new()),
    }
}

/// A control over the four parts of an identifier.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn IdentifierControl(
    /// What the template admits.
    field: IdentifierField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let held = shown(&slot);
    let id = RwSignal::new(held.0);
    let issuer = RwSignal::new(held.1);
    let assigner = RwSignal::new(held.2);
    let identifier_type = RwSignal::new(held.3);

    let record = {
        let slot = slot.clone();
        let field = field.clone();
        move || {
            slot.apply(
                admit(
                    &field,
                    &id.get_untracked(),
                    &issuer.get_untracked(),
                    &assigner.get_untracked(),
                    &identifier_type.get_untracked(),
                ),
                refused,
            );
        }
    };

    let boxes = [
        ("Identifier", id, slot.id.clone(), field.id.clone()),
        ("Issuer", issuer, slot.part("issuer"), field.issuer.clone()),
        (
            "Assigner",
            assigner,
            slot.part("assigner"),
            field.assigner.clone(),
        ),
        (
            "Type",
            identifier_type,
            slot.part("type"),
            field.identifier_type.clone(),
        ),
    ]
    .into_iter()
    .map(|(label, draft, id, constraint)| {
        let record = record.clone();
        view! {
            <div>
                <label class=LABEL for=id.clone()>
                    {label}
                </label>
                <input
                    id=id
                    class=INPUT
                    type="text"
                    pattern=pattern_attribute(&constraint.patterns)
                    prop:value=move || draft.get()
                    on:input=move |event| {
                        draft.set(event_target_value(&event));
                        record();
                    }
                />
            </div>
        }
    })
    .collect::<Vec<_>>();

    view! {
        <div class="grid gap-2 sm:grid-cols-2">{boxes}</div>
        <RefusalNote refused=refused />
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::{IdentifierField, TextField};
    use ferrochart_form::values::Datum;

    use super::admit;
    use crate::control::admit::Refusal;

    #[test]
    fn an_identifier_with_no_id_is_refused_by_id_valid() {
        assert_eq!(
            admit(&IdentifierField::default(), "", "NHS", "", ""),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn the_three_optional_parts_are_absent_rather_than_empty() {
        assert_eq!(
            admit(&IdentifierField::default(), "1234", "", "", ""),
            Ok(Datum::Identifier {
                id: "1234".to_owned(),
                issuer: None,
                assigner: None,
                identifier_type: None,
            })
        );
    }

    #[test]
    fn each_part_honours_its_own_closed_list() {
        let field = IdentifierField {
            identifier_type: TextField {
                patterns: Vec::new(),
                options: vec!["MRN".to_owned()],
                options_closed: true,
            },
            ..IdentifierField::default()
        };
        assert_eq!(
            admit(&field, "1234", "", "", "MRN"),
            Ok(Datum::Identifier {
                id: "1234".to_owned(),
                issuer: None,
                assigner: None,
                identifier_type: Some("MRN".to_owned()),
            })
        );
        assert_eq!(
            admit(&field, "1234", "", "", "SSN"),
            Err(Refusal::NotEnumerated)
        );
    }
}
