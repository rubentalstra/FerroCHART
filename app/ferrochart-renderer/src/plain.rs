// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Reference Model class names, said the way a person building a form says
//! them.
//!
//! No specification governs this: our own design. The audience is somebody
//! making a form, and they are not required to know the openEHR Reference
//! Model to do it. `DV_COUNT` is a fact about the specification; "a whole
//! number" is the same fact in the reader's language, and the screen owes
//! them the second one.
//!
//! The class name stays available where somebody wants it, the way
//! `docs/architecture.md` section 10.1 keeps the path available beside the
//! label rather than in place of it.

/// What a value of `rm_type` is, in plain words.
///
/// An unknown class returns `None`, because a made-up name for a class this
/// build does not know would be worse than the class name itself.
pub(crate) fn value_kind(rm_type: &str) -> Option<&'static str> {
    Some(match rm_type {
        "DV_BOOLEAN" => "yes or no",
        "DV_TEXT" => "text",
        "DV_CODED_TEXT" | "CODE_PHRASE" => "a coded selection",
        "DV_URI" | "DV_EHR_URI" => "a link",
        "DV_ORDINAL" | "DV_SCALE" => "a scored choice",
        "DV_COUNT" => "a whole number",
        "DV_QUANTITY" => "a measurement",
        "DV_PROPORTION" => "a ratio",
        "DV_DATE" => "a date",
        "DV_TIME" => "a time",
        "DV_DATE_TIME" => "a date and time",
        "DV_DURATION" => "a length of time",
        "DV_IDENTIFIER" => "an identifier",
        "DV_MULTIMEDIA" => "an attachment",
        "DV_PARSABLE" => "structured text",
        "DV_INTERVAL" => "a range",
        "DV_STATE" => "a state",
        _ => return None,
    })
}

/// What a piece of structure is, in plain words.
///
/// The Reference Model distinguishes an `ITEM_TREE` from a `CLUSTER` from a
/// `SECTION`, and a person laying out a form does not care which: all three
/// are a group of fields.
pub(crate) fn structure(rm_type: &str) -> Option<&'static str> {
    Some(match rm_type {
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" | "SECTION" => {
            "a group of fields"
        }
        "ELEMENT" => "a field",
        "HISTORY" | "EVENT" | "POINT_EVENT" | "INTERVAL_EVENT" => "an event",
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" => "an entry",
        _ => return None,
    })
}

/// What `rm_type` is, in plain words, whichever kind it is.
///
/// Falls back to the class name, because a reader who meets one is better
/// served by the specification's own word than by silence.
pub(crate) fn describe(rm_type: &str) -> String {
    value_kind(rm_type)
        .or_else(|| structure(rm_type))
        .map_or_else(|| rm_type.to_owned(), ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{describe, structure, value_kind};

    #[test]
    fn every_field_kind_the_derivation_produces_has_a_plain_name() {
        // The eighteen classes `FieldKind` covers, so a control can always
        // say what it collects without naming the Reference Model.
        for rm_type in [
            "DV_BOOLEAN",
            "DV_TEXT",
            "DV_URI",
            "DV_CODED_TEXT",
            "DV_ORDINAL",
            "DV_SCALE",
            "DV_COUNT",
            "DV_QUANTITY",
            "DV_PROPORTION",
            "DV_DATE",
            "DV_TIME",
            "DV_DATE_TIME",
            "DV_DURATION",
            "DV_IDENTIFIER",
            "DV_MULTIMEDIA",
            "DV_PARSABLE",
            "DV_INTERVAL",
            "DV_STATE",
        ] {
            assert!(value_kind(rm_type).is_some(), "{rm_type} has no plain name");
        }
    }

    #[test]
    fn a_plain_name_never_reads_like_a_class_name() {
        for rm_type in ["DV_COUNT", "ITEM_TREE", "OBSERVATION", "ELEMENT"] {
            let said = describe(rm_type);
            assert!(
                !said.contains('_') && said.chars().any(char::is_lowercase),
                "{rm_type} still reads as a class name: {said}"
            );
        }
    }

    #[test]
    fn a_class_this_build_does_not_know_keeps_its_own_name() {
        // Inventing a name for an unknown class would be worse than showing
        // the one the specification gives it.
        assert_eq!(value_kind("DV_SOMETHING_NEW"), None);
        assert_eq!(structure("SOME_NEW_STRUCTURE"), None);
        assert_eq!(describe("DV_SOMETHING_NEW"), "DV_SOMETHING_NEW");
    }

    #[test]
    fn the_structures_a_person_lays_out_are_all_a_group() {
        for rm_type in ["ITEM_TREE", "ITEM_LIST", "CLUSTER", "SECTION"] {
            assert_eq!(structure(rm_type), Some("a group of fields"), "{rm_type}");
        }
    }
}
