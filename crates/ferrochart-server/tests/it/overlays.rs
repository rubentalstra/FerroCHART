// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The overlay directory a server reads at startup, and the route that serves
//! it.
//!
//! The refusals matter more than the happy path, for the same reason they do
//! in [`crate::store`]: a server that skipped a layout it could not read would
//! draw a form in template order and say nothing about the work it dropped.
//!
//! Synthetic content invented for these tests. No patient data.

use std::fs;
use std::path::{Path, PathBuf};

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::group::{FormGroup, FormItem};
use ferrochart_form::ids::LanguageTag;
use ferrochart_form::key::NodeKey;
use ferrochart_form::layout::{FormLayout, Layout, Visibility};
use ferrochart_form::text::Localized;
use ferrochart_overlay::store::{Overlay, Placement};
use ferrochart_server::overlays::{OverlayStore, OverlayStoreError};

use crate::support;

/// The label these tests author, which no archetype rubric would produce.
const AUTHORED: &str = "The name a person wrote over it";

/// A fresh directory named `name`, with nothing in it.
fn empty_dir(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the previous run's directory clears");
    }
    fs::create_dir_all(&directory).expect("the overlay directory is created");
    directory
}

/// The key of the first field the definition holds.
fn first_field(definition: &FormDefinition) -> NodeKey {
    fn walk(group: &FormGroup) -> Option<NodeKey> {
        for item in &group.items {
            match *item {
                FormItem::Field(ref field) => return Some(field.key.clone()),
                FormItem::Group(ref child) => {
                    if let Some(found) = walk(child) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(&definition.root).expect("the committed template derives at least one field")
}

/// An overlay over the committed template, labelling its first field.
fn authored(definition: &FormDefinition) -> Overlay {
    let mut author = ferrochart_overlay::store::Author::new(definition);
    let layout = Layout {
        label: Localized::in_language(LanguageTag::new("en"), AUTHORED),
        visibility: Visibility::Never,
        ..Layout::new()
    };
    let placement = author
        .set(&first_field(definition), layout)
        .expect("the key names a node of the definition");
    assert_eq!(placement, Placement::Unique);
    author.finish()
}

/// Writes `overlay` into `directory` under `name`.
fn write_overlay(directory: &Path, name: &str, overlay: &Overlay) {
    fs::write(
        directory.join(name),
        overlay.to_json().expect("the overlay writes"),
    )
    .expect("the overlay is written");
}

#[test]
fn a_directory_of_overlays_reads_into_the_layouts_it_holds() {
    let (definition, _, _) = support::case();
    let directory = empty_dir("overlays-load");
    write_overlay(&directory, "covid.json", &authored(&definition));

    let store = OverlayStore::load(&directory).expect("the authored overlay reads");
    assert_eq!(store.len(), 1);
    let layout = store
        .get(support::TEMPLATE_ID)
        .expect("the store holds the layout by the template it was authored against");
    assert!(layout.is_current_format());
    assert_eq!(layout.items.len(), 1);
    let authored_layout = layout
        .of(&first_field(&definition))
        .expect("the entry is keyed by the node it decorates");
    assert_eq!(
        authored_layout.label.get(&LanguageTag::new("en")),
        Some(AUTHORED)
    );
    assert_eq!(authored_layout.visibility, Visibility::Never);
}

#[test]
fn a_deployment_that_authored_nothing_holds_no_layout() {
    let directory = empty_dir("overlays-none");
    let store = OverlayStore::load(&directory).expect("an empty directory reads");
    assert!(store.is_empty());
}

#[test]
fn an_overlay_that_does_not_read_fails_the_startup() {
    let directory = empty_dir("overlays-broken");
    fs::write(directory.join("broken.json"), "{\"not\":\"an overlay\"}")
        .expect("the broken overlay is written");

    match OverlayStore::load(&directory) {
        Err(OverlayStoreError::Parse { path, .. }) => {
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some("broken.json")
            );
        }
        Err(other) => panic!("the failure is {other:?}"),
        Ok(store) => panic!(
            "an unreadable overlay was skipped, leaving {} held",
            store.len()
        ),
    }
}

#[test]
fn two_overlays_over_one_template_fail_the_startup() {
    // Serving either one would make which layout a form gets depend on the
    // order the filesystem listed the directory in.
    let (definition, _, _) = support::case();
    let directory = empty_dir("overlays-duplicate");
    let overlay = authored(&definition);
    write_overlay(&directory, "covid.json", &overlay);
    write_overlay(&directory, "covid-again.json", &overlay);

    match OverlayStore::load(&directory) {
        Err(OverlayStoreError::Duplicate {
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
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("overlays-absent");
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("the previous run's directory clears");
    }

    match OverlayStore::load(&directory) {
        Err(OverlayStoreError::Directory { path, .. }) => assert_eq!(path, directory),
        Err(other) => panic!("the failure is {other:?}"),
        Ok(_) => panic!("a directory that does not exist was read as an empty one"),
    }
}

#[tokio::test]
async fn the_layout_route_serves_what_a_person_authored() {
    let cdr = wiremock::MockServer::start().await;
    let (definition, _, _) = support::case();
    let directory = empty_dir("overlays-route");
    write_overlay(&directory, "covid.json", &authored(&definition));
    let running = support::serve_laid_out(
        support::template_dir("overlays-route-templates"),
        Some(directory),
        &cdr.uri(),
    )
    .await;

    let response = reqwest::get(format!(
        "{}/api/templates/{}/layout",
        running.base,
        urlencoding(support::TEMPLATE_ID)
    ))
    .await
    .expect("the layout answers");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let served: FormLayout = response.json().await.expect("the layout reads");
    assert_eq!(served.template_id.as_str(), support::TEMPLATE_ID);
    assert_eq!(served.items.len(), 1);
}

#[tokio::test]
async fn a_template_nobody_laid_out_answers_with_a_layout_that_decides_nothing() {
    // Not a `404`: that status means this server holds no such template, and
    // a client could not tell the two apart if it meant both.
    let cdr = wiremock::MockServer::start().await;
    let running = support::serve(support::template_dir("overlays-bare"), &cdr.uri()).await;

    let response = reqwest::get(format!(
        "{}/api/templates/{}/layout",
        running.base,
        urlencoding(support::TEMPLATE_ID)
    ))
    .await
    .expect("the layout answers");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let served: FormLayout = response.json().await.expect("the layout reads");
    assert!(served.sections.is_empty());
    assert!(served.items.is_empty());
}

#[tokio::test]
async fn a_layout_for_a_template_the_server_does_not_hold_is_not_found() {
    let cdr = wiremock::MockServer::start().await;
    let running = support::serve(support::template_dir("overlays-unknown"), &cdr.uri()).await;

    let response = reqwest::get(format!("{}/api/templates/nothing.v0/layout", running.base))
        .await
        .expect("the layout answers");
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

/// A template identifier as one path segment.
///
/// The committed identifier carries a space, and a raw one would make the URL
/// unparseable rather than making the route answer.
fn urlencoding(id: &str) -> String {
    id.replace(' ', "%20")
}
