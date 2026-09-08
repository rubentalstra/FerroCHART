// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! The documentation capture pass: one PNG per screen per ground, for the
//! book page at `website/book/src/operate/renderer.md`.
//!
//! It walks the same screens the journeys walk and runs the same proof
//! ([`crate::screens`]) before every shot, so an image is never a picture of
//! a screen that had not answered yet, and a screen the journeys stopped
//! covering cannot keep a photograph in the book.
//!
//! The pass is gated on [`GATE_ENV`], which `scripts/ui-e2e.sh` sets only
//! when it is asked for the capture: an ordinary run, on a pull request or on
//! a laptop, rewrites no tracked image.
//!
//! Nothing is typed into a form here. Every image is of an empty form over a
//! committed, synthetic openEHR CKM template, so no image can carry a
//! person's name, an identifier, or anything a clinician entered.

use std::path::Path;
use std::path::PathBuf;

use crate::harness::{Page, renderer, session};
use crate::screens::{Screen, Theme, wear};
use thirtyfour::prelude::*;

/// Names the capture pass. Unset, the pass skips and says so.
const GATE_ENV: &str = "FERROCHART_UI_E2E_DOCS_SHOTS";

/// Names the directory the images are written to.
const SHOTS_DIR_ENV: &str = "FERROCHART_UI_E2E_SHOTS_DIR";

/// Where the images live when nothing names a directory: the book's own
/// image directory, resolved from this crate's manifest.
const BOOK_IMAGES: &str = "../website/book/src/operate/img/renderer";

/// The width every shot is taken at, in CSS pixels.
///
/// Fixed, so every image is the same width and the book renders one column.
/// It is wide enough for the shell to draw its rail and its inspector, which
/// the responsive layout folds away below the `lg` breakpoint.
const WIDTH: u32 = 1440;

/// The shortest and tallest a shot may be, in CSS pixels.
///
/// The floor keeps a short screen from becoming a letterbox; the ceiling
/// keeps a long guide from becoming an image nobody can read.
const HEIGHT_BOUNDS: (u32, u32) = (900, 2600);

/// Where the images are written.
fn shots_dir() -> PathBuf {
    match std::env::var(SHOTS_DIR_ENV) {
        Ok(named) => PathBuf::from(named),
        Err(_absent) => Path::new(env!("CARGO_MANIFEST_DIR")).join(BOOK_IMAGES),
    }
}

/// Walks every screen on one ground, proving each before photographing it.
async fn walk(page: &mut Page, base: &str, dir: &Path, theme: Theme) -> WebDriverResult<()> {
    for screen in Screen::all() {
        let name = screen.name();
        page.now_on(&name);
        page.go(&name, &screen.address(base)).await;
        screen.prove(page).await;
        page.shoot(&dir.join(screen.image(theme)), WIDTH, HEIGHT_BOUNDS)
            .await?;
    }
    Ok(())
}

/// Captures every screen on both grounds, in one session.
///
/// One session takes every image, so they agree with each other. The ground
/// is set through the control a reader uses and is remembered from there on,
/// which is why the walk is once per ground rather than once per image.
#[tokio::test]
async fn the_documentation_screenshots_are_captured() {
    let Some(base) = renderer() else {
        return;
    };
    if std::env::var_os(GATE_ENV).is_none() {
        println!(
            "skipped: {GATE_ENV} is unset, so no tracked image is rewritten. \
             scripts/ui-e2e.sh sets it when it is asked for the capture pass."
        );
        return;
    }
    let dir = shots_dir();
    println!("capturing into {}", dir.display());
    let opening = Screen::Templates;
    let address = opening.address(&base);
    let outcome = session()
        .await
        .run_and_quit(|driver| async move {
            let mut page = Page::open(driver, &opening.name(), &address).await;
            for theme in Theme::BOTH {
                wear(&page, theme).await;
                walk(&mut page, &base, &dir, theme).await?;
            }
            Ok::<(), WebDriverError>(())
        })
        .await;
    outcome.expect("the capture pass ran and the browser session ended cleanly");
}
