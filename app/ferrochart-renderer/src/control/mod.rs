// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One control per [`ferrochart_form::field::FieldKind`], and the frame each
//! one is drawn in.
//!
//! The variant names the kind of value and never the widget, so which control
//! a four-member selection gets is a layout decision. No specification
//! governs that: our own design, and the layout overlay may override it.
//!
//! What the specifications do govern is what the control may accept. A form
//! never admits what the template refuses, so every control renders from a
//! pure admission function in its own module, and a value the function
//! refuses never reaches [`crate::state::FormState`].
//!
//! Every control is reachable and operable from the keyboard, and none of
//! them draws a focus ring: one `:focus-visible` rule in `style/tailwind.css`
//! owns the indicator.

pub(crate) mod admit;
pub(crate) mod boolean;
pub(crate) mod choice;
pub(crate) mod coded;
pub(crate) mod count;
pub(crate) mod duration;
pub(crate) mod field;
pub(crate) mod group;
pub(crate) mod identifier;
pub(crate) mod interval;
pub(crate) mod multimedia;
pub(crate) mod null_flavour;
pub(crate) mod ordinal;
pub(crate) mod parsable;
pub(crate) mod prefill;
pub(crate) mod proportion;
pub(crate) mod quantity;
pub(crate) mod repeats;
pub(crate) mod state_machine;
pub(crate) mod temporal;
pub(crate) mod text;
pub(crate) mod undetermined;
pub(crate) mod uri;

use std::hash::{DefaultHasher, Hash, Hasher};

use ferrochart_form::ids::{LanguageTag, LocalCode};
use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;
use ferrochart_form::values::{Datum, Entered};
use leptos::prelude::*;

use crate::control::admit::Refusal;
use crate::state::{FormState, Path};

/// Where one value sits: the field it belongs to, the occurrence of each
/// repeating group above it, and the occurrence of the field itself.
#[derive(Clone, Debug)]
pub(crate) struct Address {
    /// The field the value belongs to.
    pub(crate) key: NodeKey,
    /// Which occurrence of each repeating group above the field.
    pub(crate) path: Path,
    /// Which repeat of the field itself.
    pub(crate) occurrence: usize,
}

impl Address {
    /// The address of one repeat of one field.
    pub(crate) fn new(key: NodeKey, path: Path, occurrence: usize) -> Self {
        Self {
            key,
            path,
            occurrence,
        }
    }
}

/// Where a control's value lands.
///
/// Almost always the form itself. The exception is the end of an interval:
/// two ends make ONE value under one address, so each end writes into a
/// signal the interval control owns and the interval writes the pair. That is
/// what lets an interval draw its ends with the same controls as anything
/// else rather than a second set of its own.
#[derive(Clone, Copy)]
enum Sink {
    /// The form the clinician is filling.
    Form(FormState),
    /// A value another control assembles into one of its own.
    Scratch(RwSignal<Option<Entered>>),
}

/// Everything a control needs to read and write one value.
#[derive(Clone)]
pub(crate) struct Slot {
    /// Where the value lands.
    sink: Sink,
    /// The address it lands at.
    pub(crate) address: Address,
    /// The id a label points at, unique on the page.
    pub(crate) id: String,
    /// The language a rubric is shown in.
    pub(crate) language: LanguageTag,
    /// What the control is called, for a control whose own markup has to
    /// carry an accessible name.
    pub(crate) label: String,
}

impl Slot {
    /// The slot one control writes into.
    pub(crate) fn new(
        state: FormState,
        address: Address,
        language: LanguageTag,
        label: String,
    ) -> Self {
        let id = element_id(&address);
        Self {
            sink: Sink::Form(state),
            address,
            id,
            language,
            label,
        }
    }

    /// A slot whose value another control assembles into one of its own.
    pub(crate) fn scratch(
        &self,
        suffix: &str,
        held: RwSignal<Option<Entered>>,
        label: String,
    ) -> Self {
        Self {
            sink: Sink::Scratch(held),
            address: self.address.clone(),
            id: self.part(suffix),
            language: self.language.clone(),
            label,
        }
    }

    /// The slot of another node at the same occurrence.
    ///
    /// A choice's alternatives are nodes of their own, each with its own key,
    /// so the alternative a clinician fills is written under that key rather
    /// than under the choice's.
    pub(crate) fn sibling(&self, key: NodeKey, label: String) -> Self {
        let address = Address::new(key, self.address.path.clone(), self.address.occurrence);
        let id = element_id(&address);
        Self {
            sink: self.sink,
            address,
            id,
            language: self.language.clone(),
            label,
        }
    }

    /// The same slot under a suffixed id, for a control that draws more than
    /// one input.
    pub(crate) fn part(&self, suffix: &str) -> String {
        format!("{}-{suffix}", self.id)
    }

    /// What was entered here.
    pub(crate) fn entered(&self) -> Option<Entered> {
        match self.sink {
            Sink::Form(state) => state.get(
                &self.address.key,
                &self.address.path,
                self.address.occurrence,
            ),
            Sink::Scratch(held) => held.get(),
        }
    }

