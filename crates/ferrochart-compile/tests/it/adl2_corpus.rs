// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The ADL 2 reader, over the published archetype library.
//!
//! The library is not committed: of its 652 archetypes one states a licence
//! and the rest state none, so `.gitignore` refuses them and
//! `scripts/vendor/adl2-archetypes.sh` fetches them
//! (`corpus/archetypes/adl2/PROVENANCE.md`).
//!
//! A test over material that may be absent has to choose between failing on a
//! clean checkout and passing silently when the fetch stops happening. It does
//! neither: absence prints what to run and returns, and `FERROCHART_REQUIRE_ADL2`
//! makes absence a failure. CI sets that after fetching, so the assertions run
//! there every time and a fetch that quietly stops is a red build.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compile::adl2;
use openehr_adl::artefact::ArchetypeRepository;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/archetypes/adl2")
}

fn sources(extension: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, extension: &str, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, extension, found);
            } else if path.extension().is_some_and(|e| e == extension) {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&corpus(), extension, &mut found);
    found.sort();
    found
}

/// Whether the library is present, printing what to run when it is not.
#[expect(
    clippy::print_stdout,
    reason = "an integration test tells its reader what to run; clippy.toml's               in-test relaxation covers unit tests only"
)]
fn present(files: &[PathBuf]) -> bool {
    if !files.is_empty() {
        return true;
    }
    assert!(
        std::env::var_os("FERROCHART_REQUIRE_ADL2").is_none(),
        "FERROCHART_REQUIRE_ADL2 is set and the ADL 2 library is absent from {}",
        corpus().display()
    );
    println!(
        "the ADL 2 library is not fetched; run scripts/vendor/adl2-archetypes.sh to exercise \
         this suite ({})",
        corpus().display()
    );
    false
}

/// The archetype identity without its dialect or its patch version.
///
/// ADL 2 spells a three-part version where ADL 1.4 spells one part (openEHR AM
/// Release-2.3.0 `Identification.html`), so a comparison on the bare stem finds
/// nothing.
fn concept(path: &Path) -> String {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    match name.rfind(".v") {
        Some(at) => {
            let (stem, version) = name.split_at(at);
            let major = version
                .trim_start_matches(".v")
                .split('.')
                .next()
                .unwrap_or_default();
            format!("{stem}.v{major}")
        }
        None => name,
    }
}

/// How deep a specialisation an identifier names.
///
/// A specialised archetype's parent has to be in the repository before it, so
/// the pack is loaded shallowest first. openEHR AM Release-2.3.0 `ADL2.html`
/// section 9.2.2: the specialisation level is the number of `.` characters in
/// the node identifier, and the same holds of the archetype identifier's
/// concept part.
fn depth(path: &Path) -> usize {
    concept(path).matches('-').count()
}

#[test]
fn the_reader_reads_the_published_adl2_library() {
    let files = sources("adls");
    if !present(&files) {
        return;
    }
    assert_eq!(files.len(), 322, "the fetched library changed size");

    // Parents first, into one repository, so a specialised child resolves its
    // parent and a slot filler resolves its target.
    let mut ordered = files.clone();
    ordered.sort_by_key(|path| (depth(path), path.clone()));
    let mut repository = ArchetypeRepository::new();
    let mut unparseable = 0_usize;
    for path in &ordered {
        let source = fs::read_to_string(path).expect("a fetched archetype is UTF-8");
        match adl2::parse(&source) {
            Ok(archetype) => repository.insert(archetype),
            Err(_) => unparseable += 1,
        }
    }
    assert_eq!(unparseable, 0, "every published archetype parses");

    let mut read = 0_usize;
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    for path in &ordered {
        let source = fs::read_to_string(path).expect("a fetched archetype is UTF-8");
        let Ok(root) = adl2::parse(&source) else {
            continue;
        };
        match openehr_adl::opt::create_opt(&root, &repository) {
            Ok(operational) => match adl2::read(&operational) {
                Ok(_) => read += 1,
                Err(error) => *refused.entry(head_of(&error.to_string())).or_default() += 1,
            },
            Err(error) => *refused.entry(head_of(&error.to_string())).or_default() += 1,
        }
    }

    // A ratchet over real material, and the two refusal families are counted
    // apart because they belong to different owners.
    let unfilled: usize = refused
        .iter()
        .filter(|(reason, _)| reason.starts_with("requires an unfilled"))
        .map(|(_, count)| count)
        .sum();
    let unflattenable: usize = refused
        .iter()
        .filter(|(reason, _)| reason.starts_with("flattening failed"))
        .map(|(_, count)| count)
        .sum();

    // Ours, and correct: a mandatory slot the template never filled. A form
    // cannot render content the template left undetermined, and omitting it
    // would not satisfy the template either (the adjudication on #13).
    assert_eq!(unfilled, 5, "{refused:#?}");

    // Not ours: `openehr_adl::opt::create_opt` refuses these while flattening a
    // specialised archetype against its parent, because a differential path in
    // the child does not resolve. The pack is a 2013 CKM export that predates
    // the AOM 2 validity rules, and FerroEHR adjudicates the same archetypes
    // in the same pack. FerroCHART records the count rather than working
    // around it: a reader that silently accepted a child whose path does not
    // resolve would compile a form for a shape the parent refuses.
    assert_eq!(unflattenable, 16, "{refused:#?}");

    assert_eq!(
        unfilled + unflattenable,
        refused.values().sum::<usize>(),
        "a new refusal family: {refused:#?}"
    );
    assert_eq!(read, 301);
}

/// The first clause of an error message, so a count groups by reason rather
/// than by the archetype the reason names.
fn head_of(message: &str) -> String {
    message.split_once(" requires ").map_or_else(
        || message.to_owned(),
        |(_, rest)| format!("requires {rest}"),
    )
}

#[test]
fn the_library_carries_the_dual_dialect_pairing() {
    let adls = sources("adls");
    if !present(&adls) {
        return;
    }
    let adl = sources("adl");
    assert_eq!(adl.len(), 330, "the fetched library changed size");

    let two: BTreeSet<String> = adls.iter().map(|p| concept(p)).collect();
    let one: BTreeSet<String> = adl.iter().map(|p| concept(p)).collect();
    let paired = two.intersection(&one).count();

    // The pairing is what makes the library worth fetching: the same clinical
    // archetype in both dialects is the only real input the equivalence
    // property has.
    // TODO(#59): read the 1.4 twins and compare the two internal models. The
    // 1.4 reader takes an operational template rather than an archetype, so
    // the comparison needs a template that composes them.
    assert_eq!(paired, 321, "the dual-dialect pairing changed");
}
