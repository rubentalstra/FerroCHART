// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The living style guide.
//!
//! No specification governs this: our own design. Every affordance the kit
//! defines is drawn here once, on one screen, in whichever theme the reader
//! has chosen. It is the page a screenshot battery captures (issue #93), and
//! it is what makes "one definition per affordance" checkable by eye: a
//! second button style would have to appear here beside the first.

use leptos::prelude::*;

use crate::icon::{Decoration, Glyph, Size};
use crate::kit::field::{
    BTN_DANGER, BTN_PRIMARY, BTN_QUIET, BTN_SECONDARY, HINT, INPUT, LABEL, SELECT, TEXTAREA,
};
use crate::kit::page_header::{Crumb, PageHeader};
use crate::kit::surface::{CARD, CARD_PAD, CARD_TITLE, CODE, WELL};

/// Every semantic colour token, paired with what it is for.
const SWATCHES: [(&str, &str, &str); 12] = [
    ("bg-surface", "surface", "the page ground"),
    ("bg-raised", "raised", "cards, the topbar, the rail"),
    ("bg-sunken", "sunken", "wells and table headers"),
    ("bg-edge", "edge", "the decorative hairline"),
    ("bg-edge-strong", "edge-strong", "a control boundary, 3:1"),
    ("bg-ink", "ink", "primary text"),
    ("bg-ink-muted", "ink-muted", "labels and placeholders"),
    ("bg-ink-faint", "ink-faint", "a disabled value"),
    ("bg-accent", "accent", "the one brand accent"),
    ("bg-accent-subtle", "accent-subtle", "the tinted fill"),
    ("bg-ok", "ok", "a first-class state"),
    ("bg-danger", "danger", "a failure"),
];

/// The style guide screen.
#[component]
pub(crate) fn DesignSystem() -> impl IntoView {
    view! {
        <PageHeader
            title=Signal::derive(|| "Design system".to_owned())
            subtitle=Signal::derive(|| {
                "Every affordance the kit defines, drawn once, in the theme you chose.".to_owned()
            })
            crumbs=vec![
                Crumb::link("FerroCHART", crate::nav::href(crate::nav::FORMS)),
                Crumb::here("Design system"),
            ]
        >
            <button type="button" class=BTN_SECONDARY>
                <Decoration icon=icondata_lu::LuDownload size=Size::Inline />
                "Export tokens"
            </button>
        </PageHeader>

        <div class="flex flex-col gap-6">
            <Colour />

            <Section title="Buttons">
                <div class="flex flex-wrap items-center gap-3">
                    <button type="button" class=BTN_PRIMARY>
                        <Decoration icon=icondata_lu::LuCheck size=Size::Inline />
                        "Commit"
                    </button>
                    <button type="button" class=BTN_SECONDARY>
                        "Preview"
                    </button>
                    <button type="button" class=BTN_DANGER>
                        <Decoration icon=icondata_lu::LuTrash2 size=Size::Inline />
                        "Discard"
                    </button>
                    <button type="button" class=BTN_PRIMARY disabled=true>
                        "Disabled"
                    </button>
                    <button type="button" class=BTN_QUIET>
                        <Glyph icon=icondata_lu::LuEllipsis size=Size::Topbar label="More" />
                    </button>
                </div>
            </Section>

            <Inputs />

            <Section title="Surfaces">
                <div class="grid gap-4 sm:grid-cols-2">
                    <div class=CARD_PAD>
                        <h3 class=CARD_TITLE>"A card"</h3>
                        <p class="text-sm text-ink-muted">
                            "The default container for anything with a boundary."
                        </p>
                    </div>
                    <div class=WELL>
                        <p class=CODE>
                            "openEHR-EHR-OBSERVATION.blood_pressure.v2/data[id2]/events[id3]"
                        </p>
                    </div>
                    <div class=CARD>
                        <div class="border-b border-edge px-4 py-2 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                            "A card with its own header"
                        </div>
                        <p class="px-4 py-3 text-sm text-ink-muted">
                            "Depth is raised against sunken plus a hairline. There is no elevation scale."
                        </p>
                    </div>
                </div>
            </Section>

            <Icons />

            <States />
        </div>
    }
}

