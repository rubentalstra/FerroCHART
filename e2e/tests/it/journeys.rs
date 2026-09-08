// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! The browser journeys: what a person reaches, and what each screen drew.
//!
//! Every screen's proof lives in [`crate::screens`], so a journey here is the
//! navigation and nothing else. That is what keeps these tests and the
//! capture pass from drifting: they run the same proof.

use thirtyfour::prelude::*;

use crate::harness::{Page, renderer, session};
use crate::screens::{Screen, Theme, ground, wear};

/// Opens `screen` in a session of its own and runs its proof.
async fn drive(screen: &Screen) {
    let Some(base) = renderer() else {
        return;
    };
    let address = screen.address(&base);
    let name = screen.name();
    let screen = screen.clone();
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let page = Page::open(driver, &name, &address).await;
            screen.prove(&page).await;
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

#[tokio::test]
async fn the_template_library_lists_what_the_server_holds() {
    drive(&Screen::Templates).await;
}

#[tokio::test]
async fn every_form_the_server_holds_draws_its_controls() {
    if renderer().is_none() {
        return;
    }
    let forms = Screen::forms();
    assert!(
        !forms.is_empty(),
        "the battery drives no form, so nothing here proves a template becomes a screen"
    );
    for form in &forms {
        drive(form).await;
    }
}

#[tokio::test]
async fn the_design_system_draws_every_affordance() {
    drive(&Screen::Design).await;
}

/// A reader who starts at the library reaches a form by clicking a row.
///
/// The row's own `href` is read first and matched against the addresses the
/// battery would open directly, so this journey proves two things at once: a
/// form reached by clicking carries the same controls as one reached by
/// typing its address, and the renderer and the battery escape a template
/// identifier the same way.
#[tokio::test]
async fn a_row_in_the_library_leads_to_the_form_it_names() {
    let Some(base) = renderer() else {
        return;
    };
    let library = Screen::Templates;
    let address = library.address(&base);
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let mut page = Page::open(driver, &library.name(), &address).await;
            library.prove(&page).await;
            let row = By::Css("main ul li a[href^='/ui/forms/']");
            let href = page
                .attribute(row.clone(), "href", "the address the first row leads to")
                .await;
            let Some(wanted) = Screen::forms()
                .into_iter()
                .find(|form| form.address("") == href)
            else {
                panic!(
                    "the first row leads to `{href}`, which is none of the addresses this battery opens"
                );
            };
            page.click(row, "the first row of the library").await;
            page.now_on(&wanted.name());
            wanted.prove(&page).await;
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

/// The ground a reader chose survives the next screen they open.
///
/// The theme is a class on the document element held in local storage, so a
/// choice that is not remembered shows up as the next screen opening light
/// again. The capture pass depends on exactly this: it sets the ground once
/// and then walks every screen.
#[tokio::test]
async fn the_ground_a_reader_chose_survives_the_next_screen() {
    let Some(base) = renderer() else {
        return;
    };
    let library = Screen::Templates;
    let design = Screen::Design;
    let first = library.address(&base);
    let next = design.address(&base);
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let mut page = Page::open(driver, &library.name(), &first).await;
            library.prove(&page).await;
            wear(&page, Theme::Dark).await;
            page.now_on(&design.name());
            page.go(&design.name(), &next).await;
            design.prove(&page).await;
            assert_eq!(
                ground(&page).await,
                Theme::Dark.class(),
                "the next screen opened on a ground the reader did not choose"
            );
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

/// No screen speaks openEHR at the person reading it.
///
/// The audience came to build a form and is not required to know the openEHR
/// Reference Model, so a class name or a node code in the text a person reads
/// is a defect (#178). The path and the code stay reachable in the
/// monospaced surface beside the label, and the walk skips that on purpose.
#[tokio::test]
async fn no_screen_names_the_reference_model_at_the_reader() {
    let Some(base) = renderer() else {
        return;
    };
    let mut screens = vec![Screen::Templates, Screen::Design];
    screens.extend(Screen::forms());
    for screen in &screens {
        let address = screen.address(&base);
        let name = screen.name();
        let screen = screen.clone();
        let outcome = session()
            .await
            .run_and_quit(|driver| async move {
                let page = Page::open(driver, &name, &address).await;
                screen.prove(&page).await;
                let found = page.jargon().await;
                assert!(
                    found.is_empty(),
                    "{name} speaks openEHR at the reader: {}",
                    found.join("; ")
                );
                Ok::<(), WebDriverError>(())
            })
            .await;
        outcome.expect("the journey ran and the browser session ended cleanly");
    }
}
