// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The corpus, and the surgery that turns one derived form into the revision
//! a replay has to explain.
//!
//! The material is real: the committed CKM operational template pack, read by
//! the ADL 1.4 reader and derived by the compiler. What a template revision
//! would have done is applied to the derived form here, because the pack holds
//! no two versions of one template, and none can be obtained: the openEHR CKM
//! serves no earlier revision of a template and no archetype in the pack
//! appears at two major versions (#21). `real_revision.rs` replays against the
//! largest real difference the pack can produce instead.

use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compile::{adl14, derive};
use ferrochart_form::definition::FormDefinition;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::RmTypeName;
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::layout::Layout;
use ferrochart_overlay::store::{Author, Placement};

/// The committed CKM operational template pack.
pub(crate) fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/templates/ckm")
}

/// Every committed `.opt` in the pack, in a stable order.
pub(crate) fn templates() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(corpus_dir()) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "opt"))
        .collect();
    found.sort();
    found
}

/// The derived form of one committed template, or `None` when the reader
/// refuses it.
pub(crate) fn derived(path: &Path) -> Option<FormDefinition> {
    let xml = fs::read_to_string(path).expect("a committed corpus file is UTF-8");
    let template = adl14::from_xml(&xml).ok()?;
    Some(derive::form(&template).expect("a template the reader reads derives"))
}

/// The derived form of the named committed template.
pub(crate) fn form(name: &str) -> FormDefinition {
    derived(&corpus_dir().join(name)).unwrap_or_else(|| panic!("{name} reads and derives"))
}

/// Every group and field key of a form, in walk order.
pub(crate) fn keys(form: &FormDefinition) -> Vec<NodeKey> {
    let mut found = Vec::new();
    walk(&form.root, &mut |key: &NodeKey| found.push(key.clone()));
    found
}

/// The first key whose last step satisfies `wanted`.
pub(crate) fn key_where(
    form: &FormDefinition,
    wanted: impl Fn(&KeyStep) -> bool,
) -> Option<NodeKey> {
    keys(form)
        .into_iter()
        .find(|key| key.terminal().is_some_and(&wanted))
}

/// The index path from the root group to the item `steps` names.
fn locate(group: &FormGroup, steps: &[KeyStep], at: &mut Vec<usize>) -> bool {
    if group.key.steps == steps {
        return true;
    }
    for (position, item) in group.items.iter().enumerate() {
        at.push(position);
        let found = match *item {
            FormItem::Group(ref nested) => locate(nested, steps, at),
            FormItem::Field(ref field) => field.key.steps == steps,
            _ => false,
        };
        if found {
            return true;
        }
        at.pop();
    }
    false
}

/// The index path from the root group to the item `key` names.
pub(crate) fn path_to(form: &FormDefinition, key: &NodeKey) -> Vec<usize> {
    let mut at = Vec::new();
    assert!(
        locate(&form.root, &key.steps, &mut at),
        "the form has no item at {key}"
    );
    at
}

/// The group at an index path.
fn group_at<'a>(root: &'a mut FormGroup, path: &[usize]) -> &'a mut FormGroup {
    let mut current = root;
    for step in path {
        let item = current.items.get_mut(*step).expect("the path is in range");
        match *item {
            FormItem::Group(ref mut nested) => current = nested.as_mut(),
            _ => panic!("the path leads through a field"),
        }
    }
    current
}

/// Runs `visit` over every key of a subtree, groups first.
fn walk(group: &FormGroup, visit: &mut impl FnMut(&NodeKey)) {
    visit(&group.key);
    for item in &group.items {
        match *item {
            FormItem::Group(ref nested) => walk(nested, visit),
            FormItem::Field(ref field) => visit(&field.key),
            _ => {}
        }
    }
}

/// Runs `edit` over every key of a subtree, groups first.
fn edit_keys(item: &mut FormItem, edit: &mut impl FnMut(&mut NodeKey)) {
    match *item {
        FormItem::Group(ref mut group) => edit_group_keys(group, edit),
        FormItem::Field(ref mut field) => edit(&mut field.key),
        _ => {}
    }
}

