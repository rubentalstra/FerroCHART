// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A COMPOSITION becomes form values again.
//!
//! The walk is over the **data**, not over the form definition, and that is
//! the load-bearing choice. Walking the form and looking each node up in the
//! data would find every value the form expects and would never notice a node
//! the form does not cover, so an edit round trip would silently delete a
//! colleague's content. Walking the data means every node is either matched to
//! a field or reported.
//!
//! All citations are openEHR RM Release-1.1.0.

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::field::FormField;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::key::NodeKey;
use openehr_rm::v1_2::composition::composition::Composition;
use openehr_rm::v1_2::composition::content::content_item::ContentItem;
use openehr_rm::v1_2::composition::content::entry::activity::Activity;
use openehr_rm::v1_2::data_structures::history::event::Event;
use openehr_rm::v1_2::data_structures::history::history::History;
use openehr_rm::v1_2::data_structures::item_structure::item_structure::ItemStructure;
use openehr_rm::v1_2::data_structures::representation::element::Element;
use openehr_rm::v1_2::data_structures::representation::item::Item;
use openehr_rm::v1_2::data_types::text::dv_text::DvText;

use crate::datum;
use crate::error::ReadError;
use ferrochart_form::values::{Entered, FormValues};

/// How many data nodes of one identity have been seen under one parent.
///
/// A repeatable node produces siblings that share an `archetype_node_id` and
/// a name, and the form definition tells them apart by the sibling ordinal in
/// their key (`docs/architecture.md` section 6.2). The reader has to do the
/// same, or every sibling reads into the first form group and the rest come
/// back empty.
type Seen = std::collections::BTreeMap<String, Tally>;

/// How many instances of one repeating group have arrived under one occurrence
/// path.
///
/// The builder gives every instance of a repeating group an index, and the
/// slots beneath it carry that index. The read-back has to hand out the same
/// indices, and the only order the data carries is document order, so the nth
/// instance of a group under one path is instance n.
type Repeats = std::collections::BTreeMap<(NodeKey, Vec<usize>), usize>;

/// How many data nodes of one identity arrived, and how many form nodes were
/// waiting for them.
#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    /// Data nodes assigned so far.
    used: usize,
    /// Form nodes that share the identity.
    candidates: usize,
}

/// What a read-back found.
#[derive(Debug, Clone, Default)]
pub struct ReadBack {
    /// The values, keyed the way the form definition is keyed.
    pub values: FormValues,
    /// The Reference Model path of every node the form definition does not
    /// cover.
    ///
    /// Reported rather than dropped. A composition can carry content this
    /// form has no field for, because the template was revised or because
    /// another system wrote it, and an editing round trip that silently
    /// dropped it would delete somebody's data.
    pub uncovered: Vec<String>,
    /// Where the reader had to place a node by position and could not be sure
    /// it placed it right.
    ///
    /// Several form nodes can share an `archetype_node_id` and a name, and the
    /// only thing separating them is their order. openEHR BASE Release-1.2.0
    /// `architecture_overview.html` section 11.2.4 says so directly:
    /// "Archetype paths are not guaranteed to uniquely identify items in
    /// data". Where fewer data nodes arrive than the form has tied siblings,
    /// some sibling produced nothing and every later one shifts up, so the
    /// assignment is a guess. It is made, in order, and reported here rather
    /// than presented as certain.
    pub ambiguous: Vec<String>,
    /// The per-identity counters the walk keeps. Not part of the result.
    seen: Seen,
    /// The per-group instance counters the walk keeps. Not part of the result.
    repeats: Repeats,
}

/// The occurrence path below one matched group.
///
/// A group the template lets repeat contributes its instance index, counted in
/// document order from zero, and one that cannot repeat contributes nothing.
/// This is the inverse of what the builder appends, so a value comes back into
/// the slot it was built from.
fn descend(group: &FormGroup, path: &[usize], found: &mut ReadBack) -> Vec<usize> {
    if !group.occurrences.is_repeatable() {
        return path.to_vec();
    }
    let seen = found
        .repeats
        .entry((group.key.clone(), path.to_vec()))
        .or_default();
    let index = *seen;
    *seen += 1;
    let mut grown = Vec::with_capacity(path.len() + 1);
    grown.extend_from_slice(path);
    grown.push(index);
    grown
}

