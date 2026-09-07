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
//!
//! # The dual-dialect pairing, and what it can prove
//!
//! 321 of the archetypes are published in both dialects, and that pairing is
//! the only real material the equivalence property of `docs/architecture.md`
//! section 3 could have had. It cannot have it, for two reasons found by
//! measuring the pack rather than assuming.
//!
//! **The ADL 1.4 reader cannot be pointed at the 1.4 half.**
//! [`ferrochart_compile::adl14`] reads an operational template in the ITS-XML
//! 2.0.0 `Template.xsd` serialisation, and the twins are ADL 1.4 text. Nothing
//! in the pinned crate set turns one into the other: `openehr_its` parses OPT
//! 1.4 XML and AOM 2 XML and has no ADL 1.4 text parser, `openehr_adl` parses
//! ADL 1.4 text only as the front end of its own ADL 1.4 to ADL 2 converter,
//! whose own documentation records that no openEHR specification governs that
//! conversion, and the upstream tree publishes no XML and no `.opt` at all.
//! Reading a twin through that converter would run the ADL 2 reader twice over
//! a third party's conversion and say nothing about the ADL 1.4 reader, and
//! writing the conversion here would validate a reader against our own output.
//!
//! **The two halves are not two authorings.** Every one of the 321 ADL 2 files
//! carries the `generated` marker in its archetype header, which openEHR AM
//! Release-2.3.0 `ADL2.html` section 7.5 §Generated Indicator defines as the
//! flag for an ADL 2 artefact generated from a flat ADL 1.4 one. So the pack
//! holds one authoring and a conversion of it, and a full structural
//! comparison would be a comparison against that converter: it drops the
//! rubric of an RM-structure node, synthesises node identifiers with a
//! placeholder rubric, and in a long tail of archetypes rewrites a rubric
//! outright.
//!
//! What survives both facts is the narrow property
//! [`the_dual_dialect_pairs_agree_on_what_both_dialects_state`] asserts, and
//! it is stated there with what it does not prove.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_compile::adl2;
use ferrochart_compile::error::ReadError;
use ferrochart_compile::model::node::ConstraintTemplate;
use openehr_adl::artefact::ArchetypeRepository;
use openehr_adl::parse::Dialect;
use openehr_am::v2_4::aom2::archetype::archetype::Archetype;
use openehr_am::v2_4::aom2::archetype::authored_archetype::AuthoredArchetype;
use openehr_am::v2_4::aom2::constraint_model::c_object::CObject;
use openehr_am::v2_4::aom2::terminology::archetype_terminology::ArchetypeTerminology;

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

/// One published archetype, in whichever dialect, reduced to the three facts
/// both dialects state and neither the pack's conversion nor the flattening
/// can move.
#[derive(Debug, PartialEq, Eq)]
struct Stated {
    /// The language the archetype is authored in.
    language: String,
    /// The Reference Model class the root node constrains.
    rm_type: String,
    /// The rubric the archetype gives its own concept.
    concept: String,
}

/// The terminology of an assembled archetype, whichever subtype it is.
fn terminology_of(archetype: &Archetype) -> &ArchetypeTerminology {
    match *archetype {
        Archetype::AuthoredArchetype(ref authored) => match **authored {
            AuthoredArchetype::AuthoredArchetype(ref data) => &data.terminology,
            AuthoredArchetype::Template(ref template) => &template.terminology,
            AuthoredArchetype::OperationalTemplate(ref template) => &template.terminology,
        },
        Archetype::TemplateOverlay(ref overlay) => &overlay.terminology,
    }
}

/// The root object of an assembled archetype, whichever subtype it is.
fn definition_of(archetype: &Archetype) -> CObject {
    let definition = match *archetype {
        Archetype::AuthoredArchetype(ref authored) => match **authored {
            AuthoredArchetype::AuthoredArchetype(ref data) => &data.definition,
            AuthoredArchetype::Template(ref template) => &template.definition,
            AuthoredArchetype::OperationalTemplate(ref template) => &template.definition,
        },
        Archetype::TemplateOverlay(ref overlay) => &overlay.definition,
    };
    CObject::CComplexObject(definition.clone())
}

/// What the ADL 1.4 twin states, read by `openehr_adl`'s ADL 1.4 front end.
///
/// The front end is a parse and not the conversion that follows it: the
/// concept code, the rubrics and the root Reference Model type come back as
/// the 1.4 source spells them.
fn stated_by_adl14(source: &str) -> Result<Stated, String> {
    let archetype = openehr_adl::assemble::parse_artefact(source, Dialect::Adl14)
        .map_err(|errors| format!("the ADL 1.4 twin does not parse: {errors:?}"))?;
    let terminology = terminology_of(&archetype);
    let language = terminology.original_language.clone();
    let concept = terminology
        .term_definitions
        .get(&language)
        .and_then(|definitions| definitions.get(&terminology.concept_code))
        .ok_or_else(|| {
            format!(
                "the ADL 1.4 twin defines no rubric for {}",
                terminology.concept_code
            )
        })?
        .text
        .clone();
    Ok(Stated {
        language,
        rm_type: openehr_adl::aom::access::object_rm_type(&definition_of(&archetype)).to_owned(),
        concept,
    })
}