/// Runs `edit` over every key of a group and everything under it.
fn edit_group_keys(group: &mut FormGroup, edit: &mut impl FnMut(&mut NodeKey)) {
    edit(&mut group.key);
    for item in &mut group.items {
        edit_keys(item, edit);
    }
    for content in &mut group.undetermined {
        edit(&mut content.key);
    }
}

/// The item `key` names, taken out of the form.
pub(crate) fn take(form: &mut FormDefinition, key: &NodeKey) -> FormItem {
    let path = path_to(form, key);
    let (parent, at) = path.split_at(path.len().saturating_sub(1));
    let position = *at.first().expect("the item is not the root");
    group_at(&mut form.root, parent).items.remove(position)
}

/// Puts `item` under the group `parent` names, rekeyed to sit there.
pub(crate) fn put(form: &mut FormDefinition, parent: &NodeKey, mut item: FormItem) {
    let path = path_to(form, parent);
    let holder = group_at(&mut form.root, &path);
    let mut step = terminal_of(&item);
    step.sibling_ordinal = holder
        .items
        .iter()
        .filter(|held| terminal_of(held).rm_attribute == step.rm_attribute)
        .count();
    let mut prefix = parent.steps.clone();
    let depth = terminal_depth(&item);
    prefix.push(step);
    edit_keys(&mut item, &mut |key: &mut NodeKey| {
        let tail: Vec<KeyStep> = key.steps.split_off(depth);
        key.steps = prefix.iter().cloned().chain(tail).collect();
    });
    group_at(&mut form.root, &path).items.push(item);
}

/// A copy of the item `key` names, put back beside it under the same
/// attribute.
///
/// The copy carries the same five-part step tuple and the next ordinal, which
/// is the same-id sibling collision the measurement found in 14 of 102
/// templates.
pub(crate) fn duplicate(form: &mut FormDefinition, key: &NodeKey) {
    let path = path_to(form, key);
    let (parent_path, at) = path.split_at(path.len().saturating_sub(1));
    let position = *at.first().expect("the item is not the root");
    let parent = group_at(&mut form.root, parent_path);
    let mut copy = parent
        .items
        .get(position)
        .expect("the path is in range")
        .clone();
    let attribute = terminal_of(&copy).rm_attribute.clone();
    let next = parent
        .items
        .iter()
        .filter(|held| terminal_of(held).rm_attribute == attribute)
        .count();
    let depth = key.steps.len();
    edit_keys(&mut copy, &mut |target: &mut NodeKey| {
        if let Some(step) = target.steps.get_mut(depth.saturating_sub(1)) {
            step.sibling_ordinal = next;
        }
    });
    group_at(&mut form.root, parent_path).items.push(copy);
}

/// Swaps two items under the group `parent` names, ordinals included.
pub(crate) fn swap(form: &mut FormDefinition, parent: &NodeKey, left: usize, right: usize) {
    let path = path_to(form, parent);
    let depth = parent.steps.len();
    let holder = group_at(&mut form.root, &path);
    let left_ordinal = terminal_of(holder.items.get(left).expect("in range")).sibling_ordinal;
    let right_ordinal = terminal_of(holder.items.get(right).expect("in range")).sibling_ordinal;
    holder.items.swap(left, right);
    let holder = group_at(&mut form.root, &path);
    for (position, ordinal) in [(left, left_ordinal), (right, right_ordinal)] {
        let item = holder.items.get_mut(position).expect("in range");
        edit_keys(item, &mut |target: &mut NodeKey| {
            if let Some(step) = target.steps.get_mut(depth) {
                step.sibling_ordinal = ordinal;
            }
        });
    }
}

/// Drops the last item under the group `key` names.
pub(crate) fn drop_last_child(form: &mut FormDefinition, key: &NodeKey) {
    let path = path_to(form, key);
    let holder = group_at(&mut form.root, &path);
    holder.items.pop().expect("the group holds an item");
}

