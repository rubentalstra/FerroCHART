// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Writing a form definition out as a web template.
//!
//! # How a member the form definition does not model survives
//!
//! The tree comes from the form definition, and each node starts from the
//! object the source document spelled for the same key
//! ([`crate::source::SourceSpelling`]). A node the form definition still
//! states exactly as it was read goes back out untouched, so every member is
//! preserved down to the ones nested inside `inputs`. A node the form
//! definition changed keeps every member outside the modelled set for its kind
//! ([`crate::member::MODELLED_NODE_MEMBERS`] for a field,
//! [`crate::member::MODELLED_GROUP_MEMBERS`] for a group) and has the rest
//! rewritten, and inside `inputs` the same rule applies per input and per
//! coded option, so a per-option terminology binding survives a change
//! elsewhere on the node.
//!
//! A form definition with no source spelling, one the compiler derived from an
//! operational template, has every member written from the form.

use ferrochart_form::field::FormField;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;
use serde_json::{Map, Value};

use crate::document::WebTemplateDocument;
use crate::error::WriteError;
use crate::json;
use crate::kind;
use crate::member::{
    MODELLED_GROUP_MEMBERS, MODELLED_INPUT_MEMBERS, MODELLED_NODE_MEMBERS, MODELLED_OPTION_MEMBERS,
    MODELLED_ROOT_MEMBERS, MODELLED_TEXT_INPUT_MEMBERS, patched,
};
use crate::read;

/// A fact of the form definition the web template format has no member for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotCarried {
    /// What names the item the fact belongs to.
    pub key: NodeKey,
    /// The fact, as a person would name it.
    pub fact: &'static str,
}

/// A written web template, with what the format could not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct WrittenDocument {
    /// The document.
    pub json: Value,
    /// Every fact the form definition states that the format has no member
    /// for, in the order the tree states them.
    ///
    /// Empty is the normal case for a form definition read out of a web
    /// template. A form definition the compiler derived carries facts a web
    /// template cannot hold, and each one is listed here rather than dropped
    /// in silence.
    pub not_carried: Vec<NotCarried>,
}

/// `document` written as a web template.
///
/// # Errors
/// [`WriteError`] when a field states something the format has a member for
/// but no spelling of, which is a proportion kind outside the five openEHR RM
/// Release-1.1.0 `data_types.html` section 6.2.11 names.
pub fn document(document: &WebTemplateDocument) -> Result<WrittenDocument, WriteError> {
    let form = document.form();
    let mut writer = Writer {
        document,
        default_language: form.default_language.clone(),
        not_carried: Vec::new(),
    };
    let tree = writer.group(&form.root)?;

    let mut canonical = Map::new();
    canonical.insert(
        "templateId".to_owned(),
        Value::String(form.template_id.to_string()),
    );
    canonical.insert(
        "defaultLanguage".to_owned(),
        Value::String(form.default_language.to_string()),
    );
    if !form.languages.is_empty() {
        canonical.insert(
            "languages".to_owned(),
            Value::Array(
                form.languages
                    .iter()
                    .map(|tag| Value::String(tag.to_string()))
                    .collect(),
            ),
        );
    }
    canonical.insert("tree".to_owned(), tree);

    let root = patched(
        Some(document.source().document()),
        &canonical,
        &MODELLED_ROOT_MEMBERS,
    );
    Ok(WrittenDocument {
        json: Value::Object(root),
        not_carried: writer.not_carried,
    })
}

struct Writer<'a> {
    document: &'a WebTemplateDocument,
    default_language: LanguageTag,
    not_carried: Vec<NotCarried>,
}

