// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `DV_CODED_TEXT` and `CODE_PHRASE`: a selection over a value set.
//!
//! openEHR RM Release-1.1.0 `data_types.html` section 5.2.4 makes a coded text
//! a defining code plus "the rubric of that term, from a terminology service,
//! in the language in which the data were authored", so a control captures
//! both. Where the template enumerates the codes it carries the rubrics too;
//! where it names a set without enumerating it, only a terminology server can
//! say what is in the set, and this control says so rather than guessing.

use ferrochart_form::field::CodedField;
use ferrochart_form::ids::LanguageTag;
use ferrochart_form::value::{BindingStrictness, CodedOption, ValueSet};
use ferrochart_form::values::Datum;
use leptos::prelude::*;

use crate::control::admit::{Refusal, text_enumeration_admits};
use crate::control::{RefusalNote, Slot, localized};
use crate::kit::field::{HINT, INPUT, LABEL, SELECT};
use crate::kit::notice::Notice;
use crate::kit::tone::Tone;

/// What the value set lets a control offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Offer {
    /// The template lists the codes, so the control is a selection over them.
    Listed {
        /// The terminology every listed code is drawn from.
        terminology: String,
        /// The codes, in the order the template lists them.
        options: Vec<CodedOption>,
    },
    /// Any code from one terminology, which the control fixes.
    AnyIn(String),
    /// Any code at all, so the terminology is entered beside the code.
    Any,
    /// A set only a terminology server can expand.
    Unexpanded,
    /// A source this renderer does not know, whose membership it therefore
    /// cannot judge.
    Unknown,
}

/// What the field's value set lets a control offer.
pub(crate) fn offer(field: &CodedField) -> Offer {
    match field.value_set {
        ValueSet::Enumerated(ref set) => Offer::Listed {
            terminology: set.terminology.as_str().to_owned(),
            options: set.options.clone(),
        },
        ValueSet::OpenTerminology { ref terminology } => {
            Offer::AnyIn(terminology.as_str().to_owned())
        }
        ValueSet::Expansion(_) => Offer::Unexpanded,
        ValueSet::Unconstrained => Offer::Any,
        _ => Offer::Unknown,
    }
}

/// What the template says about how hard the binding is, for a reader.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.10 carries the status on
/// `C_PRIMITIVE_OBJECT.constraint_status`; ADL 1.4 has no way to state one, so
/// most fields leave it unstated.
pub(crate) const fn strictness_note(strictness: Option<BindingStrictness>) -> Option<&'static str> {
    match strictness {
        None => None,
        Some(BindingStrictness::Required) => Some("The template states this set as required."),
        Some(BindingStrictness::Extensible) => Some("The template states this set as extensible."),
        Some(BindingStrictness::Preferred) => Some("The template states this set as preferred."),
        Some(BindingStrictness::Example) => Some("The template states this set as an example."),
        Some(_) => Some("The template states a binding status this renderer does not know."),
    }
}

/// The value the parts record, where the value set admits them.
///
/// An enumerated set is the permitted set, whatever binding status the
/// template states beside it, so a code outside it is refused. No
/// specification governs what a renderer does with a code outside a set the
/// template says is only preferred: our own design, and the strict reading is
/// the one that cannot build a COMPOSITION the CDR refuses.
pub(crate) fn admit(
    field: &CodedField,
    language: &LanguageTag,
    terminology: &str,
    code: &str,
    rubric: &str,
) -> Result<Datum, Refusal> {
    if code.is_empty() {
        return Err(Refusal::Empty);
    }
    if let Some(constraint) = field.rubric.as_ref()
        && !text_enumeration_admits(constraint, rubric)
    {
        return Err(Refusal::NotEnumerated);
    }
    match offer(field) {
        Offer::Listed {
            terminology: listed,
            options,
        } => {
            let chosen = options
                .iter()
                .find(|option| option.code.code == code)
                .ok_or(Refusal::NotEnumerated)?;
            Ok(Datum::Coded {
                terminology: listed,
                code: code.to_owned(),
                rubric: localized(&chosen.label, language),
            })
        }
        Offer::AnyIn(fixed) => Ok(Datum::Coded {
            terminology: fixed,
            code: code.to_owned(),
            rubric: rubric.to_owned(),
        }),
        Offer::Any => {
            if terminology.is_empty() {
                return Err(Refusal::Empty);
            }
            Ok(Datum::Coded {
                terminology: terminology.to_owned(),
                code: code.to_owned(),
                rubric: rubric.to_owned(),
            })
        }
        Offer::Unexpanded | Offer::Unknown => Err(Refusal::NothingAdmitted),
    }
}

