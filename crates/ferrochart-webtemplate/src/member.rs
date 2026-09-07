// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Which members of a web template the form definition carries, and which it
//! does not.
//!
//! The split is the whole preservation contract, so it is stated once here and
//! read by both the reader and the writer. A member named below is recomputed
//! from the form definition when the form definition changed; every other
//! member of the same object is left exactly as the source document spelled
//! it.
//!
//! The web template is a compatibility target, so this list is an observation
//! of the published implementations rather than a conformance requirement.

use serde_json::{Map, Value};

/// The members of the document root the form definition carries.
///
/// `templateId`, `defaultLanguage` and `languages` are the identifying members
/// of [`ferrochart_form::definition::FormDefinition`], and `tree` is its root
/// group. Everything else, `semVer` and `otherDetails` among them, is kept
/// verbatim.
pub const MODELLED_ROOT_MEMBERS: [&str; 4] = ["templateId", "defaultLanguage", "languages", "tree"];

/// The members of a tree node the form definition carries.
///
/// `id`, `aqlPath`, `inContext`, `annotations`, `termBindings`,
/// `cardinalities` and `dependsOn` are deliberately absent: the form
/// definition has no place for any of them, so they are kept verbatim.
pub const MODELLED_NODE_MEMBERS: [&str; 11] = [
    "name",
    "localizedName",
    "rmType",
    "nodeId",
    "min",
    "max",
    "localizedNames",
    "localizedDescriptions",
    "proportionTypes",
    "inputs",
    "children",
];

/// The members of a tree node the form definition carries, where the node
/// became a group.
///
/// `inputs` and `proportionTypes` are absent, because they belong to a leaf
/// and a group has no field to state them from. A node whose class the
/// derivation table has no row for, the `PARTY_PROXY` family a web template
/// states for a composition's context among them, becomes a group and keeps
/// the inputs it arrived with.
pub const MODELLED_GROUP_MEMBERS: [&str; 9] = [
    "name",
    "localizedName",
    "rmType",
    "nodeId",
    "min",
    "max",
    "localizedNames",
    "localizedDescriptions",
    "children",
];

/// The members of an `inputs[]` entry the form definition carries.
///
/// `defaultValue` is absent: the form definition's prefill is a typed value
/// per Reference Model class and a web template input states one untyped JSON
/// value, so mapping one onto the other would be a guess. `listOpen` is
/// absent here and present in [`MODELLED_TEXT_INPUT_MEMBERS`], because the
/// form definition states whether a plain-text enumeration is exhaustive
/// ([`ferrochart_form::field::TextField::options_closed`]) and states nothing
/// about whether a coded or ordinal one is.
pub const MODELLED_INPUT_MEMBERS: [&str; 4] = ["suffix", "type", "list", "validation"];

/// The members of a text `inputs[]` entry the form definition carries.
pub const MODELLED_TEXT_INPUT_MEMBERS: [&str; 5] =
    ["suffix", "type", "list", "listOpen", "validation"];

/// The members of an `inputs[].list[]` entry the form definition carries.
///
/// `termBindings` is absent: the form definition binds a whole value set
/// rather than a single option, so it has no place to put a per-option
/// binding.
pub const MODELLED_OPTION_MEMBERS: [&str; 7] = [
    "value",
    "label",
    "localizedLabels",
    "localizedDescriptions",
    "validation",
    "ordinal",
    "scale",
];

/// `source` with every member of `modelled` replaced by `canonical`.
///
/// A member `canonical` states is written, a modelled member it does not state
/// is removed, and a member outside `modelled` is left exactly as `source`
/// spelled it. That last clause is the preservation rule: a member this crate
/// does not model cannot be touched by a rewrite.
#[must_use]
pub fn patched(
    source: Option<&Map<String, Value>>,
    canonical: &Map<String, Value>,
    modelled: &[&str],
) -> Map<String, Value> {
    let mut out = source.cloned().unwrap_or_default();
    for name in modelled {
        match canonical.get(*name) {
            Some(value) => {
                out.insert((*name).to_owned(), value.clone());
            }
            None => {
                out.remove(*name);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{MODELLED_NODE_MEMBERS, patched};
    use serde_json::json;

    #[test]
    fn a_member_outside_the_modelled_set_survives_a_rewrite() {
        let source = json!({"id": "systolic", "annotations": {"ui": "slider"}, "max": 1});
        let canonical = json!({"max": 4});
        let out = patched(
            source.as_object(),
            canonical.as_object().expect("an object"),
            &MODELLED_NODE_MEMBERS,
        );
        assert_eq!(out.get("annotations"), source.get("annotations"));
        assert_eq!(out.get("id"), source.get("id"));
        assert_eq!(out.get("max"), Some(&json!(4)));
    }

    #[test]
    fn a_modelled_member_the_form_no_longer_states_is_removed() {
        let source = json!({"nodeId": "at0001", "id": "systolic"});
        let out = patched(
            source.as_object(),
            &serde_json::Map::new(),
            &MODELLED_NODE_MEMBERS,
        );
        assert!(out.get("nodeId").is_none());
        assert_eq!(out.get("id"), source.get("id"));
    }
}
