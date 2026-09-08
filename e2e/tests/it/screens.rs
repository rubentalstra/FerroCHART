// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! The screens the battery drives, declared once.
//!
//! This module is the reason the journeys and the capture pass cannot
//! disagree. A screen states its address, the image it is captured into, and
//! the proof that it really drew what it exists to draw; the journeys run the
//! proof and stop, and the capture runs the same proof and then takes the
//! picture. Neither one carries a selector of its own.
//!
//! No specification governs any of this: the renderer's screens are
//! FerroCHART's own design.

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use thirtyfour::prelude::*;

use crate::harness::Page;

/// Names the forms the battery drives.
///
/// One `stem<TAB>template identifier` per line, written by
/// `scripts/ui-e2e.sh` out of the templates it staged, so the operational
/// templates the server holds and the forms the battery opens are one
/// declaration rather than two that can drift.
pub(crate) const FORMS_ENV: &str = "FERROCHART_UI_E2E_FORMS";

/// The path the renderer is mounted at.
const BASE: &str = "/ui";

/// The characters escaped in a path segment.
///
/// A template identifier is free text, and RFC 3986 section 3.3 spells a
/// segment as `pchar`s, so this is the complement an identifier can actually
/// produce: the controls, the space, the delimiter that ends a segment, the
/// two that end a path, the four the URL standard forbids in a path, and `%`
/// itself. It is the renderer's own set, and one journey proves the two agree
/// by opening the address a rendered row leads to.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`');

/// One screen of the renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Screen {
    /// The template library.
    Templates,
    /// The form one operational template compiles to.
    Form {
        /// The template file's stem, which names the image.
        stem: String,
        /// The identifier the template states for itself, which addresses
        /// the form.
        template_id: String,
    },
    /// The living style guide.
    Design,
}

impl Screen {
    /// Every screen the battery drives, in reading order.
    ///
    /// # Panics
    ///
    /// Panics when [`FORMS_ENV`] names no form, because a battery over a
    /// renderer with no form to draw would pass while proving nothing.
    pub(crate) fn all() -> Vec<Self> {
        let forms = Self::forms();
        assert!(
            !forms.is_empty(),
            "{FORMS_ENV} names no form, so there is nothing to render. \
             scripts/ui-e2e.sh sets it from the templates it staged."
        );
        let mut every = vec![Self::Templates];
        every.extend(forms);
        every.push(Self::Design);
        every
    }

    /// The forms [`FORMS_ENV`] names.
    pub(crate) fn forms() -> Vec<Self> {
        let named = std::env::var(FORMS_ENV).unwrap_or_default();
        named
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .filter(|&(stem, template_id)| !stem.is_empty() && !template_id.is_empty())
            .map(|(stem, template_id)| Self::Form {
                stem: stem.to_owned(),
                template_id: template_id.to_owned(),
            })
            .collect()
    }

    /// What the screen is called, which names every failure on it.
    pub(crate) fn name(&self) -> String {
        match *self {
            Self::Templates => "the template library".to_owned(),
            Self::Form {
                ref template_id, ..
            } => format!("the form for `{template_id}`"),
            Self::Design => "the design system".to_owned(),
        }
    }

    /// The screen's address under `base`.
    pub(crate) fn address(&self, base: &str) -> String {
        match *self {
            Self::Templates => format!("{base}{BASE}/templates"),
            Self::Form {
                ref template_id, ..
            } => {
                let segment = utf8_percent_encode(template_id, SEGMENT);
                format!("{base}{BASE}/forms/{segment}")
            }
            Self::Design => format!("{base}{BASE}/design"),
        }
    }

    /// The file one capture of this screen is written to, in `theme`.
    pub(crate) fn image(&self, theme: Theme) -> String {
        let stem = match *self {
            Self::Templates => "templates".to_owned(),
            Self::Form { ref stem, .. } => format!("form-{stem}"),
            Self::Design => "design".to_owned(),
        };
        format!("{stem}{}.png", theme.suffix())
    }

    /// Proves the screen drew what it exists to draw.
    ///
    /// Every wait is on content rather than on a heading, because a heading
    /// renders before the read behind the screen has answered and a picture
    /// taken then is a picture of an empty page.
    pub(crate) async fn prove(&self, page: &Page) {
        match *self {
            Self::Templates => templates(page).await,
            Self::Form {
                ref template_id, ..
            } => form(page, template_id).await,
            Self::Design => design(page).await,
        }
        page.quiet().await;
    }
}

/// The template library lists exactly the templates the server was given.
///
/// A row per template, each one a link into the form it compiles to. The
/// count is asserted rather than "at least one", so a server that lost a
/// template fails here instead of being photographed.
async fn templates(page: &Page) {
    let wanted = Screen::forms().len();
    let rows = page
        .at_least(
            By::Css("main ul li a[href^='/ui/forms/']"),
            wanted,
            "a row per template, each linking to the form it compiles to",
        )
        .await;
    assert_eq!(
        rows,
        wanted,
        "{}: the library lists {rows} templates and the server was given {wanted}",
        page.screen()
    );
    no_refusal(page).await;
}

