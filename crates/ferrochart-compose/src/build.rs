// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Form values become a COMPOSITION.
//!
//! The walk is over the form definition's own tree rather than over its keys,
//! because the tree is already the Reference Model tree: a group is a node,
//! its `rm_type` is the class, and the terminal key step's `rm_attribute` is
//! the attribute it sits under. Every group carries the pinned name and the
//! node id the data has to repeat.
//!
//! All citations are openEHR RM Release-1.1.0.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::FormField;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use openehr_base::containers::NonEmptyVec;
use openehr_base::v1_3::base_types::identification::archetype_id::ArchetypeId;
use openehr_base::v1_3::base_types::identification::template_id::TemplateId;
use openehr_rm::v1_2::common::archetyped::archetyped::Archetyped;
use openehr_rm::v1_2::common::generic::party_identified::{PartyIdentified, PartyIdentifiedData};
use openehr_rm::v1_2::common::generic::party_proxy::PartyProxy;
use openehr_rm::v1_2::common::generic::party_self::PartySelf;
use openehr_rm::v1_2::composition::composition::Composition;
use openehr_rm::v1_2::composition::event_context::EventContext;
use openehr_rm::v1_2::data_types::text::dv_coded_text::DvCodedText;
use openehr_rm::v1_2::data_types::text::dv_text::{DvText, DvTextData};

use crate::datum;
use crate::envelope::code_phrase;
use crate::error::BuildError;
use crate::tree;
use ferrochart_form::envelope::{Composer, Envelope, OPENEHR, RM_VERSION};
use ferrochart_form::values::{Datum, Entered, FormValues};

/// The four codes the openEHR `null flavours` group carries.
///
/// `Inv_null_flavour_valid` tests membership of the group rather than a list
/// in the prose, and `data_structures.html` section 4.1 names these four.
pub const NULL_FLAVOURS: [(&str, &str); 4] = [
    ("253", "unknown"),
    ("271", "no information"),
    ("272", "masked"),
    ("273", "not applicable"),
];

/// The four codes the openEHR `composition category` group carries.
///
/// The Reference Model prose names three and then admits "any other code
/// defined in the openEHR terminology group"; TERM Release-3.0.0 carries a
/// fourth, `815`. A list of three would refuse a document the specification
/// permits.
pub const CATEGORIES: [(&str, &str); 4] = [
    ("431", "persistent"),
    ("433", "event"),
    ("451", "episodic"),
    ("815", "report"),
];

/// Builds the COMPOSITION `values` describe against `definition`.
///
/// `envelope` supplies everything the Reference Model requires and a form
/// never shows. `uid` is deliberately not set: `LOCATABLE.uid` is 0..1, the
/// client cannot know the version identity before the commit, and the three
/// specifications that discuss what to put in it disagree.
///
/// # Errors
/// [`BuildError`] naming the field and, where a Reference Model invariant is
/// what refused it, the invariant.
pub fn composition(
    definition: &FormDefinition,
    values: &FormValues,
    envelope: &Envelope,
) -> Result<Composition, BuildError> {
    let language = &definition.default_language;
    let root = &definition.root;
    // Where the template constrains `category`, the form carries a field for
    // it and what was entered there wins: the envelope's value is the
    // configured default for a template that says nothing. All 10 of the
    // COMPOSITION-rooted templates in the committed pack constrain it.
    let category = match entered_category(root, values) {
        Some((code, rubric)) => checked_category(&code, &rubric)?,
        None => category_of(envelope)?,
    };

    // A template rooted at COMPOSITION carries its own envelope nodes, so the
    // root group is the composition. 113 of the 123 committed templates root
    // at an ENTRY or a SECTION instead, and the envelope is built around them.
    let rooted_at_composition = root.rm_type.as_str() == "COMPOSITION";

    // The root of a form is one node, so the walk starts under no repeating
    // group at all and the path grows only as it descends into one.
    let root_path: &[usize] = &[];

    let content = if rooted_at_composition {
        tree::content_of(root, values, root_path, envelope, language)?
    } else {
        vec![tree::content_item(
            root, values, root_path, envelope, language,
        )?]
    };

    let stated_context = if rooted_at_composition {
        tree::context_of(root, values, root_path, envelope, language)?
    } else {
        event_context(envelope)
    };

    // A template rooted at COMPOSITION is its own archetype root. One rooted
    // below it is wrapped in a document FerroCHART supplies, and that document
    // needs an archetype of its own.
    let archetype = if rooted_at_composition {
        root.archetype_id
            .as_ref()
            .map_or_else(String::new, |id| id.as_str().to_owned())
    } else {
        envelope
            .composition_archetype
            .clone()
            .ok_or_else(|| BuildError::Invariant {
                key: root.key.clone(),
                invariant: "COMPOSITION.Is_archetype_root",
                detail: format!(
                    "this template roots at {}, so the composition around it needs an \
                     archetype the configuration has not named",
                    root.rm_type.as_str()
                ),
            })?
    };

    Ok(Composition {
        name: name_of(root, language),
        archetype_node_id: archetype.clone(),
        uid: None,
        links: None,
        archetype_details: Some(archetyped(&archetype, definition)),
        feeder_audit: None,
        language: crate::envelope::language_code(envelope),
        territory: crate::envelope::territory_code(envelope),
        category,
        context: stated_context,
        composer: composer(&envelope.composer),
        // `Content_valid: content /= Void implies not content.is_empty`. An
        // empty list is illegal where an absent attribute is fine, and
        // `ehr.html` section 5.3 says a composition with no content "makes
        // sense" in at least two cases.
        content: NonEmptyVec::try_from(content).ok(),
    })
}

