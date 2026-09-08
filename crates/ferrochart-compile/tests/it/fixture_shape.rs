// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What every committed operational template fixture in the workspace has to
//! state, whatever it was written to prove.
//!
//! A fixture is an input the whole product is measured against, so a fixture
//! that states a shape no CDR would accept proves something about a shape that
//! cannot occur. Neither of the two shapes below was caught by the derivation
//! tests, because those never validate an instance and the readers are generic
//! over the tree.
//!
//! The check reads the parsed document rather than the internal constraint
//! model, so it also covers a fixture the reader deliberately refuses.
//!
//! Only the ADL 1.4 `.opt` fixtures are read. An ADL 2 archetype identifier
//! carries a three-part version and openEHR AM Release-2.3.0 `ADL2.html`
//! section 7.5.5 deprecates the single-number form there, so the `.adls`
//! fixtures are governed by the opposite rule and are out of scope here.

use std::fs;
use std::path::PathBuf;
use std::str::FromStr as _;

use openehr_base::v1_3::base_types::identification::archetype_id::ArchetypeId;
use openehr_its::opt14;
use openehr_its::opt14::types as opt;

/// The lower bound on the fixtures the walk has to find, so a walk that
/// silently found nothing fails rather than passing over an empty list.
const AT_LEAST: usize = 17;

/// The workspace root, with the `../..` of the manifest path resolved away so
/// a failure names a path a reader can act on.
fn workspace_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}

/// Every committed `.opt` fixture in the workspace, in a stable order.
fn fixtures() -> Vec<PathBuf> {
    let root = workspace_root();
    let entries = fs::read_dir(root.join("crates")).expect("the workspace has a crates directory");
    let mut directories: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("tests/fixtures"))
        .filter(|directory| directory.is_dir())
        .collect();
    directories.sort();

    let mut found = Vec::new();
    for directory in directories {
        let entries = fs::read_dir(&directory).expect("a fixture directory reads");
        let mut here: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "opt"))
            .collect();
        here.sort();
        found.append(&mut here);
    }
    found
}

/// Every fixture, parsed, paired with the workspace-relative path a failure
/// names.
fn parsed() -> Vec<(String, opt::OperationalTemplate)> {
    let root = workspace_root();
    fixtures()
        .into_iter()
        .map(|path| {
            let xml = fs::read_to_string(&path).expect("a committed fixture is UTF-8");
            let shown = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let template = opt14::from_xml(&xml)
                .unwrap_or_else(|error| panic!("{shown} does not parse: {error}"));
            (shown, template)
        })
        .collect()
}

/// The Reference Model type `object` constrains.
///
/// The match is exhaustive on purpose: `opt::CObject` is not
/// `#[non_exhaustive]`, so a variant added upstream stops this file compiling
/// rather than slipping past the check.
fn rm_type(object: &opt::CObject) -> &str {
    match *object {
        opt::CObject::ArchetypeInternalRef(ref o) => &o.rm_type_name,
        opt::CObject::ArchetypeSlot(ref o) => &o.rm_type_name,
        opt::CObject::ConstraintRef(ref o) => &o.rm_type_name,
        opt::CObject::CArchetypeRoot(ref o) => &o.rm_type_name,
        opt::CObject::CCodePhrase(ref o) => &o.rm_type_name,
        opt::CObject::CCodeReference(ref o) => &o.rm_type_name,
        opt::CObject::CComplexObject(ref o) => &o.rm_type_name,
        opt::CObject::CDefinedObject(ref o) => &o.rm_type_name,
        opt::CObject::CDvOrdinal(ref o) => &o.rm_type_name,
        opt::CObject::CDvQuantity(ref o) => &o.rm_type_name,
        opt::CObject::CDvState(ref o) => &o.rm_type_name,
        opt::CObject::CPrimitiveObject(ref o) => &o.rm_type_name,
        opt::CObject::TComplexObject(ref o) => &o.rm_type_name,
    }
}

/// The attributes `object` constrains, where it constrains any.
fn attributes(object: &opt::CObject) -> &[opt::CAttribute] {
    match *object {
        opt::CObject::CArchetypeRoot(ref o) => &o.attributes,
        opt::CObject::CComplexObject(ref o) => &o.attributes,
        opt::CObject::TComplexObject(ref o) => &o.attributes,
        _ => &[],
    }
}

/// The name and children of one attribute slot.
fn parts(attribute: &opt::CAttribute) -> (&str, &[opt::CObject]) {
    match *attribute {
        opt::CAttribute::CSingleAttribute(ref a) => (&a.rm_attribute_name, &a.children),
        opt::CAttribute::CMultipleAttribute(ref a) => (&a.rm_attribute_name, &a.children),
    }
}