/// The three parts of what is in the slot, as the inputs show them.
fn shown(slot: &Slot) -> (String, String, String) {
    match slot.datum() {
        Some(Datum::Coded {
            terminology,
            code,
            rubric,
        }) => (terminology, code, rubric),
        _ => (String::new(), String::new(), String::new()),
    }
}

/// A control over a coded value.
#[component]
pub(crate) fn CodedControl(
    /// What the template admits.
    field: CodedField,
    /// Where the value goes.
    at: Slot,
) -> impl IntoView {
    let slot = at;
    let refused = RwSignal::new(None::<Refusal>);
    let note = strictness_note(field.strictness);

    let body = match offer(&field) {
        Offer::Unexpanded => view! {
            <Notice
                tone=Tone::Warn
                title="These codes come from a value set the template does not list."
                detail="A terminology server has to expand it before this field can be filled."
            />
        }
        .into_any(),
        Offer::Unknown => view! {
            <Notice
                tone=Tone::Warn
                title="This renderer does not know where these codes come from."
                detail="It cannot judge what the set holds, so it offers nothing."
            />
        }
        .into_any(),
        Offer::Listed { options, .. } => {
            let language = slot.language.clone();
            let choices: Vec<_> = options
                .iter()
                .map(|option| {
                    let rubric = localized(&option.label, &language);
                    let shown = if rubric.is_empty() {
                        option.code.code.clone()
                    } else {
                        rubric
                    };
                    view! { <option value=option.code.code.clone()>{shown}</option> }
                })
                .collect();
            let held = {
                let slot = slot.clone();
                move || shown(&slot).1
            };
            let on_change = {
                let slot = slot.clone();
                let field = field.clone();
                move |event: leptos::ev::Event| {
                    let code = event_target_value(&event);
                    let language = slot.language.clone();
                    slot.apply(admit(&field, &language, "", &code, ""), refused);
                }
            };
            view! {
                <select id=slot.id.clone() class=SELECT prop:value=held on:change=on_change>
                    <option value="">"Not entered"</option>
                    {choices}
                </select>
            }
            .into_any()
        }
        Offer::AnyIn(terminology) => {
            view! { <FreeCode field=field at=slot.clone() terminology=Some(terminology) refused=refused /> }
                .into_any()
        }
        Offer::Any => {
            view! { <FreeCode field=field at=slot.clone() terminology=None refused=refused /> }
                .into_any()
        }
    };

    view! {
        {body}
        {note.map(|line| view! { <span class=HINT>{line}</span> })}
        <RefusalNote refused=refused />
    }
}