    /// What was entered here, where it is a value rather than a null flavour.
    pub(crate) fn datum(&self) -> Option<Datum> {
        match self.entered() {
            Some(Entered::Value(datum)) => Some(datum),
            Some(Entered::Null { .. }) | None => None,
        }
    }

    /// Records a value, which replaces any null flavour that was here.
    pub(crate) fn set(&self, datum: Datum) {
        match self.sink {
            Sink::Form(state) => state.set(
                &self.address.key,
                &self.address.path,
                self.address.occurrence,
                datum,
            ),
            Sink::Scratch(held) => held.set(Some(Entered::Value(datum))),
        }
    }

    /// Records a null flavour, which replaces any value that was here.
    pub(crate) fn set_null(&self, code: LocalCode, reason: Option<String>) {
        match self.sink {
            Sink::Form(state) => state.set_null(
                &self.address.key,
                &self.address.path,
                self.address.occurrence,
                code,
                reason,
            ),
            Sink::Scratch(held) => held.set(Some(Entered::Null { code, reason })),
        }
    }

    /// Forgets whatever was entered here.
    pub(crate) fn clear(&self) {
        match self.sink {
            Sink::Form(state) => {
                state.clear(
                    &self.address.key,
                    &self.address.path,
                    self.address.occurrence,
                );
            }
            Sink::Scratch(held) => held.set(None),
        }
    }

    /// Records what the admission function returned, and reports a refusal.
    ///
    /// A refused value never reaches the document: the slot is cleared, so a
    /// form can never submit something a control already judged.
    pub(crate) fn apply(
        &self,
        outcome: Result<Datum, Refusal>,
        refused: RwSignal<Option<Refusal>>,
    ) {
        match outcome {
            Ok(datum) => {
                refused.set(None);
                self.set(datum);
            }
            Err(Refusal::Empty) => {
                refused.set(None);
                self.clear();
            }
            Err(other) => {
                refused.set(Some(other));
                self.clear();
            }
        }
    }
}

/// The id one control's label points at.
///
/// A form can hold two fields of the same archetype node under two repeats of
/// one group, so the id is hashed from the whole address rather than from the
/// key alone.
fn element_id(address: &Address) -> String {
    let mut hasher = DefaultHasher::new();
    address.key.hash(&mut hasher);
    address.path.as_ref().hash(&mut hasher);
    address.occurrence.hash(&mut hasher);
    format!("field-{:016x}", hasher.finish())
}

/// The text in the reader's language, or the first the template states.
///
/// A template states its labels per language and a renderer falls back to the
/// one the template was authored in
/// ([`ferrochart_form::definition::FormDefinition::default_language`]). Where
/// even that is absent, the first language in tag order is better than an
/// empty label.
pub(crate) fn localized(text: &Localized, language: &LanguageTag) -> String {
    crate::label::text(text, language)
        .unwrap_or_default()
        .to_owned()
}

/// The input an event came from.
///
/// A control reads the element rather than only its value, because the
/// browser's own constraint validation is what judges a `pattern`, and
/// `check_validity` lives on the element.
pub(crate) fn input_of(event: &leptos::ev::Event) -> Option<web_sys::HtmlInputElement> {
    web_sys::wasm_bindgen::JsCast::dyn_into(event.target()?).ok()
}

/// The block a control draws when it refuses what was entered.
///
/// `role="alert"` rather than `role="status"`, because a refusal is about
/// what the reader just did and waiting for them to be idle would be too
/// late.
#[component]
pub(crate) fn RefusalNote(
    /// The refusal, or nothing while the value is admitted.
    refused: RwSignal<Option<Refusal>>,
) -> impl IntoView {
    view! {
        <p role="alert" class="mt-1 text-xs text-danger">
            {move || refused.get().map(Refusal::message)}
        </p>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{LanguageTag, RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};
    use ferrochart_form::text::Localized;

    use super::{Address, element_id, localized};
    use crate::state::root_path;

    fn key(attribute: &str) -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    #[test]
    fn two_repeats_of_one_field_get_two_ids() {
        let first = element_id(&Address::new(key("items"), root_path(), 0));
        let second = element_id(&Address::new(key("items"), root_path(), 1));
        assert_ne!(first, second, "two inputs cannot share one id");
        assert!(first.starts_with("field-"), "{first}");
    }

    #[test]
    fn one_address_always_gets_the_same_id() {
        let once = element_id(&Address::new(key("items"), root_path(), 0));
        let again = element_id(&Address::new(key("items"), root_path(), 0));
        assert_eq!(once, again, "a redraw cannot move a label's target");
    }

    #[test]
    fn a_label_falls_back_to_the_language_the_template_states_it_in() {
        let text = Localized::in_language(LanguageTag::new("en"), "Systolic");
        assert_eq!(localized(&text, &LanguageTag::new("en")), "Systolic");
        assert_eq!(
            localized(&text, &LanguageTag::new("nl")),
            "Systolic",
            "an untranslated label is better than an empty one"
        );
        assert_eq!(localized(&Localized::empty(), &LanguageTag::new("en")), "");
    }
}
