// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Reading a web template another tool produced.
//!
//! # What a node becomes
//!
//! A node with children becomes a group, a node with inputs becomes a field,
//! and a node with neither becomes a group with no items. A node with both is
//! refused: no published implementation writes one, and reading it would mean
//! guessing which half the document meant.
//!
//! # What names a node here
//!
//! A [`ferrochart_form::key::NodeKey`] is a chain of steps, one per node, and
//! each step is read off the end of the node's `aqlPath` plus its `nodeId`,
//! `rmType` and `name`. Two facts about that are worth knowing before
//! comparing these keys with the ones `ferrochart-compile` derives from the
//! same operational template.
//!
//! A web template has already removed levels, so a child's `aqlPath` may
//! extend its parent's by several attributes where the compiler would have a
//! step for each. The path itself is kept verbatim, so nothing is lost, but
//! the chain is shorter.
//!
//! And a leaf node states the class of the value rather than the class of the
//! node holding it: a `DV_TEXT` node whose path ends `/items[at0002]/value` is
//! the `value` of an `ELEMENT` the document never mentions (openEHR RM
//! Release-1.1.0 `data_structures.html` section 5.2.3). The step therefore
//! carries the class the document states.

use std::collections::BTreeMap;

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{FieldKind, FormField, NullFlavour};
use ferrochart_form::group::{FormGroup, FormItem, GroupShape};
use ferrochart_form::ids::{
    ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId,
};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::occurrences::Occurrences;
use ferrochart_form::text::Localized;
use serde_json::{Map, Value};

use crate::aql;
use crate::document::WebTemplateDocument;
use crate::error::ReadError;
use crate::json;
use crate::kind;
use crate::source::SourceSpelling;

/// The document a web template describes, read into a form definition beside
/// the spelling it arrived in.
///
/// # Errors
/// [`ReadError`] when the document is not a JSON object, omits a member a web
/// template always states, states one as the wrong kind of value, or states a
/// node this crate cannot read into a field.
pub fn document(value: &Value) -> Result<WebTemplateDocument, ReadError> {
    let root = json::object(value, "the document")?;
    let default_language = json::required_str(root, "defaultLanguage", "the document")?;
    let mut languages: Vec<LanguageTag> = Vec::new();
    for stated in json::optional_array(root, "languages", "the document")? {
        let tag = stated.as_str().ok_or_else(|| ReadError::WrongType {
            at: "the document".to_owned(),
            member: "languages",
            expected: "an array of strings",
            found: json::kind_of(stated),
        })?;
        languages.push(LanguageTag::new(tag));
    }

    let tree = root
        .get("tree")
        .ok_or_else(|| ReadError::MissingMember {
            at: "the document".to_owned(),
            member: "tree",
        })
        .and_then(|tree| json::object(tree, "the tree"))?;

    let mut reader = Reader {
        default_language: LanguageTag::new(default_language),
        source: SourceSpelling::none(),
        unmodelled: Vec::new(),
    };
    let root_ident = Reader::ident(tree, "the tree")?;
    let root_key = NodeKey::root().child(root_ident.step(0));
    let root_group = match reader.node(tree, &root_key, &root_ident)? {
        FormItem::Group(group) => *group,
        FormItem::Field(field) => {
            return Err(ReadError::UnreadableInputs {
                at: field.key.to_string(),
                rm_type: field.rm_type.to_string(),
                reason: "the tree root states inputs, and a form is rooted in a group".to_owned(),
            });
        }
        _ => {
            return Err(ReadError::NotAnObject {
                at: "the tree".to_owned(),
            });
        }
    };

    let mut document_members = root.clone();
    document_members.remove("tree");
    reader.source.set_document(document_members);

    let form = FormDefinition {
        format_version: FORMAT_VERSION,
        template_id: TemplateId::new(json::required_str(root, "templateId", "the document")?),
        default_language: reader.default_language.clone(),
        languages,
        root: root_group,
    };
    Ok(WebTemplateDocument::from_parts(
        form,
        reader.source,
        reader.unmodelled,
    ))
}

/// What tells one node apart from its siblings.
pub(crate) struct Ident {
    rm_attribute: String,
    node_id: Option<String>,
    archetype_id: Option<String>,
    rm_type: String,
    pinned_name: Option<String>,
}

impl Ident {
    /// The five parts that separate a node from a sibling, without its
    /// position (`docs/architecture.md` section 6.2; no specification governs
    /// this: our own design).
    fn discriminator(&self) -> (String, String, String, String, String) {
        (
            self.rm_attribute.clone(),
            self.node_id.clone().unwrap_or_default(),
            self.archetype_id.clone().unwrap_or_default(),
            self.rm_type.clone(),
            self.pinned_name.clone().unwrap_or_default(),
        )
    }

