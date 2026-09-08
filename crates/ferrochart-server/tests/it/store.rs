// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The template directory a server compiles at startup.
//!
//! The refusals matter more than the happy path: a server that skipped a
//! template it could not compile would serve fewer forms than its operator
//! installed and say nothing about it.
//!
//! Synthetic content invented for these tests. No patient data.

use std::fs;
use std::path::Path;

use ferrochart_server::store::{StoreError, TemplateStore};

use crate::support;

#[test]
fn a_directory_of_templates_compiles_into_the_forms_it_holds() {
    let directory = support::template_dir("store-load");
    let store = TemplateStore::load(&directory).expect("the committed template compiles");
    assert_eq!(store.len(), 1);
    let held = store
        .get(support::TEMPLATE_ID)
        .expect("the store holds the template by the identifier it states");
    assert_eq!(held.definition.template_id.as_str(), support::TEMPLATE_ID);
    assert_eq!(held.validator.template_id(), support::TEMPLATE_ID);
}

#[test]
fn a_template_that_does_not_parse_fails_the_startup() {
    let directory = support::template_dir("store-broken");
    fs::write(directory.join("broken.opt"), "<not-a-template/>")
        .expect("the broken template is written");

    match TemplateStore::load(&directory) {
        Err(StoreError::Parse { path, .. }) => {
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some("broken.opt")
            );
        }
        Err(other) => panic!("the failure is {other:?}"),
        Ok(store) => panic!(
            "a template that does not parse was skipped, leaving {} held",
            store.len()
        ),
    }
}

#[test]
fn two_files_stating_one_identifier_fail_the_startup() {
    // Serving either one would make which form a request gets depend on the
    // order the filesystem listed the directory in.
    let directory = support::template_dir("store-duplicate");
    support::write_template(&directory, "a-second-copy.opt");

    match TemplateStore::load(&directory) {
        Err(StoreError::Duplicate {
            template_id,
            first,
            second,
        }) => {
            assert_eq!(template_id.as_str(), support::TEMPLATE_ID);
            assert_ne!(first, second);
        }
        Err(other) => panic!("the failure is {other:?}"),
        Ok(_) => panic!("one of the two copies was silently dropped"),
    }
}

#[test]
fn a_directory_that_does_not_exist_fails_the_startup() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-absent");
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the previous run's directory clears");
    }

    match TemplateStore::load(&directory) {
        Err(StoreError::Directory { .. }) => {}
        Err(other) => panic!("the failure is {other:?}"),
        Ok(_) => panic!("a directory that does not exist served an empty store"),
    }
}

#[test]
fn a_directory_holding_no_template_holds_no_form() {
    // An operator who installed nothing gets nothing, which is honest, and it
    // is the same answer an unset `FERROCHART_TEMPLATES` gives.
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("store-empty");
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the previous run's directory clears");
    }
    fs::create_dir_all(&directory).expect("the directory is created");

    let store = TemplateStore::load(&directory).expect("an empty directory is not a failure");
    assert!(store.is_empty());
    assert_eq!(store.ids().count(), 0);
}

#[test]
fn a_file_that_is_not_an_operational_template_is_not_read() {
    // The directory is a mount point, so a README or a checksum file beside
    // the templates is ordinary and is not a template that failed to compile.
    let directory = support::template_dir("store-other-files");
    fs::write(directory.join("README.md"), "# synthetic notes\n").expect("the note is written");

    let store = TemplateStore::load(&directory).expect("the note is not a template");
    assert_eq!(store.len(), 1);
}