/// Reads `composition` back into the values `definition` describes.
///
/// # Errors
/// [`ReadError::WrongTemplate`] when the document was built from a different
/// template, and [`ReadError::WrongShape`] when a node carries a value class
/// the field there does not derive to.
pub fn values(
    definition: &FormDefinition,
    composition: &Composition,
) -> Result<ReadBack, ReadError> {
    if let Some(stated) = composition
        .archetype_details
        .as_ref()
        .and_then(|details| details.template_id.as_ref())
        .filter(|stated| stated.value != definition.template_id.as_str())
    {
        return Err(ReadError::WrongTemplate {
            expected: definition.template_id.as_str().to_owned(),
            found: stated.value.clone(),
        });
    }

    let mut found = ReadBack::default();
    let root = &definition.root;

    // Where the template constrains `category`, the form has a field for it,
    // so the value has to come back into that field or the round trip loses
    // it.
    if let Some(field) = crate::build::category_field(root) {
        found.values.set(
            field.key.clone(),
            Entered::Value(ferrochart_form::values::Datum::Coded {
                terminology: composition
                    .category
                    .defining_code
                    .terminology_id
                    .value
                    .clone(),
                code: composition.category.defining_code.code_string.clone(),
                rubric: composition.category.value.clone(),
            }),
        );
    }
    let empty: Vec<ContentItem> = Vec::new();
    let content = composition.content.as_deref().unwrap_or(&empty);
    // The root is one node, so nothing above it repeats.
    let root_path: &[usize] = &[];

    if root.rm_type.as_str() == "COMPOSITION" {
        // `EVENT_CONTEXT.other_context` is the one place a template can put
        // fields outside `content`, and the builder writes it, so a reader
        // that skipped it would lose them without reporting anything.
        if let (Some(context), Some(group)) = (
            composition.context.as_ref(),
            child_by_attribute(root, "context"),
        ) {
            structure(
                group,
                "other_context",
                "/context",
                root_path,
                context.other_context.as_ref(),
                &mut found,
            );
        }
        for (index, item) in content.iter().enumerate() {
            content_item(root, "content", index, item, root_path, &mut found);
        }
    } else {
        // The envelope was built around a template rooted at an entry, so the
        // form's root is that entry and the composition's content holds it.
        // The identity is still checked: a document can carry a second entry
        // this form knows nothing about.
        let root_step = root.key.terminal();
        for (index, item) in content.iter().enumerate() {
            let path = format!("/content[{index}]");
            let (node_id, name) = content_identity(item);
            let matched = identity_matches(
                root.archetype_id
                    .as_ref()
                    .map(ferrochart_form::ids::ArchetypeId::as_str),
                root_step
                    .and_then(|step| step.node_id.as_ref())
                    .map(ferrochart_form::ids::LocalCode::as_str),
                root_step.and_then(|step| step.pinned_name.as_deref()),
                node_id,
                name,
            );
            if matched.is_some() {
                read_against(root, &path, root_path, item, &mut found);
            } else {
                found.uncovered.push(path);
            }
        }
    }
    // A tie the data did not fill completely was resolved by position, and
    // position is the one thing the specification refuses to guarantee.
    let shifted: Vec<String> = found
        .seen
        .iter()
        .filter(|&(_, tally)| tally.used < tally.candidates)
        .map(|(slot, _)| slot.clone())
        .collect();
    found.ambiguous = shifted;
    Ok(found)
}

/// Matches one content item against the group the form has for it.
fn content_item(
    parent: &FormGroup,
    attribute: &str,
    index: usize,
    item: &ContentItem,
    path: &[usize],
    found: &mut ReadBack,
) {
    let rm_path = format!("/{attribute}[{index}]");
    let (node_id, name) = content_identity(item);
    let Some(group) = child_matching(parent, attribute, node_id, name, found) else {
        found.uncovered.push(rm_path);
        return;
    };
    let inner = descend(group, path, found);
    read_against(group, &rm_path, &inner, item, found);
}

