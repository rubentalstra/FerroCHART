// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a group or a field is called on the screen.
//!
//! No specification governs this: our own design. A [`Localized`] states its
//! text per language and is absent in a language the template never
//! translated, so a renderer needs a rule for the gap. The rule is: the
//! form's own default language, then whatever language the template DID state
//! it in, then the node's identifier, which is never nothing.
//!
//! The last step matters. A template can leave a node unlabelled, and a
//! blank heading hides a whole group from the person filling the form, so the
//! identifier is shown instead of nothing at all.

use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;

/// The text `localized` states, preferring `language`.
pub(crate) fn text<'t>(localized: &'t Localized, language: &LanguageTag) -> Option<&'t str> {
    localized
        .get(language)
        .filter(|found| stated(found))
        .or_else(|| {
            localized
                .by_language
                .values()
                .map(String::as_str)
                .find(|found| stated(found))
        })
}

/// Whether the template said anything in this text.
fn stated(text: &str) -> bool {
    !text.trim().is_empty()
}

/// What the item `key` names is called, which is never empty.
pub(crate) fn of(localized: &Localized, language: &LanguageTag, key: &NodeKey) -> String {
    if let Some(stated) = text(localized, language) {
        return stated.to_owned();
    }
    identifier(key)
}

/// The identifier of the node a key names.
///
/// openEHR RM Release-1.1.0 `common.html` section 3.2.2 makes
/// `archetype_node_id` the archetype identifier at an archetype root and the
/// node's own code below it, and a key step carries both, so this is the same
/// pair of facts a path predicate would show.
fn identifier(key: &NodeKey) -> String {
    let Some(step) = key.terminal() else {
        return "The form".to_owned();
    };
    if let Some(archetype) = step.archetype_id.as_ref() {
        return archetype.as_str().to_owned();
    }
    if let Some(node_id) = step.node_id.as_ref() {
        return node_id.as_str().to_owned();
    }
    step.rm_attribute.as_str().to_owned()
}

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{ArchetypeId, LanguageTag, LocalCode, RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};
    use ferrochart_form::text::Localized;

    use super::{of, text};

    fn english() -> LanguageTag {
        LanguageTag::new("en")
    }

    fn step(node_id: Option<&str>, archetype_id: Option<&str>) -> KeyStep {
        KeyStep {
            rm_attribute: RmAttributeName::new("items"),
            node_id: node_id.map(LocalCode::new),
            archetype_id: archetype_id.map(ArchetypeId::new),
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        }
    }

    fn key(node_id: Option<&str>, archetype_id: Option<&str>) -> NodeKey {
        NodeKey::root().child(step(node_id, archetype_id))
    }

    #[test]
    fn the_forms_own_language_wins_where_the_template_states_it() {
        let mut stated = Localized::empty();
        stated.insert(LanguageTag::new("de"), "Systolisch");
        stated.insert(english(), "Systolic");
        assert_eq!(text(&stated, &english()), Some("Systolic"));
    }

    #[test]
    fn a_language_the_template_never_translated_falls_back_to_one_it_did() {
        let stated = Localized::in_language(LanguageTag::new("de"), "Systolisch");
        assert_eq!(text(&stated, &english()), Some("Systolisch"));
    }

    #[test]
    fn an_unlabelled_node_shows_its_identifier_rather_than_nothing() {
        assert_eq!(
            of(&Localized::empty(), &english(), &key(Some("at0004"), None)),
            "at0004"
        );
    }

    #[test]
    fn an_archetype_root_shows_the_archetype_it_is_the_root_of() {
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(None, Some("openEHR-EHR-OBSERVATION.blood_pressure.v2"))
            ),
            "openEHR-EHR-OBSERVATION.blood_pressure.v2"
        );
    }

    #[test]
    fn a_node_with_neither_shows_the_attribute_it_sits_under() {
        assert_eq!(
            of(&Localized::empty(), &english(), &key(None, None)),
            "items"
        );
    }

    #[test]
    fn the_root_of_the_form_is_never_nameless() {
        assert!(!of(&Localized::empty(), &english(), &NodeKey::root()).is_empty());
    }

    #[test]
    fn a_label_stated_as_whitespace_is_treated_as_unstated() {
        // An empty heading hides a whole group, so blank text falls through
        // to the identifier the same way an absent one does.
        let stated = Localized::in_language(english(), "   ");
        assert_eq!(text(&stated, &english()), None);
        assert_eq!(
            of(&stated, &english(), &key(Some("at0004"), None)),
            "at0004"
        );
    }
}