/// Pins `name` on the node `key` names, as a revision restating a name does.
pub(crate) fn rename(form: &mut FormDefinition, key: &NodeKey, name: &str) {
    let path = path_to(form, key);
    let (parent_path, at) = path.split_at(path.len().saturating_sub(1));
    let position = *at.first().expect("the item is not the root");
    let depth = key.steps.len().saturating_sub(1);
    let item = group_at(&mut form.root, parent_path)
        .items
        .get_mut(position)
        .expect("the path is in range");
    edit_keys(item, &mut |target: &mut NodeKey| {
        if let Some(step) = target.steps.get_mut(depth) {
            step.pinned_name = Some(name.to_owned());
        }
    });
}

/// Gives the node `key` names another Reference Model class.
pub(crate) fn retype(form: &mut FormDefinition, key: &NodeKey, rm_type: &str) {
    let path = path_to(form, key);
    let (parent_path, at) = path.split_at(path.len().saturating_sub(1));
    let position = *at.first().expect("the item is not the root");
    let depth = key.steps.len().saturating_sub(1);
    let item = group_at(&mut form.root, parent_path)
        .items
        .get_mut(position)
        .expect("the path is in range");
    match *item {
        FormItem::Group(ref mut group) => group.rm_type = RmTypeName::new(rm_type),
        FormItem::Field(ref mut field) => field.rm_type = RmTypeName::new(rm_type),
        _ => panic!("the item is neither a group nor a field"),
    }
    edit_keys(item, &mut |target: &mut NodeKey| {
        if let Some(step) = target.steps.get_mut(depth) {
            step.rm_type = RmTypeName::new(rm_type);
        }
    });
}

/// The last key step of one item.
fn terminal_of(item: &FormItem) -> KeyStep {
    let key = key_of(item);
    key.terminal().expect("an item is not the root").clone()
}

/// How many steps the key of one item carries.
fn terminal_depth(item: &FormItem) -> usize {
    key_of(item).steps.len()
}

/// The key of one item.
fn key_of(item: &FormItem) -> &NodeKey {
    match *item {
        FormItem::Group(ref group) => &group.key,
        FormItem::Field(ref field) => &field.key,
        _ => panic!("the item is neither a group nor a field"),
    }
}

/// The first key of a form that names more than one node.
///
/// A key ties when the five-part step tuple of every step is shared with a
/// sibling, which the store reports as a positional placement rather than
/// resolving silently.
pub(crate) fn tied_key(form: &FormDefinition) -> Option<NodeKey> {
    let mut author = Author::new(form);
    keys(form).into_iter().find(|key| {
        matches!(
            author.set(key, Layout::new()),
            Ok(Placement::Positional { .. })
        )
    })
}

/// Where the siblings sharing the five-part tuple of `terminal` sit under the
/// group `parent` names.
pub(crate) fn sibling_positions(
    form: &FormDefinition,
    parent: &NodeKey,
    terminal: &KeyStep,
) -> Vec<usize> {
    let path = path_to(form, parent);
    let mut holder = &form.root;
    for step in &path {
        match *holder.items.get(*step).expect("the path is in range") {
            FormItem::Group(ref nested) => holder = nested,
            _ => panic!("the path leads through a field"),
        }
    }
    holder
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| ties(&terminal_of(item), terminal))
        .map(|(position, _)| position)
        .collect()
}

/// Whether two steps agree on all five parts of the key.
fn ties(left: &KeyStep, right: &KeyStep) -> bool {
    left.rm_attribute == right.rm_attribute
        && left.node_id == right.node_id
        && left.archetype_id == right.archetype_id
        && left.rm_type == right.rm_type
        && left.pinned_name == right.pinned_name
}

/// The key of the group holding the node `key` names.
pub(crate) fn parent_key(key: &NodeKey) -> NodeKey {
    NodeKey {
        steps: key
            .steps
            .split_last()
            .expect("the node is not the root")
            .1
            .to_vec(),
        is_positional: false,
    }
}

/// The key of the node whose terminal step carries `node_id`.
pub(crate) fn key_with_node_id(form: &FormDefinition, node_id: &str) -> NodeKey {
    key_where(form, |step| {
        step.node_id
            .as_ref()
            .is_some_and(|code| code.as_str() == node_id)
    })
    .unwrap_or_else(|| panic!("the form holds no node {node_id}"))
}
