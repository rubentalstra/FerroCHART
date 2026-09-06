// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The terminology an ADL 1.4 operational template carries.
//!
//! An operational template states rubrics in two places. The `ontology` and
//! `component_ontologies` sections carry a per-language
//! `FLAT_ARCHETYPE_ONTOLOGY` for the template and for each constituent
//! archetype. Every `C_ARCHETYPE_ROOT` in the definition also carries its own
//! `term_definitions` and `term_bindings`, in the template's language.
//!
//! NOTE: the ITS-XML `Template.xsd` `C_ARCHETYPE_ROOT` declares
//! `term_definitions` as a bare `ARCHETYPE_TERM` list with no language, while
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 7.3.2 keys every rubric by
//! language through `ARCHETYPE_ONTOLOGY.term_definitions`; the reader reads
//! those rubrics under the template's own language.

use std::collections::BTreeMap;

use openehr_its::opt14::types as opt;

use crate::model::ids::{ArchetypeId, LanguageTag, LocalCode, TerminologyName};
use crate::model::terminology::{ExternalTerm, TermDefinition, Terminology};

/// Reads every ontology section the template carries.
pub(crate) fn from_ontologies(
    template: &opt::OperationalTemplate,
) -> BTreeMap<ArchetypeId, Terminology> {
    let mut built = BTreeMap::new();
    for ontology in template
        .ontology
        .iter()
        .chain(&template.component_ontologies)
    {
        let entry: &mut Terminology = built
            .entry(ArchetypeId::new(ontology.archetype_id.clone()))
            .or_default();
        for set in &ontology.term_definitions {
            let language = LanguageTag::new(set.language.clone());
            for item in &set.items {
                entry.insert_definition(
                    language.clone(),
                    LocalCode::new(item.code.clone()),
                    term(item),
                );
            }
        }
        for set in &ontology.constraint_definitions {
            let language = LanguageTag::new(set.language.clone());
            for item in &set.items {
                entry.insert_definition(
                    language.clone(),
                    LocalCode::new(item.code.clone()),
                    term(item),
                );
            }
        }
        add_term_bindings(entry, &ontology.term_bindings);
        for set in &ontology.constraint_bindings {
            let terminology = TerminologyName::new(set.terminology.clone());
            for item in &set.items {
                entry.insert_constraint_binding(
                    LocalCode::new(item.code.clone()),
                    ExternalTerm::new(terminology.clone(), item.value.clone()),
                );
            }
        }
    }
    built
}

/// Merges the rubrics and bindings an archetype root states inline.
pub(crate) fn add_archetype_root(
    into: &mut BTreeMap<ArchetypeId, Terminology>,
    root: &opt::CArchetypeRoot,
    language: &LanguageTag,
) {
    let entry: &mut Terminology = into
        .entry(ArchetypeId::new(root.archetype_id.value.clone()))
        .or_default();
    for item in &root.term_definitions {
        entry.insert_definition(
            language.clone(),
            LocalCode::new(item.code.clone()),
            term(item),
        );
    }
    add_term_bindings(entry, &root.term_bindings);
}

fn add_term_bindings(entry: &mut Terminology, sets: &[opt::Termbindingset]) {
    for set in sets {
        for item in &set.items {
            // The binding item's own `CODE_PHRASE.terminology_id` is the
            // authoritative terminology; the set's `terminology` attribute
            // repeats it, and the two disagree in real exports.
            let terminology = TerminologyName::new(item.value.terminology_id.value.clone());
            entry.insert_binding(
                LocalCode::new(item.code.clone()),
                ExternalTerm::new(terminology, item.value.code_string.clone()),
            );
        }
    }
}

fn term(item: &opt::ArchetypeTerm) -> TermDefinition {
    let mut other: BTreeMap<String, String> = BTreeMap::new();
    let mut text = String::new();
    let mut description = None;
    for (key, value) in &item.items {
        match key.as_str() {
            "text" => text.clone_from(value),
            "description" => description = Some(value.clone()),
            _ => {
                other.insert(key.clone(), value.clone());
            }
        }
    }
    TermDefinition::new(text, description, other)
}