/// The members every node states, whichever kind it is.
fn common_members(
    label: &Localized,
    help: &Localized,
    rm_type: &str,
    node_id: Option<String>,
    pinned_name: Option<&str>,
    occurrences: ferrochart_form::occurrences::Occurrences,
    default_language: &LanguageTag,
) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(name) = pinned_name {
        out.insert("name".to_owned(), Value::String(name.to_owned()));
    }
    if let Some(localized_name) = label.get(default_language) {
        out.insert(
            "localizedName".to_owned(),
            Value::String(localized_name.to_owned()),
        );
    }
    out.insert("rmType".to_owned(), Value::String(rm_type.to_owned()));
    if let Some(node_id) = node_id {
        out.insert("nodeId".to_owned(), Value::String(node_id));
    }
    out.insert("min".to_owned(), Value::from(occurrences.minimum));
    out.insert(
        "max".to_owned(),
        Value::from(occurrences.maximum.map_or(-1_i64, i64::from)),
    );
    if let Some(names) = json::localized_value(label) {
        out.insert("localizedNames".to_owned(), names);
    }
    if let Some(descriptions) = json::localized_value(help) {
        out.insert("localizedDescriptions".to_owned(), descriptions);
    }
    out
}

/// The `nodeId` a step states, which is its archetype identifier where it is
/// an archetype root and its own node code otherwise.
fn node_id_of(key: &NodeKey) -> Option<String> {
    let step = key.terminal()?;
    step.archetype_id
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| step.node_id.as_ref().map(ToString::to_string))
}

/// The node's source object with `children` set from `children`.
fn with_children(mut node: Map<String, Value>, children: Vec<Value>) -> Value {
    if children.is_empty() {
        node.remove("children");
    } else {
        node.insert("children".to_owned(), Value::Array(children));
    }
    Value::Object(node)
}

/// The canonical `inputs`, with every member the form definition does not
/// model taken from the source input of the same suffix.
///
/// A source input no canonical one claims is kept as it was. The `other`
/// input a web template states beside an open value set is the case that
/// matters: it describes the escape from the set rather than the value
/// (openEHR ITS-REST Release-1.1.0 `simplified_formats.html` section 4.7 on
/// the `|other` suffix), so nothing in the form definition regenerates it.
fn patched_inputs(source: Option<&Map<String, Value>>, canonical: &[Value]) -> Vec<Value> {
    let stated = source
        .and_then(|node| node.get("inputs"))
        .and_then(Value::as_array);
    let claimed: Vec<&str> = canonical
        .iter()
        .filter_map(|input| input.get("suffix").and_then(Value::as_str))
        .collect();
    let mut out: Vec<Value> = canonical
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let Some(input) = input.as_object() else {
                return input.clone();
            };
            let suffix = input.get("suffix").and_then(Value::as_str);
            let source_input = stated.and_then(|inputs| match suffix {
                Some(suffix) => inputs
                    .iter()
                    .find(|stated| stated.get("suffix").and_then(Value::as_str) == Some(suffix)),
                None => inputs.get(index),
            });
            let source_input = source_input.and_then(Value::as_object);
            let modelled = if input.get("type").and_then(Value::as_str) == Some("TEXT") {
                &MODELLED_TEXT_INPUT_MEMBERS[..]
            } else {
                &MODELLED_INPUT_MEMBERS[..]
            };
            let mut patched_input = patched(source_input, input, modelled);
            if let Some(list) = input.get("list").and_then(Value::as_array) {
                let source_list = source_input
                    .and_then(|stated| stated.get("list"))
                    .and_then(Value::as_array);
                patched_input.insert(
                    "list".to_owned(),
                    Value::Array(patched_options(source_list, list)),
                );
            }
            Value::Object(patched_input)
        })
        .collect();
    for input in stated.into_iter().flatten() {
        let Some(suffix) = input.get("suffix").and_then(Value::as_str) else {
            continue;
        };
        if !claimed.contains(&suffix) {
            out.push(input.clone());
        }
    }
    out
}

/// The canonical coded options, with every member the form definition does not
/// model taken from the source option of the same code.
fn patched_options(source: Option<&Vec<Value>>, canonical: &[Value]) -> Vec<Value> {
    canonical
        .iter()
        .map(|option| {
            let Some(option) = option.as_object() else {
                return option.clone();
            };
            let code = option.get("value").and_then(Value::as_str);
            let stated = source
                .and_then(|options| {
                    options
                        .iter()
                        .find(|stated| stated.get("value").and_then(Value::as_str) == code)
                })
                .and_then(Value::as_object);
            Value::Object(patched(stated, option, &MODELLED_OPTION_MEMBERS))
        })
        .collect()
}