/// Applies `visit` to `object` and to every object below it.
fn walk(object: &opt::CObject, visit: &mut impl FnMut(&opt::CObject)) {
    visit(object);
    for attribute in attributes(object) {
        let (_, children) = parts(attribute);
        for child in children {
            walk(child, visit);
        }
    }
}

/// The definition of `template` as a `C_OBJECT`, so the root walks like any
/// other node.
fn definition(template: &opt::OperationalTemplate) -> opt::CObject {
    opt::CObject::CArchetypeRoot(template.definition.clone())
}

#[test]
fn the_walk_finds_the_committed_fixtures() {
    let found = fixtures();
    assert!(
        found.len() >= AT_LEAST,
        "expected at least {AT_LEAST} committed .opt fixtures, found {}",
        found.len()
    );
}

#[test]
fn every_archetype_id_states_a_single_version_number() {
    // openEHR BASE Release-1.2.0 `base_types.html` section 5.5:
    // `archetype_id = qualified_rm_entity, '.', domain_concept, '.',
    // version_id` with `version-id = 'v', ( '0' | non-zero-digit, [ number ] )`.
    // Section 5.4.11 leaves `TEMPLATE_ID` with no lexical form at all, which is
    // why only the archetype identifier is held to one: the two sit next to
    // each other in an operational template, and the three-part form belongs
    // to the template identifier.
    let mut wrong = Vec::new();
    for (path, template) in parsed() {
        let mut stated = Vec::new();
        walk(&definition(&template), &mut |object| {
            if let opt::CObject::CArchetypeRoot(ref root) = *object {
                stated.push(root.archetype_id.value.clone());
            }
        });
        for ontology in template
            .ontology
            .iter()
            .chain(&template.component_ontologies)
        {
            stated.push(ontology.archetype_id.clone());
        }
        for value in stated {
            if ArchetypeId::from_str(&value).is_err() {
                wrong.push(format!("{path}: {value}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "an archetype_id is not in the BASE Release-1.2.0 section 5.5 lexical \
         form (rm_originator-rm_name-rm_entity.domain_concept.vN):\n{}",
        wrong.join("\n")
    );
}

#[test]
fn an_observation_states_its_data_and_its_state_as_a_history() {
    // openEHR RM Release-1.1.0 `ehr.html` section 8.3.4 types both
    // `OBSERVATION.data` and `OBSERVATION.state` as `HISTORY<ITEM_STRUCTURE>`,
    // so an `ITEM_STRUCTURE` cannot sit directly under either: a `HISTORY`,
    // with its `1..1 origin`, comes between. A template that constrains
    // neither attribute states nothing wrong and is left alone.
    let mut wrong = Vec::new();
    for (path, template) in parsed() {
        walk(&definition(&template), &mut |object| {
            if rm_type(object) != "OBSERVATION" {
                return;
            }
            for attribute in attributes(object) {
                let (name, children) = parts(attribute);
                if name != "data" && name != "state" {
                    continue;
                }
                for child in children {
                    if rm_type(child) != "HISTORY" {
                        wrong.push(format!(
                            "{path}: OBSERVATION.{name} holds a {}",
                            rm_type(child)
                        ));
                    }
                }
            }
        });
    }
    assert!(
        wrong.is_empty(),
        "an OBSERVATION states a data or state attribute the Reference Model \
         types as HISTORY<ITEM_STRUCTURE> (RM Release-1.1.0 `ehr.html` \
         section 8.3.4):\n{}",
        wrong.join("\n")
    );
}

#[test]
fn a_history_states_its_events_as_events() {
    // openEHR RM Release-1.1.0 `data_structures.html` section 6.2.1 types
    // `HISTORY.events` as `List<EVENT>`, and section 6.2.2 makes `EVENT`
    // abstract with `POINT_EVENT` (6.2.3) and `INTERVAL_EVENT` (6.2.4) as its
    // concrete subtypes.
    const EVENTS: [&str; 3] = ["EVENT", "POINT_EVENT", "INTERVAL_EVENT"];
    let mut wrong = Vec::new();
    for (path, template) in parsed() {
        walk(&definition(&template), &mut |object| {
            if rm_type(object) != "HISTORY" {
                return;
            }
            for attribute in attributes(object) {
                let (name, children) = parts(attribute);
                if name != "events" {
                    continue;
                }
                for child in children {
                    if !EVENTS.contains(&rm_type(child)) {
                        wrong.push(format!("{path}: HISTORY.events holds a {}", rm_type(child)));
                    }
                }
            }
        });
    }
    assert!(
        wrong.is_empty(),
        "a HISTORY states an events attribute the Reference Model types as \
         List<EVENT> (RM Release-1.1.0 `data_structures.html` \
         section 6.2.1):\n{}",
        wrong.join("\n")
    );
}
