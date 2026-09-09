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

/// The form grows as it is answered, because a person laid it out that way.
///
/// `e2e/overlays/family-history-summary-item-r2.json` is the one authored
/// layout this repository serves. It renames the deceased question, moves the
/// alias to the end, and hides the two death questions behind the answer to
/// it. No specification governs any of that: it is what the overlay is for,
/// and until issue #201 the browser read none of it.
///
/// The journey is the proof that the three arrive: the authored name is on
/// screen, the death questions are not, and answering yes brings them in.
#[tokio::test]
async fn a_form_the_overlay_lays_out_grows_as_it_is_answered() {
    let Some(base) = renderer() else {
        return;
    };
    let Some(screen) = Screen::forms()
        .into_iter()
        .find(|form| form.stem().is_some_and(|stem| stem == LAID_OUT))
    else {
        // The battery drives a template set the script names, and a run that
        // does not include this one has nothing to prove here.
        return;
    };
    let address = screen.address(&base);
    let name = screen.name();
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let page = Page::open(driver, &name, &address).await;
            screen.prove(&page).await;

            // The authored name, which no archetype rubric produces.
            page.element(
                By::XPath(format!("//main//span[normalize-space()='{DECEASED}']")),
                "the question under the name a person wrote over it",
            )
            .await;

            // The two questions the rule governs, before it holds.
            for hidden in GATED {
                page.none(
                    By::XPath(format!("//main//span[normalize-space()='{hidden}']")),
                    &format!("`{hidden}`, which the layout hides until the answer calls for it"),
                )
                .await;
            }

            // The authored order: the alias sits after the comment, and the
            // template puts it second.
            let labels = page.texts(By::Css("main span.block.font-medium")).await;
            let place = |wanted: &str| labels.iter().position(|read| read == wanted);
            if let (Some(alias), Some(comment)) = (place("Alias"), place("Comment")) {
                assert!(
                    alias > comment,
                    "the alias is drawn at {alias} and the comment at {comment}, so the authored order did not reach the browser"
                );
            }

            page.click(
                By::XPath(format!(
                    "//main//fieldset[legend[normalize-space()='{DECEASED}']]\
                     //input[@type='checkbox']"
                )),
                "the answer the rule reads",
            )
            .await;

            for shown in GATED {
                page.element(
                    By::XPath(format!("//main//span[normalize-space()='{shown}']")),
                    &format!("`{shown}`, which the answer brings in"),
                )
                .await;
            }
            page.quiet().await;
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

/// The template the committed overlay was authored over.
const LAID_OUT: &str = "family-history-summary-item-r2";

/// The label the overlay writes over the deceased question.
const DECEASED: &str = "Has this family member died?";

/// The questions the overlay hides until that one is answered yes.
const GATED: [&str; 2] = ["Age at death", "Date of death"];

/// A zone is not an offset, and the form resolves it at the instant entered.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 7.2.4 types
/// `DV_DATE_TIME` on `Iso8601_date_time`, so what a value carries is `Z` or
/// `±hh:mm` and never a zone name. Europe/Amsterdam is `+01:00` in January and
/// `+02:00` in July, so one zone and two dates have to produce two offsets, and
/// that conversion is the thing a clinician should not be doing in their head
/// (issue #197).
///
/// The resolution runs against the browser's own IANA database through
/// `Intl.DateTimeFormat`, so only a browser can prove it.
#[tokio::test]
async fn one_zone_and_two_dates_resolve_to_two_offsets() {
    let Some(base) = renderer() else {
        return;
    };
    let Some(screen) = Screen::forms()
        .into_iter()
        .find(|form| form.stem().is_some_and(|stem| stem == LAID_OUT))
    else {
        return;
    };
    let address = screen.address(&base);
    let name = screen.name();
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let page = Page::open(driver, &name, &address).await;
            screen.prove(&page).await;

            // The date of birth, which the template admits at any precision
            // and with a timezone beside it.
            let under = format!("//main//div[div/span[normalize-space()='{DATED}']]");
            let part = |name: &str| {
                By::XPath(format!(
                    "{under}//label[normalize-space()='{name}']/following-sibling::input"
                ))
            };
            let zone = By::XPath(format!("{under}//select"));
            let resolved = By::XPath(format!("{under}//span[contains(text(), 'at that moment')]"));

            page.fill(part("Year"), "2026", "the year of the date of birth")
                .await;
            page.fill(part("Month"), "1", "the month of the date of birth")
                .await;
            page.fill(part("Day"), "15", "the day of the date of birth")
                .await;
            page.choose(zone, ZONE, "the timezone beside the date of birth")
                .await;
            let winter = page
                .text_becoming(resolved.clone(), "+01:00", "the offset in force in January")
                .await;

            page.fill(part("Month"), "7", "the month of the date of birth")
                .await;
            let summer = page
                .text_becoming(resolved, "+02:00", "the offset in force in July")
                .await;

            assert_ne!(
                winter, summer,
                "one zone produced one offset for both halves of the year"
            );
            page.quiet().await;
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the journey ran and the browser session ended cleanly");
}

/// The field whose timezone the journey above resolves.
const DATED: &str = "Date of birth";

/// A zone whose offset moves with the season, which is what makes the test
/// mean anything.
const ZONE: &str = "Europe/Amsterdam";
