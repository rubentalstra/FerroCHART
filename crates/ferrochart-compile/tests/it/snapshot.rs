// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Snapshots of the derived form, over the whole vendored corpus.
//!
//! The derivation tests assert totals, which catch that something moved. These
//! catch which template moved and how, which is what makes a model crate bump
//! reviewable rather than merely visible.
//!
//! Two layers, because a whole document for every template would be megabytes
//! nobody reads. The inventory carries one line per template with its group
//! and field counts and the kinds it derives, so a change shows as one changed
//! line naming the template. The full documents cover four templates, so a
//! reviewer can see what actually changed rather than only that a count did.
//!
//! Read a change with `cargo insta review` before accepting it
//! (`.claude/rules/testing.md`). Accepting a snapshot you have not read is how
//! a snapshot suite stops meaning anything.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compile::{adl14, derive};
use ferrochart_form::definition::FormDefinition;

use crate::support::{corpus_dir, templates};

/// The templates whose whole document is snapshotted.
///
/// The grid fixture carries every field kind the derivation produces. The
/// clinical three are real CKM templates an order of magnitude apart in size,
/// so a change that only shows at scale is caught beside one that shows in a
/// small form.
const FULL: &[&str] = &[
    "ferro_datatype_grid.opt",
    "phone-jm-work.opt",
    "csf-blood-cell-count-and-differential.opt",
    "openehr-confirmed-covid-19-infection-report-v0.opt",
];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The derived form for one operational template, or `None` when the reader
/// refuses it.
///
/// The two templates the reader refuses carry a mandatory unfilled slot, so
/// they are absent from the inventory by construction. That absence is itself
/// an assertion: if either starts deriving, a line appears.
fn derived(path: &Path) -> Option<FormDefinition> {
    let xml = fs::read_to_string(path).expect("a corpus file is UTF-8");
    let template = adl14::from_xml(&xml).ok()?;
    Some(derive::form(&template).expect("a template the reader reads derives"))
}

/// One inventory line: what the form has, in a shape a reviewer can read.
fn line(name: &str, form: &FormDefinition) -> String {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fields = 0_usize;
    for field in form.fields() {
        fields += 1;
        *kinds.entry(field.kind.name()).or_default() += 1;
    }
    let mut rendered = format!("{name}  groups={} fields={fields}", form.groups().count());
    for (kind, count) in &kinds {
        let _ = write!(rendered, " {kind}={count}");
    }
    rendered
}

#[test]
fn the_corpus_inventory_is_unchanged() {
    let mut lines: Vec<String> = Vec::new();
    for path in templates() {
        let Some(form) = derived(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a corpus file has a name");
        lines.push(line(name, &form));
    }
    lines.sort();
    insta::assert_snapshot!("corpus_inventory", lines.join("\n"));
}

#[test]
fn the_full_documents_are_unchanged() {
    for name in FULL {
        let source = if name.starts_with("ferro_") {
            fixture_dir().join(name)
        } else {
            corpus_dir().join(name)
        };
        assert!(
            source.exists(),
            "{} is named in the full snapshot set but is not present",
            source.display()
        );
        let form = derived(&source).unwrap_or_else(|| {
            panic!(
                "{} is named in the full snapshot set but does not derive",
                source.display()
            )
        });
        insta::assert_json_snapshot!(name.trim_end_matches(".opt"), form);
    }
}

#[test]
fn deriving_twice_gives_identical_bytes() {
    // The format claims byte-determinism, and every snapshot above rests on
    // it, so this checks the claim rather than trusting it.
    for path in templates().into_iter().take(20) {
        let Some(first) = derived(&path) else {
            continue;
        };
        let second = derived(&path).expect("it derived a moment ago");
        let first = serde_json::to_string(&first).expect("a form definition serializes");
        let second = serde_json::to_string(&second).expect("a form definition serializes");
        assert_eq!(
            first,
            second,
            "{} serialized differently twice",
            path.display()
        );
    }
}