/// Reads a content item against the form group that matched it.
fn read_against(
    group: &FormGroup,
    rm_path: &str,
    path: &[usize],
    item: &ContentItem,
    found: &mut ReadBack,
) {
    match *item {
        ContentItem::Section(ref section) => {
            let empty: Vec<ContentItem> = Vec::new();
            for (index, child) in section
                .items
                .as_deref()
                .unwrap_or(&empty)
                .iter()
                .enumerate()
            {
                content_item(group, "items", index, child, path, found);
            }
        }
        ContentItem::Observation(ref entry) => {
            history(group, "data", rm_path, path, &entry.data, found);
            if let Some(ref state) = entry.state {
                history(group, "state", rm_path, path, state, found);
            }
            structure(
                group,
                "protocol",
                rm_path,
                path,
                entry.protocol.as_ref(),
                found,
            );
        }
        ContentItem::Evaluation(ref entry) => {
            structure(group, "data", rm_path, path, Some(&entry.data), found);
            structure(
                group,
                "protocol",
                rm_path,
                path,
                entry.protocol.as_ref(),
                found,
            );
        }
        ContentItem::AdminEntry(ref entry) => {
            structure(group, "data", rm_path, path, Some(&entry.data), found);
        }
        ContentItem::Instruction(ref entry) => {
            let empty: Vec<Activity> = Vec::new();
            for (index, activity) in entry
                .activities
                .as_deref()
                .unwrap_or(&empty)
                .iter()
                .enumerate()
            {
                let inner = format!("{rm_path}/activities[{index}]");
                let Some(node) = child_matching(
                    group,
                    "activities",
                    &activity.archetype_node_id,
                    text_of(&activity.name),
                    found,
                ) else {
                    found.uncovered.push(inner);
                    continue;
                };
                let under = descend(node, path, found);
                structure(
                    node,
                    "description",
                    &inner,
                    &under,
                    Some(&activity.description),
                    found,
                );
            }
            structure(
                group,
                "protocol",
                rm_path,
                path,
                entry.protocol.as_ref(),
                found,
            );
        }
        ContentItem::Action(ref entry) => {
            structure(
                group,
                "description",
                rm_path,
                path,
                Some(&entry.description),
                found,
            );
            structure(
                group,
                "protocol",
                rm_path,
                path,
                entry.protocol.as_ref(),
                found,
            );
        }
        // A `GENERIC_ENTRY` holds data no archetype governs (openEHR RM
        // Release-1.1.0 `ehr.html` section 8.3.9), so no form field can
        // cover it and it is reported rather than read.
        ContentItem::GenericEntry(_) => found.uncovered.push(rm_path.to_owned()),
    }
}

/// Reads a `HISTORY` and the events under it.
fn history(
    parent: &FormGroup,
    attribute: &str,
    rm_path: &str,
    path: &[usize],
    history: &History<ItemStructure>,
    found: &mut ReadBack,
) {
    let rm_path = format!("{rm_path}/{attribute}");
    let Some(group) = child_matching(
        parent,
        attribute,
        &history.archetype_node_id,
        text_of(&history.name),
        found,
    ) else {
        found.uncovered.push(rm_path);
        return;
    };
    let empty: Vec<Event<ItemStructure>> = Vec::new();
    for (index, event) in history.events.as_ref().unwrap_or(&empty).iter().enumerate() {
        let inner = format!("{rm_path}/events[{index}]");
        let (node_id, name, data, state) = match *event {
            Event::PointEvent(ref it) => (
                &it.archetype_node_id,
                text_of(&it.name),
                &it.data,
                it.state.as_ref(),
            ),
            Event::IntervalEvent(ref it) => (
                &it.archetype_node_id,
                text_of(&it.name),
                &it.data,
                it.state.as_ref(),
            ),
        };
        let Some(node) = child_matching(group, "events", node_id, name, found) else {
            found.uncovered.push(inner);
            continue;
        };
        let under = descend(node, path, found);
        structure(node, "data", &inner, &under, Some(data), found);
        structure(node, "state", &inner, &under, state, found);
    }
}

/// Reads an `ITEM_STRUCTURE` under `attribute`.
fn structure(
    parent: &FormGroup,
    attribute: &str,
    rm_path: &str,
    path: &[usize],
    structure: Option<&ItemStructure>,
    found: &mut ReadBack,
) {
    let Some(structure) = structure else {
        return;
    };
    let rm_path = format!("{rm_path}/{attribute}");
    let (node_id, name, items) = match *structure {
        ItemStructure::ItemTree(ref it) => (
            &it.archetype_node_id,
            text_of(&it.name),
            it.items.as_deref().unwrap_or_default(),
        ),
        ItemStructure::ItemList(ref it) => {
            let Some(group) = child_matching(
                parent,
                attribute,
                &it.archetype_node_id,
                text_of(&it.name),
                found,
            ) else {
                found.uncovered.push(rm_path);
                return;
            };
            for (index, element) in it.items.as_deref().unwrap_or_default().iter().enumerate() {
                element_under(group, &rm_path, path, index, element, found);
            }
            return;
        }
        ItemStructure::ItemSingle(ref it) => {
            let Some(group) = child_matching(
                parent,
                attribute,
                &it.archetype_node_id,
                text_of(&it.name),
                found,
            ) else {
                found.uncovered.push(rm_path);
                return;
            };
            element_under(group, &rm_path, path, 0, &it.item, found);
            return;
        }
        ItemStructure::ItemTable(ref it) => {
            let Some(group) = child_matching(
                parent,
                attribute,
                &it.archetype_node_id,
                text_of(&it.name),
                found,
            ) else {
                found.uncovered.push(rm_path);
                return;
            };
            for (index, row) in it.rows.as_deref().unwrap_or_default().iter().enumerate() {
                let inner = format!("{rm_path}/rows[{index}]");
                for (column, element) in row.items.iter().enumerate() {
                    item_under(group, &inner, path, column, element, found);
                }
            }
            return;
        }
    };
    let Some(group) = child_matching(parent, attribute, node_id, name, found) else {
        found.uncovered.push(rm_path);
        return;
    };
    for (index, item) in items.iter().enumerate() {
        item_under(group, &rm_path, path, index, item, found);
    }
}

