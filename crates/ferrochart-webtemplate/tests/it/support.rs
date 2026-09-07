// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The corpus the compatibility surface is tested against.
//!
//! The web templates read here are built from the committed CKM operational
//! template pack by `openehr_its::flat::webtemplate`, so the documents are
//! real rather than invented. The web template is a compatibility target, so
//! that is exactly the right evidence: the format is what the published
//! implementations write.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

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
        .filter(|path| path.extension().is_some_and(|ext| ext == "opt"))
        .collect();
    found.sort();
    found
}

/// A web template built from every template of the pack that reads, named by
/// the file it came from.
pub(crate) fn web_templates() -> Vec<(String, Value)> {
    let mut built = Vec::new();
    for path in templates() {
        let Ok(xml) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(opt) = openehr_its::opt14::from_xml(&xml) else {
            continue;
        };
        let Ok(web_template) = openehr_its::flat::webtemplate::builder::build_web_template(&opt)
        else {
            continue;
        };
        let Ok(json) = serde_json::to_value(&web_template) else {
            continue;
        };
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        built.push((name, json));
    }
    built
}

/// The first web template of the pack, for a test that needs one document.
///
/// `None` only where the vendored pack is missing, which the caller reports
/// as the failure it is.
pub(crate) fn one_web_template() -> Option<Value> {
    web_templates().into_iter().next().map(|(_, json)| json)
}

/// Walks every node of a web template tree, root first.
pub(crate) fn walk(tree: &Value, visit: &mut dyn FnMut(&Value)) {
    visit(tree);
    if let Some(children) = tree.get("children").and_then(Value::as_array) {
        for child in children {
            walk(child, visit);
        }
    }
}

/// Walks every node of a web template tree so a test can change it.
pub(crate) fn walk_mut(tree: &mut Value, visit: &mut dyn FnMut(&mut Value)) {
    visit(tree);
    if let Some(children) = tree.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            walk_mut(child, visit);
        }
    }
}
