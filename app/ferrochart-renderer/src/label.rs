// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a group or a field is called on the screen.
//!
//! No specification governs this: our own design. A [`Localized`] states its
//! text per language and is absent in a language the template never
//! translated, so a renderer needs a rule for the gap. The rule is: the
//! form's own default language, then whatever language the template DID state
//! it in, then what the node IS, which is never nothing.
//!
//! The last step matters, and it is where the audience decides the design. A
//! template can leave a node unlabelled, and `at0004` is a true answer that
//! tells a person building a form nothing. At an archetype root the
//! identifier carries a concept name, so the fallback says the concept; below
//! one it says what the node collects and keeps the code beside it, because
//! two fields in a group would otherwise share one name.

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
///
/// `collects` is what the item holds, in plain words, and is what the name
/// falls back to where the template states no label and the node carries no
/// concept of its own.
pub(crate) fn of(
    localized: &Localized,
    language: &LanguageTag,
    key: &NodeKey,
    collects: &str,
) -> String {
    if let Some(stated) = text(localized, language) {
        return stated.to_owned();
    }
    unlabelled(key, collects)
}

/// What to call a node the template never labelled.
///
/// openEHR RM Release-1.1.0 `common.html` section 3.2.2 makes
/// `archetype_node_id` the archetype identifier at an archetype root and the
/// node's own code below it, and a key step carries both. An archetype
/// identifier names a concept (openEHR BASE Release-1.2.0 `base_types.html`
/// section 5.4.10), so a root can be called by it; a node code names nothing,
/// so the node is called by what it collects and the code goes in brackets to
/// keep two siblings apart.
fn unlabelled(key: &NodeKey, collects: &str) -> String {
    let Some(step) = key.terminal() else {
        return "The form".to_owned();
    };
    if let Some(archetype) = step.archetype_id.as_ref() {
        // An identifier this build cannot read keeps its own text, because a
        // root shown as "A group of fields" would be one of many with that
        // name.
        return crate::plain::concept(archetype.as_str())
            .unwrap_or_else(|| archetype.as_str().to_owned());
    }
    let said = if collects.trim().is_empty() {
        crate::plain::describe(step.rm_type.as_str())
    } else {
        collects.to_owned()
    };
    let apart = step
        .node_id
        .as_ref()
        .map_or_else(|| step.rm_attribute.as_str(), |node_id| node_id.as_str());
    format!("{} ({apart})", crate::plain::sentence_case(&said))
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
    fn an_unlabelled_node_says_what_it_collects_and_keeps_its_code() {
        // `at0004` is a true answer that tells a person building a form
        // nothing, so the name leads with the value and keeps the code, which
        // is what tells two siblings apart.
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(Some("at0004"), None),
                "a whole number"
            ),
            "A whole number (at0004)"
        );
    }

    #[test]
    fn an_unlabelled_node_with_nothing_to_say_falls_back_to_its_class() {
        // The class name is the last resort, never the first.
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(Some("at0004"), None),
                ""
            ),
            "A field (at0004)"
        );
    }

    #[test]
    fn an_archetype_root_is_called_by_the_concept_it_names() {
        // openEHR BASE Release-1.2.0 `base_types.html` section 5.4.10.
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(None, Some("openEHR-EHR-OBSERVATION.blood_pressure.v2")),
                "a group of fields"
            ),
            "Blood pressure"
        );
    }

    #[test]
    fn an_archetype_identifier_this_build_cannot_read_keeps_its_own_text() {
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(None, Some("not-an-archetype-id")),
                "a group of fields"
            ),
            "not-an-archetype-id"
        );
    }

    #[test]
    fn a_node_with_neither_is_told_apart_by_the_attribute_it_sits_under() {
        assert_eq!(
            of(
                &Localized::empty(),
                &english(),
                &key(None, None),
                "a whole number"
            ),
            "A whole number (items)"
        );
    }

    #[test]
    fn the_root_of_the_form_is_never_nameless() {
        assert!(!of(&Localized::empty(), &english(), &NodeKey::root(), "").is_empty());
    }

    #[test]
    fn a_label_stated_as_whitespace_is_treated_as_unstated() {
        // An empty heading hides a whole group, so blank text falls through
        // the same way an absent one does.
        let stated = Localized::in_language(english(), "   ");
        assert_eq!(text(&stated, &english()), None);
        assert_eq!(
            of(&stated, &english(), &key(Some("at0004"), None), "a date"),
            "A date (at0004)"
        );
    }
}