/// What the ADL 2 half states, read through the internal constraint model.
fn stated_by_the_model(model: &ConstraintTemplate) -> Result<Stated, String> {
    let root = model.root();
    let code = root
        .identity()
        .node_id()
        .ok_or_else(|| "the root node carries no code".to_owned())?;
    let concept = model
        .terminology(root.terminology_scope())
        .and_then(|terminology| terminology.rubric(model.language(), code))
        .ok_or_else(|| format!("the model defines no rubric for {code}"))?
        .text()
        .to_owned();
    Ok(Stated {
        language: model.language().as_str().to_owned(),
        rm_type: root.identity().rm_type().as_str().to_owned(),
        concept,
    })
}

/// Every pair in the library agrees on the language, the root Reference Model
/// type and the concept rubric.
///
/// **What this proves.** For every archetype the library publishes in both
/// dialects and this reader can read, the internal constraint model the ADL 2
/// reader fills names the same clinical concept, in the same language, over
/// the same Reference Model class as the published ADL 1.4 twin. The three
/// facts are the ones both dialects state the same way, and the ones the
/// pack's own 1.4 to 2 conversion and the ADL 2 flattening both leave alone.
///
/// **What it does not prove.** It does not exercise
/// [`ferrochart_compile::adl14`] at all: the 1.4 half is read by
/// `openehr_adl`'s ADL 1.4 front end, for the reason the module doc records.
/// So it is not the equivalence of `docs/architecture.md` section 3, which
/// only the hand-authored pair in `matched_pair.rs` states, and it says
/// nothing about node structure, occurrences or constraint payloads.
#[test]
fn the_dual_dialect_pairs_agree_on_what_both_dialects_state() {
    let adls = sources("adls");
    if !present(&adls) {
        return;
    }
    let adl = sources("adl");
    assert_eq!(adl.len(), 330, "the fetched library changed size");

    let mut two: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in &adls {
        two.insert(concept(path), path.clone());
    }
    let mut one: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in &adl {
        one.insert(concept(path), path.clone());
    }
    let paired: Vec<(&String, &PathBuf)> = two
        .iter()
        .filter(|(identity, _)| one.contains_key(*identity))
        .collect();

    // The pairing is what makes the library worth fetching, and it is asserted
    // before it is used so a pack that lost its 1.4 half cannot pass by
    // comparing nothing.
    assert_eq!(paired.len(), 321, "the dual-dialect pairing changed");

    let mut repository = ArchetypeRepository::new();
    let mut ordered = adls.clone();
    ordered.sort_by_key(|path| (depth(path), path.clone()));
    for path in &ordered {
        let source = fs::read_to_string(path).expect("a fetched archetype is UTF-8");
        if let Ok(archetype) = adl2::parse(&source) {
            repository.insert(archetype);
        }
    }

    let mut compared = 0_usize;
    let mut generated = 0_usize;
    let mut unreadable: BTreeMap<String, usize> = BTreeMap::new();
    let mut disagreed: Vec<String> = Vec::new();
    for (identity, two_path) in paired {
        let one_path = &one[identity];
        let from_adl14 =
            stated_by_adl14(&fs::read_to_string(one_path).expect("a fetched archetype is UTF-8"))
                .unwrap_or_else(|reason| panic!("{identity}: {reason}"));

        let source = fs::read_to_string(two_path).expect("a fetched archetype is UTF-8");
        if openehr_adl::source::parse_source(&source, Dialect::Adl2)
            .is_ok_and(|artefact| artefact.meta.generated)
        {
            generated += 1;
        }
        let read = adl2::parse(&source).and_then(|root| {
            openehr_adl::opt::create_opt(&root, &repository)
                .map_err(ReadError::from)
                .and_then(|operational| adl2::read(&operational))
        });
        let model = match read {
            Ok(model) => model,
            Err(error) => {
                *unreadable.entry(head_of(&error.to_string())).or_default() += 1;
                continue;
            }
        };
        compared += 1;
        let from_adl2 =
            stated_by_the_model(&model).unwrap_or_else(|reason| panic!("{identity}: {reason}"));
        if from_adl14 != from_adl2 {
            disagreed.push(format!(
                "{identity}: 1.4 states {from_adl14:?}, ADL 2 {from_adl2:?}"
            ));
        }
    }

    assert!(disagreed.is_empty(), "{}", disagreed.join("\n"));

    // The module doc rests on this: the ADL 2 half of every pair declares
    // itself generated, so the pack is one authoring and a conversion of it.
    assert_eq!(
        generated, 321,
        "the pack stopped declaring itself generated"
    );

    // The 21 pairs left out are the ADL 2 halves the reader refuses, and they
    // are the same two families `the_reader_reads_the_published_adl2_library`
    // adjudicates. Counting them here keeps a refusal from quietly shrinking
    // the comparison.
    assert_eq!(unreadable.values().sum::<usize>(), 21, "{unreadable:#?}");
    assert!(
        unreadable
            .keys()
            .all(|reason| reason.starts_with("requires an unfilled")
                || reason.starts_with("the operational template could not be generated")),
        "a new refusal family: {unreadable:#?}"
    );
    assert_eq!(compared, 300);
}