/// Reads one `ITEM`, which is a `CLUSTER` or an `ELEMENT`.
fn item_under(
    parent: &FormGroup,
    rm_path: &str,
    path: &[usize],
    index: usize,
    item: &Item,
    found: &mut ReadBack,
) {
    match *item {
        Item::Element(ref element) => element_under(parent, rm_path, path, index, element, found),
        Item::Cluster(ref cluster) => {
            let inner = format!("{rm_path}/items[{index}]");
            let Some(group) = child_matching(
                parent,
                "items",
                &cluster.archetype_node_id,
                text_of(&cluster.name),
                found,
            ) else {
                found.uncovered.push(inner);
                return;
            };
            let under = descend(group, path, found);
            for (child, item) in cluster.items.iter().enumerate() {
                item_under(group, &inner, &under, child, item, found);
            }
        }
    }
}

/// Reads one `ELEMENT` into the field the form has for it.
fn element_under(
    parent: &FormGroup,
    rm_path: &str,
    path: &[usize],
    index: usize,
    element: &Element,
    found: &mut ReadBack,
) {
    let inner = format!("{rm_path}/items[{index}]");
    let Some(field) = field_matching(
        parent,
        &element.archetype_node_id,
        text_of(&element.name),
        found,
    ) else {
        found.uncovered.push(inner);
        return;
    };
    // Section 5.2.3: an element carries exactly one of a value and a null
    // flavour. A node with neither is malformed rather than empty, and is
    // reported rather than read as an absent value.
    let entered = match (element.value.as_ref(), element.null_flavour.as_ref()) {
        (Some(value), _) => {
            let Ok(datum) = datum::read(&inner, value) else {
                // A class no field derives to is content this form cannot
                // hold, which is the same case as a node it does not cover.
                found.uncovered.push(inner);
                return;
            };
            Entered::Value(datum)
        }
        (None, Some(flavour)) => Entered::Null {
            code: ferrochart_form::ids::LocalCode::new(&flavour.defining_code.code_string),
            reason: element
                .null_reason
                .as_ref()
                .map(|text| text_of(text).to_owned()),
        },
        (None, None) => {
            found.uncovered.push(inner);
            return;
        }
    };
    let occurrence = found.values.occurrences_in(&field.key, path);
    found
        .values
        .set_in(field.key.clone(), path.to_vec(), occurrence, entered);
}

