// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The vendored CKM template pack is present and shaped as advertised.
//!
//! A vendored input is not done until something reads it
//! (`.claude/rules/vendored-inputs.md`). This is the floor: the real reader
//! arrives with the ADL 1.4 path and replaces these assertions with parsing.

use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/templates/ckm")
}

fn templates() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(corpus_dir()) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "opt"))
        .collect();
    found.sort();
    found
}

#[test]
fn the_pack_is_present() {
    let found = templates();
    assert!(
        found.len() >= 100,
        "expected the vendored CKM pack, found {} templates in {}. \
         Run scripts/vendor/ckm-templates.sh",
        found.len(),
        corpus_dir().display()
    );
}

#[test]
fn every_template_is_an_operational_template() {
    for path in templates() {
        let bytes = fs::read(&path).expect("a committed corpus file reads");
        let cut = bytes.len().min(4096);
        let head = String::from_utf8_lossy(&bytes[..cut]).into_owned();
        assert!(
            head.starts_with("<?xml"),
            "{} does not start with an XML declaration",
            path.display()
        );
        assert!(
            head.contains("<template"),
            "{} carries no <template> root",
            path.display()
        );
        // The id sits behind the description, which runs past 4 KB in some
        // exports, so this one reads the whole file.
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("<template_id>"),
            "{} carries no template_id",
            path.display()
        );
    }
}

#[test]
fn every_committed_template_states_a_licence() {
    for path in templates() {
        let text = fs::read_to_string(&path).expect("a committed corpus file is UTF-8");
        assert!(
            text.contains(r#"<other_details id="licence">"#),
            "{} states no licence, so it must not be committed \
             (corpus/templates/ckm/PROVENANCE.md)",
            path.display()
        );
    }
}