/// The `EVENT_CONTEXT` an envelope describes.
///
/// `start_time` and `setting` are both 1..1 (`ehr.html` section 5.4.2), and no
/// committed template constrains either, so both come from the session.
/// Returns `None` where no setting was supplied, because a context without one
/// would violate `Setting_valid`.
fn event_context(envelope: &Envelope) -> Option<EventContext> {
    let setting = envelope.setting.as_ref()?;
    Some(EventContext {
        start_time: datum::date_time(&envelope.now),
        end_time: None,
        location: None,
        setting: coded(OPENEHR, &setting.code, &setting.rubric),
        other_context: None,
        health_care_facility: None,
        participations: None,
    })
}

/// The `category` an envelope names, checked against the openEHR group.
fn category_of(envelope: &Envelope) -> Result<DvCodedText, BuildError> {
    checked_category(&envelope.category, &envelope.category_rubric)
}

/// A `category` checked against the openEHR `composition category` group.
fn checked_category(code: &str, rubric: &str) -> Result<DvCodedText, BuildError> {
    if !CATEGORIES.iter().any(|&(known, _)| known == code) {
        return Err(BuildError::UnknownCategory {
            code: code.to_owned(),
        });
    }
    Ok(coded(OPENEHR, code, rubric))
}

/// The category the form carries, where it carries one and it was entered.
pub(crate) fn category_field(root: &FormGroup) -> Option<&FormField> {
    root.items.iter().find_map(|item| {
        let FormItem::Field(ref field) = *item else {
            return None;
        };
        field
            .key
            .terminal()
            .is_some_and(|step| step.rm_attribute.as_str() == "category")
            .then_some(&**field)
    })
}

/// The code and rubric entered against the form's `category` field.
fn entered_category(root: &FormGroup, values: &FormValues) -> Option<(String, String)> {
    let field = category_field(root)?;
    match *values.get(&field.key)? {
        Entered::Value(Datum::Coded {
            ref code,
            ref rubric,
            ..
        }) => Some((code.clone(), rubric.clone())),
        _ => None,
    }
}

/// The `composer`, which is always sent.
fn composer(composer: &Composer) -> PartyProxy {
    match *composer {
        Composer::SelfParty => PartyProxy::PartySelf(PartySelf { external_ref: None }),
        Composer::Identified { ref name } => {
            PartyProxy::PartyIdentified(PartyIdentified::PartyIdentified(PartyIdentifiedData {
                external_ref: None,
                name: Some(name.clone()),
                identifiers: None,
            }))
        }
    }
}

/// The `ARCHETYPED` a node below the document root carries, where it is an
/// archetype root itself.
///
/// `common.html` section 3.1.2: `Archetyped_valid: is_archetype_root xor
/// archetype_details = Void`. Every node the template composed from an
/// archetype of its own is an archetype root, so it carries the details, and
/// `ehr.html` section 8.3.1 makes that explicit for an ENTRY with
/// `Is_archetype_root: is_archetype_root`. A node identified by its own
/// at-code is not a root and carries nothing.
///
/// The template id is absent here. `common.html` section 3.2.3: "Normally, a
/// template would only be used at the top of a top-level structure", so it is
/// written once, at the composition root, by [`archetyped`].
pub(crate) fn nested_archetyped(group: &FormGroup) -> Option<Archetyped> {
    let archetype = group.archetype_id.as_ref()?;
    Some(Archetyped {
        archetype_id: ArchetypeId {
            value: archetype.as_str().to_owned(),
        },
        template_id: None,
        rm_version: RM_VERSION.to_owned(),
    })
}