    fn step(&self, sibling_ordinal: usize) -> KeyStep {
        KeyStep {
            rm_attribute: RmAttributeName::new(self.rm_attribute.clone()),
            node_id: self.node_id.clone().map(LocalCode::new),
            archetype_id: self.archetype_id.clone().map(ArchetypeId::new),
            rm_type: RmTypeName::new(self.rm_type.clone()),
            pinned_name: self.pinned_name.clone(),
            sibling_ordinal,
        }
    }
}

struct Reader {
    default_language: LanguageTag,
    source: SourceSpelling,
    unmodelled: Vec<NodeKey>,
}

/// Whether a `nodeId` names a whole archetype rather than a node inside one.
///
/// An openEHR archetype identifier is a dotted, hyphenated string
/// (`openEHR-EHR-OBSERVATION.blood_pressure.v2`; openEHR BASE Release-1.2.0
/// `base_types.html` section 5, `ARCHETYPE_ID`), and an archetype's own node
/// code is an `at`, `id` or `ac` code with no dot in it (openEHR AM
/// Release-2.3.0 `AOM1.4.html` section 4.2.3.1).
fn names_an_archetype(node_id: &str) -> bool {
    node_id.contains('.')
}

/// What shape the Reference Model gives a group's content.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 4.3 defines the
/// `ITEM_STRUCTURE` subtypes, and section 5.2.2 the `CLUSTER`.
fn shape_of(rm_type: &str) -> GroupShape {
    match rm_type {
        "ITEM_SINGLE" => GroupShape::Single,
        "ITEM_LIST" => GroupShape::List,
        "ITEM_TABLE" => GroupShape::Table,
        "ITEM_TREE" => GroupShape::Tree,
        "CLUSTER" => GroupShape::Cluster,
        _ => GroupShape::Plain,
    }
}

/// How many times a node may appear.
///
/// `min` and `max` are the flattened integers a web template carries rather
/// than an occurrences interval to re-derive, so they are read as they are.
/// `max` is `-1` for an unbounded node. A document that omits `min` is read as
/// zero, which is what the published implementations fill in for a sparse
/// root; one that omits `max` is read as one, the count that admits a value
/// without admitting a repeat the document never stated.
fn occurrences_of(obj: &Map<String, Value>, at: &str) -> Result<Occurrences, ReadError> {
    let min = json::optional_i64(obj, "min", at)?.unwrap_or(0);
    let max = json::optional_i64(obj, "max", at)?.unwrap_or(1);
    let impossible = || ReadError::ImpossibleOccurrences {
        at: at.to_owned(),
        min,
        max,
    };
    let minimum = u32::try_from(min).map_err(|_| impossible())?;
    if max == -1 {
        return Ok(Occurrences::unbounded_from(minimum));
    }
    let maximum = u32::try_from(max).map_err(|_| impossible())?;
    if maximum < minimum {
        return Err(impossible());
    }
    Ok(Occurrences::bounded(minimum, maximum))
}

impl Reader {
    /// What tells the node at `obj` apart from its siblings.
    fn ident(obj: &Map<String, Value>, at: &str) -> Result<Ident, ReadError> {
        let aql_path = json::optional_str(obj, "aqlPath", at)?.unwrap_or_default();
        let node_id = json::optional_str(obj, "nodeId", at)?.filter(|id| !id.is_empty());
        let archetype = node_id.filter(|id| names_an_archetype(id));
        Ok(Ident {
            rm_attribute: aql::last_attribute(aql_path).to_owned(),
            node_id: node_id
                .filter(|id| !names_an_archetype(id))
                .map(str::to_owned),
            archetype_id: archetype.map(str::to_owned),
            rm_type: json::required_str(obj, "rmType", at)?.to_owned(),
            pinned_name: json::optional_str(obj, "name", at)?.map(str::to_owned),
        })
    }

    /// The text a node is labelled with.
    ///
    /// `localizedNames` states it per language and is preferred; a document
    /// that states only `localizedName` states it in the document's default
    /// language, and one that states only `name` states it there too.
    fn label(&self, obj: &Map<String, Value>, at: &str) -> Result<Localized, ReadError> {
        let localized = json::localized(obj, "localizedNames", at)?;
        if !localized.is_empty() {
            return Ok(localized);
        }
        let stated =
            json::optional_str(obj, "localizedName", at)?.or(json::optional_str(obj, "name", at)?);
        Ok(stated.map_or_else(Localized::empty, |text| {
            Localized::in_language(self.default_language.clone(), text)
        }))
    }