/// Every semantic colour, with what it is for.
#[component]
fn Colour() -> impl IntoView {
    view! {
        <Section title="Colour">
            <ul class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
                {SWATCHES
                    .iter()
                    .map(|(class, name, purpose)| {
                        view! {
                            <li class="flex items-center gap-3">
                                <span class=format!(
                                    "h-9 w-9 shrink-0 rounded-control border border-edge {class}",
                                )></span>
                                <span class="min-w-0">
                                    <span class="block truncate font-mono text-xs text-ink">
                                        {*name}
                                    </span>
                                    <span class="block truncate text-xs text-ink-muted">
                                        {*purpose}
                                    </span>
                                </span>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()}
            </ul>
        </Section>
    }
}

/// The controls a reader types into.
#[component]
fn Inputs() -> impl IntoView {
    view! {
        <Section title="Inputs">
            <div class="grid gap-4 sm:grid-cols-2">
                <div>
                    <label class=LABEL for="guide-text">
                        "Template id"
                    </label>
                    <input id="guide-text" class=INPUT type="text" placeholder="vital_signs.v1" />
                    <span class=HINT>"The operational template this form compiled from."</span>
                </div>
                <div>
                    <label class=LABEL for="guide-select">
                        "Unit"
                    </label>
                    <select id="guide-select" class=SELECT>
                        <option>"mm[Hg]"</option>
                        <option>"kPa"</option>
                    </select>
                </div>
                <div class="sm:col-span-2">
                    <label class=LABEL for="guide-area">
                        "Comment"
                    </label>
                    <textarea id="guide-area" class=TEXTAREA></textarea>
                </div>
            </div>
        </Section>
    }
}

/// The size ladder, drawn at every step.
#[component]
fn Icons() -> impl IntoView {
    let steps = [
        (Size::Chip, "12, a chip"),
        (Size::Inline, "14, beside text"),
        (Size::Rail, "16, a rail entry"),
        (Size::Topbar, "18, a topbar control"),
        (Size::Plate, "20, a plate"),
    ];
    view! {
        <Section title="Icons">
            <div class="flex flex-wrap items-end gap-6">
                {steps
                    .into_iter()
                    .map(|(size, caption)| {
                        view! {
                            <span class="flex flex-col items-center gap-1 text-ink">
                                <Decoration icon=icondata_lu::LuActivity size=size />
                                <span class="text-xs text-ink-muted">{caption}</span>
                            </span>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </Section>
    }
}

/// A glyph that is the only thing carrying its meaning is labelled, so a
/// screen reader hears the state rather than "graphics symbol".
#[component]
fn States() -> impl IntoView {
    let states = [
        (icondata_lu::LuCircleCheck, "Committed", "text-ok"),
        (icondata_lu::LuTriangleAlert, "Needs review", "text-warn"),
        (icondata_lu::LuCircleX, "Rejected", "text-danger"),
    ];
    view! {
        <Section title="States">
            <ul class="flex flex-wrap items-center gap-5">
                {states
                    .into_iter()
                    .map(|(icon, label, tint)| {
                        view! {
                            <li class=format!("flex items-center gap-2 text-sm {tint}")>
                                <Glyph icon=icon size=Size::Plate label=label />
                                <span class="text-ink">{label}</span>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()}
            </ul>
        </Section>
    }
}

/// One titled block of the guide.
#[component]
fn Section(
    /// What the block shows.
    title: &'static str,
    /// The affordances themselves.
    children: Children,
) -> impl IntoView {
    view! {
        <section>
            <h2 class="mb-3 text-xs font-semibold uppercase tracking-wide text-ink-muted">
                {title}
            </h2>
            {children()}
        </section>
    }
}