/// A code entered by hand, with the rubric that goes with it.
#[component]
fn FreeCode(
    /// What the template admits.
    field: CodedField,
    /// Where the value goes.
    at: Slot,
    /// The terminology the set fixes, or `None` where it is entered too.
    terminology: Option<String>,
    /// Where a refusal is reported.
    refused: RwSignal<Option<Refusal>>,
) -> impl IntoView {
    let slot = at;
    let terminology_id = slot.part("terminology");
    let rubric_id = slot.part("rubric");
    let fixed = terminology.clone();

    // The three inputs write one datum together, so each one holds its own
    // draft and every keystroke re-records the triple.
    let held = shown(&slot);
    let draft_terminology = RwSignal::new(fixed.clone().unwrap_or(held.0));
    let draft_code = RwSignal::new(held.1);
    let draft_rubric = RwSignal::new(held.2);

    let write = {
        let slot = slot.clone();
        let field = field.clone();
        let fixed = fixed.clone();
        move || {
            let terminology = fixed
                .clone()
                .unwrap_or_else(|| draft_terminology.get_untracked());
            let language = slot.language.clone();
            slot.apply(
                admit(
                    &field,
                    &language,
                    &terminology,
                    &draft_code.get_untracked(),
                    &draft_rubric.get_untracked(),
                ),
                refused,
            );
        }
    };

    view! {
        <div class="grid gap-2 sm:grid-cols-3">
            <div>
                <label class=LABEL for=terminology_id.clone()>
                    "Terminology"
                </label>
                <input
                    id=terminology_id
                    class=INPUT
                    type="text"
                    readonly=fixed.is_some()
                    prop:value=move || draft_terminology.get()
                    on:input={
                        let write = write.clone();
                        move |event| {
                            draft_terminology.set(event_target_value(&event));
                            write();
                        }
                    }
                />
            </div>
            <div>
                <label class=LABEL for=slot.id.clone()>
                    "Code"
                </label>
                <input
                    id=slot.id.clone()
                    class=INPUT
                    type="text"
                    prop:value=move || draft_code.get()
                    on:input={
                        let write = write.clone();
                        move |event| {
                            draft_code.set(event_target_value(&event));
                            write();
                        }
                    }
                />
            </div>
            <div>
                <label class=LABEL for=rubric_id.clone()>
                    "Rubric"
                </label>
                <input
                    id=rubric_id
                    class=INPUT
                    type="text"
                    prop:value=move || draft_rubric.get()
                    on:input=move |event| {
                        draft_rubric.set(event_target_value(&event));
                        write();
                    }
                />
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::CodedField;
    use ferrochart_form::ids::{LanguageTag, TerminologyName, local_terminology};
    use ferrochart_form::text::Localized;
    use ferrochart_form::value::{Code, CodedOption, EnumeratedSet, ExpansionSource, ValueSet};
    use ferrochart_form::values::Datum;

    use super::{Offer, admit, offer};
    use crate::control::admit::Refusal;

    fn english() -> LanguageTag {
        LanguageTag::new("en")
    }

    fn listed() -> CodedField {
        CodedField {
            value_set: ValueSet::Enumerated(EnumeratedSet {
                terminology: local_terminology(),
                options: vec![CodedOption {
                    code: Code::new(local_terminology(), "at0004"),
                    label: Localized::in_language(english(), "Sitting"),
                    description: Localized::empty(),
                }],
                needs_display_lookup: false,
            }),
            strictness: None,
            rubric: None,
        }
    }

    #[test]
    fn a_listed_set_admits_a_listed_code_with_the_template_s_own_rubric() {
        assert_eq!(
            admit(&listed(), &english(), "", "at0004", "whatever"),
            Ok(Datum::Coded {
                terminology: "local".to_owned(),
                code: "at0004".to_owned(),
                rubric: "Sitting".to_owned(),
            })
        );
    }

    #[test]
    fn a_listed_set_refuses_a_code_it_does_not_list() {
        assert_eq!(
            admit(&listed(), &english(), "", "at9999", ""),
            Err(Refusal::NotEnumerated)
        );
    }

    #[test]
    fn a_set_only_a_server_can_expand_admits_nothing_here() {
        let field = CodedField {
            value_set: ValueSet::Expansion(ExpansionSource::ReferenceSet {
                uri: "http://example.invalid/ValueSet/x".to_owned(),
            }),
            strictness: None,
            rubric: None,
        };
        assert_eq!(offer(&field), Offer::Unexpanded);
        assert_eq!(
            admit(&field, &english(), "", "1234", ""),
            Err(Refusal::NothingAdmitted)
        );
    }

    #[test]
    fn an_open_terminology_fixes_the_terminology_and_takes_any_code() {
        let field = CodedField {
            value_set: ValueSet::OpenTerminology {
                terminology: TerminologyName::new("SNOMED-CT"),
            },
            strictness: None,
            rubric: None,
        };
        assert_eq!(
            admit(&field, &english(), "ignored", "271649006", "Systolic"),
            Ok(Datum::Coded {
                terminology: "SNOMED-CT".to_owned(),
                code: "271649006".to_owned(),
                rubric: "Systolic".to_owned(),
            })
        );
    }

    #[test]
    fn an_unconstrained_set_needs_a_terminology_beside_the_code() {
        let field = CodedField {
            value_set: ValueSet::Unconstrained,
            strictness: None,
            rubric: None,
        };
        assert_eq!(admit(&field, &english(), "", "1", ""), Err(Refusal::Empty));
        assert_eq!(
            admit(&field, &english(), "ICD10", "A00", ""),
            Ok(Datum::Coded {
                terminology: "ICD10".to_owned(),
                code: "A00".to_owned(),
                rubric: String::new(),
            })
        );
    }

    #[test]
    fn nothing_chosen_is_nothing_entered() {
        assert_eq!(
            admit(&listed(), &english(), "", "", ""),
            Err(Refusal::Empty)
        );
    }
}