    /// The node at `obj`, as a group or a field.
    fn node(
        &mut self,
        obj: &Map<String, Value>,
        key: &NodeKey,
        ident: &Ident,
    ) -> Result<FormItem, ReadError> {
        let at = key.to_string();
        let children = json::optional_array(obj, "children", &at)?;
        let input_values = json::optional_array(obj, "inputs", &at)?;
        if !children.is_empty() && !input_values.is_empty() {
            return Err(ReadError::ChildrenAndInputs { at });
        }

        let mut members = obj.clone();
        members.remove("children");
        self.source.set_node(key.clone(), members);

        let label = self.label(obj, &at)?;
        let help = json::localized(obj, "localizedDescriptions", &at)?;
        let occurrences = occurrences_of(obj, &at)?;

        if !input_values.is_empty() {
            let inputs = kind::read::parse_inputs(input_values, &at)?;
            let mut proportion_types = Vec::new();
            for stated in json::optional_array(obj, "proportionTypes", &at)? {
                proportion_types.push(stated.as_str().unwrap_or_default().to_owned());
            }
            if let Some(field_kind) =
                kind::read::field_kind(&ident.rm_type, &proportion_types, &inputs, &at)?
            {
                return Ok(FormItem::Field(Box::new(Reader::field(
                    key,
                    ident,
                    label,
                    help,
                    occurrences,
                    field_kind,
                ))));
            }
            // A class the derivation table has no row for, the `PARTY_PROXY`
            // family a web template states for a composition's context among
            // them. It is recorded so a caller can see it rather than find it
            // missing.
            self.unmodelled.push(key.clone());
        }

        let items = self.items(children, key)?;
        Ok(FormItem::Group(Box::new(FormGroup {
            key: key.clone(),
            rm_type: RmTypeName::new(ident.rm_type.clone()),
            archetype_id: ident.archetype_id.clone().map(ArchetypeId::new),
            label,
            help,
            occurrences,
            shape: shape_of(&ident.rm_type),
            name_constraint: None,
            is_ordered: false,
            is_unique: false,
            is_deprecated: false,
            items,
            undetermined: Vec::new(),
        })))
    }

    fn field(
        key: &NodeKey,
        ident: &Ident,
        label: Localized,
        help: Localized,
        occurrences: Occurrences,
        kind: FieldKind,
    ) -> FormField {
        FormField {
            key: key.clone(),
            rm_type: RmTypeName::new(ident.rm_type.clone()),
            label,
            help,
            occurrences,
            kind,
            name_constraint: None,
            reference_ranges: None,
            is_ordered: false,
            is_unique: false,
            // NOTE: RM Release-1.1.0 `data_structures.html` section 5.2.3
            // makes the null mechanism an `ELEMENT` invariant, and a web
            // template states no member for it.
            null_flavour: NullFlavour::absent(),
            prefill: None,
            is_deprecated: false,
            is_fixed: false,
        }
    }

    /// The children of a node, keyed and read in document order.
    fn items(&mut self, children: &[Value], parent: &NodeKey) -> Result<Vec<FormItem>, ReadError> {
        let at = parent.to_string();
        let mut idents = Vec::with_capacity(children.len());
        for child in children {
            let obj = json::object(child, &at)?;
            idents.push(Reader::ident(obj, &at)?);
        }
        let mut counts: BTreeMap<(String, String, String, String, String), usize> = BTreeMap::new();
        for ident in &idents {
            *counts.entry(ident.discriminator()).or_default() += 1;
        }
        let mut seen: BTreeMap<(String, String, String, String, String), usize> = BTreeMap::new();
        let mut items = Vec::with_capacity(children.len());
        for (child, ident) in children.iter().zip(&idents) {
            let discriminator = ident.discriminator();
            let ordinal = seen.entry(discriminator.clone()).or_default();
            let tied = counts.get(&discriminator).copied().unwrap_or(1) > 1;
            let mut key = parent.child(ident.step(*ordinal));
            key.is_positional = key.is_positional || tied;
            *ordinal = ordinal.saturating_add(1);
            let obj = json::object(child, &at)?;
            items.push(self.node(obj, &key, ident)?);
        }
        Ok(items)
    }
}

/// The node at `obj` read on its own, for the writer to compare a source node
/// against the form definition it is writing.
///
/// The object carries no `children`, so a group comes back with no items. That
/// is what the comparison wants: the writer builds the child list from the
/// form definition and only asks whether the node itself still says what the
/// source said.
///
/// # Errors
/// [`ReadError`] for the reasons [`document`] does.
pub(crate) fn node_alone(
    obj: &Map<String, Value>,
    key: &NodeKey,
    default_language: &LanguageTag,
) -> Result<FormItem, ReadError> {
    let mut reader = Reader {
        default_language: default_language.clone(),
        source: SourceSpelling::none(),
        unmodelled: Vec::new(),
    };
    let ident = Reader::ident(obj, &key.to_string())?;
    reader.node(obj, key, &ident)
}