/// A form draws the controls a clinician types into.
///
/// The heading is not the proof. A form that could not be read still heads
/// the screen with the identifier it was asked for and then draws a refusal,
/// so the proof is the controls: a group headed by the label the template
/// gave it, a field drawn as its own fieldset, and inside that fieldset at
/// least one element a person can enter a value into.
async fn form(page: &Page, template_id: &str) {
    let heading = page
        .element(By::Css("main h1"), "the identifier the form is headed with")
        .await;
    let headed = heading.text().await.unwrap_or_default();
    assert_eq!(
        headed.trim(),
        template_id,
        "{}: the screen is headed `{headed}`",
        page.screen()
    );
    no_refusal(page).await;
    page.at_least(
        By::Css("main section h3"),
        1,
        "a group of the form, headed by the label the template gave it",
    )
    .await;
    page.at_least(
        By::Css("main section fieldset"),
        1,
        "a field of the form, drawn as its own fieldset",
    )
    .await;
    page.at_least(
        By::Css("main fieldset input, main fieldset select, main fieldset textarea"),
        1,
        "a control a clinician can enter a value into",
    )
    .await;
}

/// The design system draws the affordances the kit defines.
///
/// It is the one screen where a refusal notice is content: the guide draws
/// every tone, and the danger tone is a refusal. The proof is therefore the
/// breadth of the guide rather than the absence of an alert.
async fn design(page: &Page) {
    page.at_least(By::Css("main section"), 6, "the panels of the guide")
        .await;
    page.at_least(By::Css("main button"), 4, "the buttons the kit defines")
        .await;
    page.at_least(
        By::Css("main input, main select, main textarea"),
        3,
        "the entry controls the kit defines",
    )
    .await;
    page.at_least(
        By::Css("main div[role='status'], main div[role='alert']"),
        2,
        "the notices, one per tone",
    )
    .await;
}

/// Fails when the screen drew a refusal in place of its content.
async fn no_refusal(page: &Page) {
    let alerts = page.count(By::Css("main div[role='alert']")).await;
    assert_eq!(
        alerts,
        0,
        "{}: the screen drew {alerts} refusals instead of what it is for",
        page.screen()
    );
}

/// Which ground a capture is taken on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Theme {
    /// The light ground, which is the one the book shows by default.
    Light,
    /// The dark ground.
    Dark,
}

impl Theme {
    /// Both grounds, light first, which is the order a capture walks them in.
    pub(crate) const BOTH: [Self; 2] = [Self::Light, Self::Dark];

    /// The class the renderer puts on the document element for this ground.
    ///
    /// Both are written out. An absent class is not the light theme: it is
    /// "no choice yet", which the stylesheet resolves from the system
    /// preference.
    pub(crate) const fn class(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// What the image file's name carries, so the light shot is the plain one.
    const fn suffix(self) -> &'static str {
        match self {
            Self::Light => "",
            Self::Dark => "-dark",
        }
    }
}

/// The ground the renderer has put the document on.
///
/// # Panics
///
/// Panics when the shell never writes a ground onto the document element.
pub(crate) async fn ground(page: &Page) -> String {
    let both = [Theme::Light.class(), Theme::Dark.class()];
    page.class_becoming(
        By::Css("html"),
        &both,
        "the ground the shell writes onto the document",
    )
    .await
}

/// Puts the renderer on `theme`, through the control a reader uses.
///
/// The header carries one toggle rather than a picker, so reaching a named
/// ground means clicking it once from the other one. The choice is remembered
/// in local storage, so it survives every navigation the capture makes
/// afterwards.
///
/// # Panics
///
/// Panics when the document never reaches `theme`.
pub(crate) async fn wear(page: &Page, theme: Theme) {
    if ground(page).await == theme.class() {
        return;
    }
    page.click(
        By::Css("header button[aria-label^='Switch to the']"),
        "the control that switches the ground",
    )
    .await;
    page.class_becoming(
        By::Css("html"),
        &[theme.class()],
        "the ground the control switches to",
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::{Screen, Theme};

    fn every_shape() -> Vec<Screen> {
        vec![
            Screen::Templates,
            Screen::Design,
            Screen::Form {
                stem: "s".to_owned(),
                template_id: "t".to_owned(),
            },
        ]
    }

    #[test]
    fn a_template_identifier_carrying_a_space_stays_inside_its_segment() {
        let screen = Screen::Form {
            stem: "family-history-summary-item-r2".to_owned(),
            template_id: "Family history summary item R2".to_owned(),
        };
        assert_eq!(
            screen.address("http://renderer:8081"),
            "http://renderer:8081/ui/forms/Family%20history%20summary%20item%20R2"
        );
    }

    #[test]
    fn a_separator_in_a_template_identifier_does_not_address_another_screen() {
        let screen = Screen::Form {
            stem: "a-b".to_owned(),
            template_id: "a/b".to_owned(),
        };
        assert_eq!(screen.address(""), "/ui/forms/a%2Fb");
    }

    #[test]
    fn the_two_grounds_write_two_different_images_of_one_screen() {
        for screen in every_shape() {
            let light = screen.image(Theme::Light);
            let dark = screen.image(Theme::Dark);
            assert_ne!(light, dark, "{screen:?}");
            assert_eq!(
                std::path::Path::new(&light).extension(),
                Some(std::ffi::OsStr::new("png")),
                "{light}"
            );
            assert!(dark.ends_with("-dark.png"), "{dark}");
        }
    }

    #[test]
    fn no_two_screens_are_captured_into_one_image() {
        let mut names: Vec<String> = Vec::new();
        for screen in every_shape() {
            for theme in Theme::BOTH {
                names.push(screen.image(theme));
            }
        }
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "two screens write one image");
    }

    #[test]
    fn every_screen_names_itself_and_hangs_off_the_one_base() {
        for screen in every_shape() {
            assert!(!screen.name().is_empty(), "{screen:?}");
            assert!(screen.address("").starts_with("/ui/"), "{screen:?}");
        }
    }

    #[test]
    fn the_two_grounds_do_not_share_a_class() {
        assert_ne!(Theme::Light.class(), Theme::Dark.class());
    }
}
