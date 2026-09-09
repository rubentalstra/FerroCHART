// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! What a published artefact has to do, whichever release it is.
//!
//! The rest of the battery drives a build of this tree and asserts what this
//! tree does. These two drive whatever image they are pointed at, which is
//! usually the last release, so they assert only what every release has to
//! carry: the shell answers under `/ui`, the library lists the templates the
//! deployment was given, and each of those templates draws a form with
//! controls in it.
//!
//! Anything narrower would be a promise about a release that had not been cut
//! when the journey was written, and the lane would go red on the day it was
//! most useful (issue #166).

use thirtyfour::prelude::*;

use crate::harness::{Page, renderer, session};
use crate::screens::Screen;

#[tokio::test]
async fn a_published_artefact_serves_the_template_library() {
    let Some(base) = renderer() else {
        return;
    };
    let library = Screen::Templates;
    let address = library.address(&base);
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let page = Page::open(driver, &library.name(), &address).await;
            library.prove(&page).await;
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

#[tokio::test]
async fn a_published_artefact_draws_every_form_it_was_given() {
    let Some(base) = renderer() else {
        return;
    };
    let forms = Screen::forms();
    assert!(
        !forms.is_empty(),
        "the battery drives no form, so nothing here proves the artefact renders one"
    );
    for form in forms {
        let address = form.address(&base);
        let name = form.name();
        let outcome = session()
            .await
            .run_and_quit(|driver| async move {
                let page = Page::open(driver, &name, &address).await;
                form.prove(&page).await;
                Ok::<(), WebDriverError>(())
            })
            .await;
        outcome.expect("the journey ran and the browser session ended cleanly");
    }
}
