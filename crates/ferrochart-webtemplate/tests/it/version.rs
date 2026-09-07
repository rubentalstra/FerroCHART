// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Telling one template apart from another, without `semVer`.

use ferrochart_webtemplate::document::WebTemplateDocument;
use serde_json::{Value, json};

use crate::support::{one_web_template, web_templates};

#[test]
fn not_one_document_of_the_pack_states_a_sem_ver() {
    // This is the reason version detection never consults it: the whole
    // vendored CKM pack is ADL 1.4, and no document in it states the member.
    let built = web_templates();
    assert!(built.len() >= 100, "expected the vendored CKM pack");
    for (name, json) in built {
        assert!(
            json.get("semVer").is_none_or(Value::is_null),
            "{name} states a semVer, which changes what version detection may rely on"
        );
        let document = WebTemplateDocument::from_value(&json)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        assert_eq!(document.stated_sem_ver(), None);
    }
}

#[test]
fn every_document_of_the_pack_states_a_version_detection_can_use() {
    for (name, json) in web_templates() {
        let document = WebTemplateDocument::from_value(&json)
            .unwrap_or_else(|error| panic!("{name} was refused: {error}"));
        let version = document.source_version();
        assert!(
            !version.template_id.as_str().is_empty(),
            "{name} states no template id"
        );
        let archetype = version
            .root_archetype_id
            .unwrap_or_else(|| panic!("{name} states no root archetype id"));
        assert!(
            archetype.as_str().contains(".v"),
            "{name} states a root archetype id with no version part: {archetype}"
        );
    }
}

#[test]
fn two_revisions_of_the_same_template_are_told_apart() {
    // The pack carries the same summary item at two revisions, and version
    // detection separates them without a semVer between them.
    let built = web_templates();
    let revision = |suffix: &str| {
        let Some((_, json)) = built.iter().find(|(name, _)| name.ends_with(suffix)) else {
            panic!("the pack carries no template ending {suffix}")
        };
        json.clone()
    };
    let first = WebTemplateDocument::from_value(&revision("alcohol-consumption-summary-item-r1"))
        .expect("the first revision");
    let second = WebTemplateDocument::from_value(&revision("alcohol-consumption-summary-item-r2"))
        .expect("the second revision");
    assert_eq!(first.stated_sem_ver(), None);
    assert_eq!(second.stated_sem_ver(), None);
    assert_ne!(
        first.source_version(),
        second.source_version(),
        "two revisions are not told apart"
    );
}

#[test]
fn a_sem_ver_the_document_states_is_kept_and_never_consulted() {
    let mut json = one_web_template().expect("the vendored CKM pack");
    if let Some(root) = json.as_object_mut() {
        root.insert("semVer".to_owned(), json!("2.0.0"));
    }
    let document = WebTemplateDocument::from_value(&json).expect("a document the pack built");
    assert_eq!(document.stated_sem_ver(), Some("2.0.0"));

    let mut without = json.clone();
    if let Some(root) = without.as_object_mut() {
        root.remove("semVer");
    }
    let bare = WebTemplateDocument::from_value(&without).expect("a document the pack built");
    assert_eq!(
        document.source_version(),
        bare.source_version(),
        "removing semVer changed what version detection answered"
    );
    assert_eq!(
        document.write().expect("a document writes").json,
        json,
        "the semVer the document stated did not come back"
    );
}

#[test]
fn the_format_version_is_the_documents_own_and_not_the_templates() {
    let json = one_web_template().expect("the vendored CKM pack");
    let stated = json.get("version").and_then(Value::as_str);
    let document = WebTemplateDocument::from_value(&json).expect("a document the pack built");
    assert_eq!(document.format_version(), stated);
    assert!(
        stated.is_some(),
        "the published implementations state a format version"
    );
}
