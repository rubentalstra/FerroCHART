// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The walk that turns a form group into a Reference Model node.
//!
//! Every class here fills the attributes the Reference Model declares 1..1 and
//! a form never shows. Where the value comes from the session or from
//! configuration rather than from the template, the comment says so, because
//! an invented value is a decision a reader is entitled to question.
//!
//! All citations are openEHR RM Release-1.1.0.

use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use openehr_base::containers::NonEmptyVec;
use openehr_rm::v1_2::common::generic::party_proxy::PartyProxy;
use openehr_rm::v1_2::common::generic::party_self::PartySelf;
use openehr_rm::v1_2::composition::content::content_item::ContentItem;
use openehr_rm::v1_2::composition::content::entry::action::Action;
use openehr_rm::v1_2::composition::content::entry::activity::Activity;
use openehr_rm::v1_2::composition::content::entry::admin_entry::AdminEntry;
use openehr_rm::v1_2::composition::content::entry::evaluation::Evaluation;
use openehr_rm::v1_2::composition::content::entry::instruction::Instruction;
use openehr_rm::v1_2::composition::content::entry::ism_transition::IsmTransition;
use openehr_rm::v1_2::composition::content::entry::observation::Observation;
use openehr_rm::v1_2::composition::content::navigation::section::Section;
use openehr_rm::v1_2::composition::event_context::EventContext;
use openehr_rm::v1_2::data_structures::history::event::Event;
use openehr_rm::v1_2::data_structures::history::history::History;
use openehr_rm::v1_2::data_structures::history::point_event::PointEvent;
use openehr_rm::v1_2::data_structures::item_structure::item_structure::ItemStructure;
use openehr_rm::v1_2::data_structures::item_structure::item_tree::ItemTree;
use openehr_rm::v1_2::data_structures::representation::cluster::Cluster;
use openehr_rm::v1_2::data_structures::representation::element::Element;
use openehr_rm::v1_2::data_structures::representation::item::Item;

use crate::build::{
    check_null_flavour, coded, entered_for, field_name_of, name_of, nested_archetyped, node_id_of,
    null_flavour_rubric,
};
use crate::datum;
use crate::envelope::{ACTIVE, Envelope, OPENEHR, Subject};
use crate::error::BuildError;
use ferrochart_form::values::{Entered, FormValues};

