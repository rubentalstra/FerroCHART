// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The name a template pins on a node.
//!
//! openEHR RM Release-1.1.0 `common.html` gives every `LOCATABLE` a `name` of
//! type `DV_TEXT`. A template pins it by constraining `name/value` to a single
//! string, and that pinned name is the strongest discriminator between sibling
//! nodes that share a node identifier. It is a definition fact the template
//! states, not a predicate matched against data.

use openehr_its::opt14::types as opt;

/// The name the template pins on `object`, where it states exactly one.
///
/// A constrained pattern, an open list, or a list of more than one string
/// leaves the name unpinned: the template admits several names, so none of
/// them identifies the node.
pub(crate) fn pinned(object: &opt::CObject) -> Option<String> {
    let name_children = super::path::attributes(object)
        .iter()
        .find_map(|attribute| {
            let (attribute_name, children) = super::path::attribute_parts(attribute);
            (attribute_name == "name").then_some(children)
        })?;
    let name_object = name_children.first()?;
    let value_children = super::path::attributes(name_object)
        .iter()
        .find_map(|attribute| {
            let (attribute_name, children) = super::path::attribute_parts(attribute);
            (attribute_name == "value").then_some(children)
        })?;
    let opt::CObject::CPrimitiveObject(ref primitive) = *value_children.first()? else {
        return None;
    };
    let opt::CPrimitive::CString(ref string) = *primitive.item.as_deref()? else {
        return None;
    };
    match string.list.as_slice() {
        [only] if string.pattern.is_none() && string.list_open != Some(true) => Some(only.clone()),
        _ => None,
    }
}
