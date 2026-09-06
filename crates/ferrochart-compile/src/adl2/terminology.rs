// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The terminology an ADL 2 operational template carries.
//!
//! openEHR AM Release-2.3.0 `OPT2.html` section 3.4 gathers the flat
//! terminology of every constituent archetype into `component_terminologies`,
//! keyed by archetype identifier, and leaves the root template's own in
//! `terminology`.

use std::collections::BTreeMap;

use openehr_am::v2_4::aom2::archetype::operational_template::OperationalTemplate;
use openehr_am::v2_4::aom2::terminology::archetype_terminology::ArchetypeTerminology;

use crate::model::ids::{ArchetypeId, LanguageTag, LocalCode, TerminologyName};
use crate::model::terminology::{ExternalTerm, TermDefinition, Terminology};

/// Reads every terminology the template carries.
pub(crate) fn read(template: &OperationalTemplate) -> BTreeMap<ArchetypeId, Terminology> {
    let mut built = BTreeMap::new();
    built.insert(
        ArchetypeId::new(template.archetype_id.physical_id()),
        one(&template.terminology),
    );
    for (archetype, terminology) in template.component_terminologies.iter().flatten() {
        built.insert(ArchetypeId::new(archetype.clone()), one(terminology));
    }
    built
}

fn one(source: &ArchetypeTerminology) -> Terminology {
    let mut built = Terminology::new();
    for (language, terms) in &source.term_definitions {
        let language = LanguageTag::new(language.clone());
        for (code, term) in terms {
            built.insert_definition(
                language.clone(),
                LocalCode::new(code.clone()),
                TermDefinition::new(
                    term.text.clone(),
                    Some(term.description.clone()),
                    term.other_items.clone().unwrap_or_default(),
                ),
            );
        }
    }
    for (terminology, entries) in source.term_bindings.iter().flatten() {
        let terminology = TerminologyName::new(terminology.clone());
        for (code, target) in entries {
            let local = LocalCode::new(code.clone());
            let external = ExternalTerm::new(terminology.clone(), target.clone());
            // AOM 2 keeps value-set and node bindings in one map, so the code's
            // own prefix says which of the two a row is.
            if local.kind() == crate::model::ids::CodeKind::ValueSet {
                built.insert_constraint_binding(local, external);
            } else {
                built.insert_binding(local, external);
            }
        }
    }
    for (code, set) in source.value_sets.iter().flatten() {
        built.insert_value_set(
            LocalCode::new(code.clone()),
            set.members.iter().cloned().map(LocalCode::new).collect(),
        );
    }
    built
}