/// The `ARCHETYPED` the document root carries.
///
/// `Archetyped_valid: is_archetype_root xor archetype_details = Void`, and the
/// root of a template is always an archetype root.
fn archetyped(archetype: &str, definition: &FormDefinition) -> Archetyped {
    Archetyped {
        archetype_id: ArchetypeId {
            value: archetype.to_owned(),
        },
        // `common.html` section 3.2.3: "Normally, a template would only be
        // used at the top of a top-level structure", so the template id is
        // written at the composition root and nowhere below it.
        template_id: Some(TemplateId {
            value: definition.template_id.as_str().to_owned(),
        }),
        rm_version: RM_VERSION.to_owned(),
    }
}

/// A `DV_CODED_TEXT` whose value is the rubric of its code.
pub(crate) fn coded(terminology: &str, code: &str, rubric: &str) -> DvCodedText {
    DvCodedText {
        value: rubric.to_owned(),
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
        defining_code: code_phrase(terminology, code),
    }
}

/// The `name` a group's data node carries.
///
/// `common.html` section 3.1.2: "The default value for name should be assumed
/// to be the text value in the local language for the `archetype_node_id`
/// code on the node in question, unless explicitly set otherwise." A pinned
/// name is that explicit setting, and it is what makes the overlay key match
/// on both sides, so it wins where the template states one.
pub(crate) fn name_of(group: &FormGroup, language: &LanguageTag) -> DvText {
    let pinned = group
        .key
        .terminal()
        .and_then(|step| step.pinned_name.as_deref());
    let text = pinned
        .map(str::to_owned)
        .or_else(|| group.label.get(language).map(str::to_owned))
        .unwrap_or_default();
    DvText::DvText(DvTextData {
        value: text,
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
    })
}

/// The `name` a field's element carries.
pub(crate) fn field_name_of(field: &FormField, language: &LanguageTag) -> DvText {
    let pinned = field
        .key
        .terminal()
        .and_then(|step| step.pinned_name.as_deref());
    let text = pinned
        .map(str::to_owned)
        .or_else(|| field.label.get(language).map(str::to_owned))
        .unwrap_or_default();
    DvText::DvText(DvTextData {
        value: text,
        hyperlink: None,
        formatting: None,
        mappings: None,
        language: None,
        encoding: None,
    })
}

/// The `archetype_node_id` a data node carries.
///
/// `common.html` section 3.2.2: at an archetype root the value is "the
/// stringified form of the `archetype_id`", and at a non-root node it is the
/// node's own code. The value is never pattern-matched as an at-code: ADL 2
/// uses `id`-codes and a root uses neither.
pub(crate) fn node_id_of(key: &NodeKey, group: &FormGroup) -> String {
    if let Some(archetype) = group.archetype_id.as_ref() {
        return archetype.as_str().to_owned();
    }
    key.terminal()
        .and_then(|step| step.node_id.as_ref())
        .map_or_else(String::new, |code| code.as_str().to_owned())
}

/// Whether `entered` is a null flavour the openEHR terminology defines.
///
/// # Errors
/// [`BuildError::UnknownNullFlavour`] for a code outside the group.
pub(crate) fn check_null_flavour(entered: &Entered) -> Result<(), BuildError> {
    let Entered::Null { ref code, .. } = *entered else {
        return Ok(());
    };
    if NULL_FLAVOURS
        .iter()
        .any(|&(known, _)| known == code.as_str())
    {
        return Ok(());
    }
    Err(BuildError::UnknownNullFlavour {
        code: code.as_str().to_owned(),
    })
}

/// The rubric the openEHR terminology gives a null flavour.
pub(crate) fn null_flavour_rubric(code: &str) -> &'static str {
    NULL_FLAVOURS
        .iter()
        .find(|&&(known, _)| known == code)
        .map_or("", |&(_, rubric)| rubric)
}

/// Every value entered against `key` under one occurrence path, in occurrence
/// order.
///
/// The path is part of the address: the same field inside the second instance
/// of a repeating group is a different slot from the one inside the first, and
/// a lookup that ignored the path would put both into whichever instance it
/// happened to be building.
pub(crate) fn entered_for<'v>(
    values: &'v FormValues,
    key: &NodeKey,
    path: &[usize],
) -> Vec<&'v Entered> {
    let mut found: Vec<_> = values
        .iter()
        .filter(|(slot, _)| slot.key == *key && slot.group_path == path)
        .collect();
    found.sort_by_key(|(slot, _)| slot.occurrence);
    found.into_iter().map(|(_, entered)| entered).collect()
}