/// The node id and name a content item carries.
fn content_identity(item: &ContentItem) -> (&str, &str) {
    match *item {
        ContentItem::Section(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::Observation(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::Evaluation(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::AdminEntry(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::Instruction(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::Action(ref it) => (&it.archetype_node_id, text_of(&it.name)),
        ContentItem::GenericEntry(ref it) => (&it.archetype_node_id, text_of(&it.name)),
    }
}

/// The text of a `DV_TEXT`, whichever subtype it is.
fn text_of(text: &DvText) -> &str {
    match *text {
        DvText::DvText(ref it) => &it.value,
        DvText::DvCodedText(ref it) => &it.value,
    }
}

/// The group under `attribute` whose node id, and pinned name where the
/// template pins one, match the data.
///
/// openEHR BASE Release-1.2.0 `architecture_overview.html` section 11.2.2.3
/// pairs the node id with the name for exactly this reason: "the combination
/// of an archetype node identifier and a name value is very common in
/// archetyped databases". Where the template pins no name, the clinician chose
/// it, so matching on it would be a guess and the node id alone decides.
fn child_matching<'g>(
    parent: &'g FormGroup,
    attribute: &str,
    node_id: &str,
    name: &str,
    found: &mut ReadBack,
) -> Option<&'g FormGroup> {
    let candidates: Vec<(Match, &FormGroup)> = parent
        .items
        .iter()
        .filter_map(|item| {
            let FormItem::Group(ref child) = *item else {
                return None;
            };
            let step = child.key.terminal()?;
            if step.rm_attribute.as_str() != attribute {
                return None;
            }
            let strength = identity_matches(
                child
                    .archetype_id
                    .as_ref()
                    .map(ferrochart_form::ids::ArchetypeId::as_str),
                step.node_id
                    .as_ref()
                    .map(ferrochart_form::ids::LocalCode::as_str),
                step.pinned_name.as_deref(),
                node_id,
                name,
            )?;
            Some((strength, &**child))
        })
        .collect();
    pick(candidates, parent, attribute, node_id, name, found)
}

/// The field of `parent` whose node id, and pinned name where there is one,
/// match the data.
fn field_matching<'g>(
    parent: &'g FormGroup,
    node_id: &str,
    name: &str,
    found: &mut ReadBack,
) -> Option<&'g FormField> {
    let candidates: Vec<(Match, &FormField)> = parent
        .items
        .iter()
        .filter_map(|item| {
            let FormItem::Field(ref field) = *item else {
                return None;
            };
            let step = field.key.terminal()?;
            let strength = identity_matches(
                None,
                step.node_id
                    .as_ref()
                    .map(ferrochart_form::ids::LocalCode::as_str),
                step.pinned_name.as_deref(),
                node_id,
                name,
            )?;
            Some((strength, &**field))
        })
        .collect();
    pick(candidates, parent, "", node_id, name, found)
}

/// The single child group under `attribute`, whatever its identity.
///
/// `EVENT_CONTEXT` is not a `LOCATABLE` (openEHR RM Release-1.1.0 `ehr.html`
/// section 5.4.2 has it inherit `PATHABLE`), so it carries no name and no
/// `archetype_node_id` to match on. The attribute is 0..1, so naming it is
/// enough.
fn child_by_attribute<'g>(parent: &'g FormGroup, attribute: &str) -> Option<&'g FormGroup> {
    parent.items.iter().find_map(|item| {
        let FormItem::Group(ref child) = *item else {
            return None;
        };
        child
            .key
            .terminal()
            .is_some_and(|step| step.rm_attribute.as_str() == attribute)
            .then_some(&**child)
    })
}

/// The candidate whose turn it is.
///
/// Where one form node matches, it is that one and the counter is irrelevant.
/// Where several do, they are siblings the template cannot tell apart by
/// anything but position, so the nth data node of that identity belongs to the
/// nth form node. Beyond the last one the data carries more repeats than the
/// template declared, which is content the form does not cover.
fn pick<'g, T>(
    candidates: Vec<(Match, &'g T)>,
    parent: &FormGroup,
    attribute: &str,
    node_id: &str,
    name: &str,
    found: &mut ReadBack,
) -> Option<&'g T> {
    // An exact name match beats a wildcard, whatever the order the form
    // states them in.
    let best = candidates.iter().map(|&(strength, _)| strength).min()?;
    let candidates: Vec<&T> = candidates
        .into_iter()
        .filter_map(|(strength, node)| (strength == best).then_some(node))
        .collect();
    if candidates.len() <= 1 {
        return candidates.into_iter().next();
    }
    let slot = format!("{}|{attribute}|{node_id}|{name}", parent.key);
    let tally = found.seen.entry(slot).or_default();
    tally.candidates = candidates.len();
    let chosen = candidates.get(tally.used).copied();
    tally.used += 1;
    chosen
}

/// How well a form node's identity matches a data node's.
///
/// A form node that pins no name matches any name, so it would otherwise
/// shadow a sibling that pins exactly this one. An exact match therefore beats
/// a wildcard: the pinned name is a fact the template states, and the
/// unpinned sibling is the one whose name the clinician chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Match {
    /// The node id matches and the template pins this exact name.
    Exact,
    /// The node id matches and the template pins no name.
    Wildcard,
}

/// Whether a form node's identity matches a data node's.
///
/// At an archetype root the data carries the stringified archetype id rather
/// than a node code (`common.html` section 3.2.2), so both are tried. The
/// value is compared, never parsed: the Reference Model calls it "always an
/// at-code" and contradicts itself in the next sentence, and ADL 2 uses
/// `id`-codes.
fn identity_matches(
    archetype_id: Option<&str>,
    form_node_id: Option<&str>,
    pinned_name: Option<&str>,
    data_node_id: &str,
    data_name: &str,
) -> Option<Match> {
    let identified = archetype_id == Some(data_node_id) || form_node_id == Some(data_node_id);
    if !identified {
        return None;
    }
    match pinned_name {
        Some(pinned) if pinned == data_name => Some(Match::Exact),
        Some(_) => None,
        None => Some(Match::Wildcard),
    }
}