impl Writer<'_> {
    /// Whether the source node at `key` still states what the form definition
    /// states, so its own spelling may go back out untouched.
    fn source_agrees(&self, key: &NodeKey, item: &FormItem) -> bool {
        let Some(source) = self.document.source().node(key) else {
            return false;
        };
        let Ok(read_back) = read::node_alone(source, key, &self.default_language) else {
            return false;
        };
        match (&read_back, item) {
            (FormItem::Group(read_back), FormItem::Group(stated)) => {
                let mut stated = (**stated).clone();
                stated.items = Vec::new();
                **read_back == stated
            }
            (FormItem::Field(read_back), FormItem::Field(stated)) => **read_back == **stated,
            _ => false,
        }
    }

    fn item(&mut self, item: &FormItem) -> Result<Value, WriteError> {
        match *item {
            FormItem::Group(ref group) => self.group(group),
            FormItem::Field(ref field) => self.field(field),
            _ => Ok(Value::Null),
        }
    }

    fn group(&mut self, group: &FormGroup) -> Result<Value, WriteError> {
        let mut children = Vec::with_capacity(group.items.len());
        for item in &group.items {
            children.push(self.item(item)?);
        }
        if group.name_constraint.is_some() {
            self.note(&group.key, "the constraint on the node's name");
        }
        for undetermined in &group.undetermined {
            self.note(&undetermined.key, "content the template left undetermined");
        }

        let source = self.document.source().node(&group.key);
        let owned = FormItem::Group(Box::new(group.clone()));
        if self.source_agrees(&group.key, &owned)
            && let Some(source) = source
        {
            return Ok(with_children(source.clone(), children));
        }

        let canonical = common_members(
            &group.label,
            &group.help,
            group.rm_type.as_str(),
            node_id_of(&group.key),
            group
                .key
                .terminal()
                .and_then(|step| step.pinned_name.as_deref()),
            group.occurrences,
            &self.default_language,
        );
        Ok(with_children(
            patched(source, &canonical, &MODELLED_GROUP_MEMBERS),
            children,
        ))
    }

    fn field(&mut self, field: &FormField) -> Result<Value, WriteError> {
        let at = field.key.to_string();
        if field.name_constraint.is_some() {
            self.note(&field.key, "the constraint on the node's name");
        }
        if field.reference_ranges.is_some() {
            self.note(&field.key, "the reference bands shown beside the value");
        }
        if field.prefill.is_some() {
            self.note(&field.key, "the value the template prefills the field with");
        }

        let source = self.document.source().node(&field.key);
        let owned = FormItem::Field(Box::new(field.clone()));
        if self.source_agrees(&field.key, &owned)
            && let Some(source) = source
        {
            return Ok(with_children(source.clone(), Vec::new()));
        }

        let written = kind::write::inputs_of(&field.kind, self.default_language.as_str(), &at)?;
        for fact in written.not_carried {
            self.note(&field.key, fact);
        }
        let mut canonical = common_members(
            &field.label,
            &field.help,
            field.rm_type.as_str(),
            node_id_of(&field.key),
            field
                .key
                .terminal()
                .and_then(|step| step.pinned_name.as_deref()),
            field.occurrences,
            &self.default_language,
        );
        if !written.proportion_types.is_empty() {
            canonical.insert(
                "proportionTypes".to_owned(),
                Value::Array(written.proportion_types),
            );
        }
        if !written.inputs.is_empty() {
            canonical.insert(
                "inputs".to_owned(),
                Value::Array(patched_inputs(source, &written.inputs)),
            );
        }
        Ok(with_children(
            patched(source, &canonical, &MODELLED_NODE_MEMBERS),
            Vec::new(),
        ))
    }

    fn note(&mut self, key: &NodeKey, fact: &'static str) {
        self.not_carried.push(NotCarried {
            key: key.clone(),
            fact,
        });
    }
}