/// Everything under a composition group's `content` attribute.
///
/// # Errors
/// [`BuildError`] from any node below.
pub(crate) fn content_of(
    root: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Vec<ContentItem>, BuildError> {
    let mut built = Vec::new();
    for item in &root.items {
        let FormItem::Group(ref group) = *item else {
            continue;
        };
        if attribute_of(group) != "content" {
            continue;
        }
        built.push(content_item(group, values, envelope, language)?);
    }
    Ok(built)
}

/// The `EVENT_CONTEXT` a composition group states, where it states one.
///
/// The template decides whether a context exists: `ehr.html` section 5.2.3.1,
/// "Ultimately, the use of Event context will be controlled by
/// Composition-level archetypes". The invariant that once forbade a context on
/// a persistent composition was removed in RM Release-1.0.4 and must not be
/// reintroduced.
///
/// # Errors
/// [`BuildError`] from any node below.
pub(crate) fn context_of(
    root: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Option<EventContext>, BuildError> {
    let stated = root.items.iter().find_map(|item| match *item {
        FormItem::Group(ref group) if attribute_of(group) == "context" => Some(group),
        _ => None,
    });
    let Some(group) = stated else {
        return Ok(None);
    };
    let Some(setting) = envelope.setting.as_ref() else {
        return Ok(None);
    };
    let other = items_under(group, "other_context", values, envelope, language)?;
    Ok(Some(EventContext {
        // Both 1..1, and no committed template constrains either, so the
        // session supplies them.
        start_time: datum::date_time(&envelope.now),
        end_time: None,
        location: None,
        setting: coded(OPENEHR, &setting.code, &setting.rubric),
        other_context: other.map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        health_care_facility: None,
        participations: None,
    }))
}

/// One `CONTENT_ITEM`: a `SECTION` or an `ENTRY`.
///
/// # Errors
/// [`BuildError::Invariant`] for a class that cannot be a `CONTENT_ITEM`, and
/// anything the nodes below refuse.
pub(crate) fn content_item(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<ContentItem, BuildError> {
    match group.rm_type.as_str() {
        "SECTION" => Ok(ContentItem::Section(section(
            group, values, envelope, language,
        )?)),
        "OBSERVATION" => Ok(ContentItem::Observation(observation(
            group, values, envelope, language,
        )?)),
        "EVALUATION" => Ok(ContentItem::Evaluation(evaluation(
            group, values, envelope, language,
        )?)),
        "ADMIN_ENTRY" => Ok(ContentItem::AdminEntry(admin_entry(
            group, values, envelope, language,
        )?)),
        "INSTRUCTION" => Ok(ContentItem::Instruction(instruction(
            group, values, envelope, language,
        )?)),
        "ACTION" => Ok(ContentItem::Action(action(
            group, values, envelope, language,
        )?)),
        other => Err(BuildError::Invariant {
            key: group.key.clone(),
            invariant: "COMPOSITION.content",
            detail: format!(
                "`{other}` is not a CONTENT_ITEM, so a template rooted at it \
                 describes a fragment rather than a document"
            ),
        }),
    }
}

/// A `SECTION`.
///
/// `ehr.html` section 7.2.1: `items` is 0..1 with
/// `Items_valid: items /= Void implies not items.is_empty`.
fn section(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Section, BuildError> {
    let mut items = Vec::new();
    for item in &group.items {
        let FormItem::Group(ref child) = *item else {
            continue;
        };
        if attribute_of(child) != "items" {
            continue;
        }
        items.push(content_item(child, values, envelope, language)?);
    }
    Ok(Section {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        items: NonEmptyVec::try_from(items).ok(),
    })
}

/// An `OBSERVATION`.
///
/// `ehr.html` section 8.3.4 makes `data: HISTORY` 1..1, and section 8.3.1
/// makes `language`, `encoding` and `subject` 1..1 on every `ENTRY`. None of
/// the four comes from a template.
fn observation(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Observation, BuildError> {
    let data = history_under(group, "data", values, envelope, language)?.ok_or_else(|| {
        BuildError::Invariant {
            key: group.key.clone(),
            invariant: "OBSERVATION.data",
            detail: "an observation must carry a HISTORY, and the form has none".to_owned(),
        }
    })?;
    Ok(Observation {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        language: envelope.language_code(),
        encoding: envelope.encoding_code(),
        other_participations: None,
        workflow_id: None,
        subject: subject(&envelope.subject),
        provider: None,
        protocol: items_under(group, "protocol", values, envelope, language)?
            .map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        guideline_id: None,
        data,
        state: history_under(group, "state", values, envelope, language)?,
    })
}

/// An `EVALUATION`. `ehr.html` section 8.3.5 makes `data: ITEM_STRUCTURE` 1..1.
fn evaluation(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Evaluation, BuildError> {
    let data = items_under(group, "data", values, envelope, language)?.ok_or_else(|| {
        BuildError::Invariant {
            key: group.key.clone(),
            invariant: "EVALUATION.data",
            detail: "an evaluation must carry an ITEM_STRUCTURE, and the form has none".to_owned(),
        }
    })?;
    Ok(Evaluation {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        language: envelope.language_code(),
        encoding: envelope.encoding_code(),
        other_participations: None,
        workflow_id: None,
        subject: subject(&envelope.subject),
        provider: None,
        protocol: items_under(group, "protocol", values, envelope, language)?
            .map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        guideline_id: None,
        data: ItemStructure::ItemTree(Box::new(data)),
    })
}

/// An `ADMIN_ENTRY`. `ehr.html` section 8.3.3 makes `data` 1..1.
fn admin_entry(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<AdminEntry, BuildError> {
    let data = items_under(group, "data", values, envelope, language)?.ok_or_else(|| {
        BuildError::Invariant {
            key: group.key.clone(),
            invariant: "ADMIN_ENTRY.data",
            detail: "an admin entry must carry an ITEM_STRUCTURE, and the form has none".to_owned(),
        }
    })?;
    Ok(AdminEntry {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        language: envelope.language_code(),
        encoding: envelope.encoding_code(),
        other_participations: None,
        workflow_id: None,
        subject: subject(&envelope.subject),
        provider: None,
        data: ItemStructure::ItemTree(Box::new(data)),
    })
}

/// An `INSTRUCTION`.
///
/// `ehr.html` section 8.3.7 makes `narrative: DV_TEXT` 1..1 and `activities`
/// 0..1 with `Activities_valid: activities /= Void implies not
/// activities.is_empty`. No template constrains the narrative, so it is the
/// entry's own label: an instruction with an empty narrative would say
/// nothing about what was instructed.
fn instruction(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Instruction, BuildError> {
    let mut activities = Vec::new();
    for item in &group.items {
        let FormItem::Group(ref child) = *item else {
            continue;
        };
        if attribute_of(child) != "activities" {
            continue;
        }
        if let Some(built) = activity(child, values, envelope, language)? {
            activities.push(built);
        }
    }
    Ok(Instruction {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        language: envelope.language_code(),
        encoding: envelope.encoding_code(),
        other_participations: None,
        workflow_id: None,
        subject: subject(&envelope.subject),
        provider: None,
        protocol: items_under(group, "protocol", values, envelope, language)?
            .map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        guideline_id: None,
        narrative: name_of(group, language),
        expiry_time: None,
        wf_definition: None,
        activities: NonEmptyVec::try_from(activities).ok(),
    })
}

/// One `ACTIVITY`.
///
/// `ehr.html` section 8.3.8 makes `description: ITEM_STRUCTURE` 1..1 and
/// `action_archetype_id: String` 1..1. The latter is a regular expression
/// naming the ACTION archetypes that may fulfil the activity, and it is a
/// template fact rather than an entered one.
fn activity(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Option<Activity>, BuildError> {
    let Some(description) = items_under(group, "description", values, envelope, language)? else {
        return Ok(None);
    };
    Ok(Some(Activity {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        timing: None,
        // The form definition does not carry the constraint, so the widest
        // pattern is written rather than a narrower one FerroCHART invented.
        // No specification governs the choice of default: our own design.
        action_archetype_id: "/.*/".to_owned(),
        description: ItemStructure::ItemTree(Box::new(description)),
    }))
}

/// An `ACTION`.
///
/// `ehr.html` section 8.3.6 makes `time: DV_DATE_TIME` 1..1,
/// `ism_transition: ISM_TRANSITION` 1..1 and `description: ITEM_STRUCTURE`
/// 1..1. The transition's `current_state` is 1..1 and is coded from the
/// openEHR `instruction states` group.
fn action(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Action, BuildError> {
    let description =
        items_under(group, "description", values, envelope, language)?.ok_or_else(|| {
            BuildError::Invariant {
                key: group.key.clone(),
                invariant: "ACTION.description",
                detail: "an action must carry an ITEM_STRUCTURE, and the form has none".to_owned(),
            }
        })?;
    Ok(Action {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        language: envelope.language_code(),
        encoding: envelope.encoding_code(),
        other_participations: None,
        workflow_id: None,
        subject: subject(&envelope.subject),
        provider: None,
        protocol: items_under(group, "protocol", values, envelope, language)?
            .map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        guideline_id: None,
        // 1..1, from the session.
        time: datum::date_time(&envelope.now),
        ism_transition: ism_transition(group, values, language),
        instruction_details: None,
        description: ItemStructure::ItemTree(Box::new(description)),
    })
}

/// The `ISM_TRANSITION` an action carries.
///
/// `current_state` is 1..1 and coded from the openEHR `instruction states`
/// group. Where the form states no transition, the action is recorded as
/// having happened, which is `532|active|` in that group. No specification
/// says what a form with no transition means: our own design.
fn ism_transition(group: &FormGroup, values: &FormValues, language: &LanguageTag) -> IsmTransition {
    let entered = group.items.iter().find_map(|item| match *item {
        FormItem::Group(ref child) if attribute_of(child) == "ism_transition" => Some(child),
        _ => None,
    });
    let state = entered
        .and_then(|node| {
            node.items.iter().find_map(|item| match *item {
                FormItem::Field(ref field)
                    if field
                        .key
                        .terminal()
                        .is_some_and(|step| step.rm_attribute.as_str() == "current_state") =>
                {
                    entered_for(values, &field.key)
                        .first()
                        .and_then(|entry| match **entry {
                            Entered::Value(ferrochart_form::values::Datum::Coded {
                                ref terminology,
                                ref code,
                                ref rubric,
                            }) => Some(coded(terminology, code, rubric)),
                            _ => None,
                        })
                }
                _ => None,
            })
        })
        .unwrap_or_else(|| coded(OPENEHR, ACTIVE, "active"));
    let _ = language;
    IsmTransition {
        current_state: state,
        transition: None,
        careflow_step: None,
        reason: None,
    }
}

/// The `HISTORY` under `attribute`, where the form states one.
///
/// `data_structures.html` section 6.2.1 makes `origin: DV_DATE_TIME` 1..1 and
/// requires `Events_valid: (events /= Void and then not events.is_empty) or
/// summary /= Void`, so a history with neither is illegal and is not built.
fn history_under(
    group: &FormGroup,
    attribute: &str,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Option<History<ItemStructure>>, BuildError> {
    let Some(node) = child_group(group, attribute, "HISTORY") else {
        return Ok(None);
    };
    let mut events = Vec::new();
    for item in &node.items {
        let FormItem::Group(ref child) = *item else {
            continue;
        };
        if attribute_of(child) != "events" {
            continue;
        }
        events.push(event(child, values, envelope, language)?);
    }
    if events.is_empty() {
        return Ok(None);
    }
    Ok(Some(History {
        name: name_of(node, language),
        archetype_node_id: node_id_of(&node.key, node),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(node),
        feeder_audit: None,
        // 1..1, and no template constrains it: the session supplies it.
        origin: datum::date_time(&envelope.now),
        period: None,
        duration: None,
        events: Some(events),
        summary: None,
    }))
}

/// One `EVENT`. `data_structures.html` section 6.2.2 makes `time` 1..1 and
/// `data` 1..1.
fn event(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Event<ItemStructure>, BuildError> {
    let data = items_under(group, "data", values, envelope, language)?.ok_or_else(|| {
        BuildError::Invariant {
            key: group.key.clone(),
            invariant: "EVENT.data",
            detail: "an event must carry an ITEM_STRUCTURE, and the form has none".to_owned(),
        }
    })?;
    // An INTERVAL_EVENT additionally needs `width` and `math_function`, which
    // no form supplies, so a template stating one is refused rather than
    // built with invented bounds.
    if group.rm_type.as_str() == "INTERVAL_EVENT" {
        return Err(BuildError::Invariant {
            key: group.key.clone(),
            invariant: "INTERVAL_EVENT.width",
            detail: "an interval event needs a width and a math function that no form supplies"
                .to_owned(),
        });
    }
    Ok(Event::PointEvent(PointEvent {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        // 1..1, from the session.
        time: datum::date_time(&envelope.now),
        state: items_under(group, "state", values, envelope, language)?
            .map(|tree| ItemStructure::ItemTree(Box::new(tree))),
        data: ItemStructure::ItemTree(Box::new(data)),
    }))
}

/// The `ITEM_TREE` under `attribute`, where the form states content for one.
///
/// Returns `None` where nothing was entered under it: section 4.3.5 permits an
/// empty tree, and an attribute omitted entirely is safer than one carrying an
/// empty list.
fn items_under(
    group: &FormGroup,
    attribute: &str,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Option<ItemTree>, BuildError> {
    let Some(node) = child_group_any(group, attribute) else {
        return Ok(None);
    };
    let items = items_of(node, values, envelope, language)?;
    if items.is_empty() {
        return Ok(None);
    }
    Ok(Some(ItemTree {
        name: name_of(node, language),
        archetype_node_id: node_id_of(&node.key, node),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(node),
        feeder_audit: None,
        items: Some(items),
    }))
}

/// Every `ITEM` a structure group contributes.
fn items_of(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Vec<Item>, BuildError> {
    let mut items = Vec::new();
    for item in &group.items {
        match *item {
            FormItem::Field(ref field) => {
                for element in elements_of(field, values, language)? {
                    items.push(Item::Element(element));
                }
            }
            FormItem::Group(ref child) => match child.rm_type.as_str() {
                "CLUSTER" => {
                    if let Some(cluster) = cluster(child, values, envelope, language)? {
                        items.push(Item::Cluster(cluster));
                    }
                }
                // A nested structure below a structure is folded into its
                // parent's items: `ITEM_TREE.items` takes ITEMs, not another
                // ITEM_STRUCTURE (section 4.3.5).
                _ => items.extend(items_of(child, values, envelope, language)?),
            },
            // `FormItem` is non-exhaustive, so a kind added later reaches
            // here. Dropping it would lose content silently, which is the one
            // thing this crate may not do.
            ref other => {
                return Err(BuildError::Invariant {
                    key: group.key.clone(),
                    invariant: "ITEM_STRUCTURE.items",
                    detail: format!(
                        "the form definition carries {other:?}, which this build does not know"
                    ),
                });
            }
        }
    }
    Ok(items)
}

/// A `CLUSTER`, or `None` where nothing under it was entered.
///
/// `data_structures.html` section 5.2.2 declares `items` 1..1, so a cluster
/// with nothing in it is not built at all.
fn cluster(
    group: &FormGroup,
    values: &FormValues,
    envelope: &Envelope,
    language: &LanguageTag,
) -> Result<Option<Cluster>, BuildError> {
    let items = items_of(group, values, envelope, language)?;
    let Ok(items) = NonEmptyVec::try_from(items) else {
        return Ok(None);
    };
    Ok(Some(Cluster {
        name: name_of(group, language),
        archetype_node_id: node_id_of(&group.key, group),
        uid: None,
        links: None,
        archetype_details: nested_archetyped(group),
        feeder_audit: None,
        items,
    }))
}

/// Every `ELEMENT` a field contributes, one per occurrence entered.
///
/// Section 5.2.3: an element carries exactly one of a value and a null
/// flavour, so each occurrence produces one or the other and never both.
fn elements_of(
    field: &ferrochart_form::field::FormField,
    values: &FormValues,
    language: &LanguageTag,
) -> Result<Vec<Element>, BuildError> {
    let mut built = Vec::new();
    for entered in entered_for(values, &field.key) {
        check_null_flavour(entered)?;
        let (value, null_flavour, null_reason) = match *entered {
            Entered::Value(ref datum) => (
                Some(datum::build(&field.key, field.rm_type.as_str(), datum)?),
                None,
                None,
            ),
            Entered::Null {
                ref code,
                ref reason,
            } => (
                None,
                Some(coded(
                    OPENEHR,
                    code.as_str(),
                    null_flavour_rubric(code.as_str()),
                )),
                // `Inv_null_reason_valid: null_reason /= Void implies
                // is_null()`, which holds here because this arm is the null
                // one.
                reason.as_ref().map(|text| {
                    openehr_rm::v1_2::data_types::text::dv_text::DvText::DvText(
                        openehr_rm::v1_2::data_types::text::dv_text::DvTextData {
                            value: text.clone(),
                            hyperlink: None,
                            formatting: None,
                            mappings: None,
                            language: None,
                            encoding: None,
                        },
                    )
                }),
            ),
        };
        built.push(Element {
            name: field_name_of(field, language),
            archetype_node_id: field
                .key
                .terminal()
                .and_then(|step| step.node_id.as_ref())
                .map_or_else(String::new, |code| code.as_str().to_owned()),
            uid: None,
            links: None,
            // The element's `archetype_node_id` above is its own at-code, so
            // it is not an archetype root and `Archetyped_valid` forbids it
            // details (`common.html` section 3.1.2).
            archetype_details: None,
            feeder_audit: None,
            null_flavour,
            value,
            null_reason,
        });
    }
    Ok(built)
}

/// The `PARTY_PROXY` an entry's subject is.
fn subject(subject: &Subject) -> PartyProxy {
    match *subject {
        Subject::SelfParty => PartyProxy::PartySelf(PartySelf { external_ref: None }),
    }
}

/// The Reference Model attribute a group sits under.
fn attribute_of(group: &FormGroup) -> &str {
    group
        .key
        .terminal()
        .map_or("", |step| step.rm_attribute.as_str())
}

/// The child group under `attribute` whose class is `rm_type`.
fn child_group<'g>(group: &'g FormGroup, attribute: &str, rm_type: &str) -> Option<&'g FormGroup> {
    group.items.iter().find_map(|item| match *item {
        FormItem::Group(ref child)
            if attribute_of(child) == attribute && child.rm_type.as_str() == rm_type =>
        {
            Some(&**child)
        }
        _ => None,
    })
}

/// The child group under `attribute`, whatever its class.
fn child_group_any<'g>(group: &'g FormGroup, attribute: &str) -> Option<&'g FormGroup> {
    group.items.iter().find_map(|item| match *item {
        FormItem::Group(ref child) if attribute_of(child) == attribute => Some(&**child),
        _ => None,
    })
}
