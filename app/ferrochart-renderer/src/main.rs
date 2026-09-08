// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The FerroCHART renderer (`docs/architecture.md` section 10).
//!
//! A Leptos client-side binary. It reads the form definition the server
//! publishes and links no engine crate, which
//! `scripts/checks/crate-closure.sh` enforces from the resolved dependency
//! graph rather than from intent.
//!
//! This module is the entry point and nothing else. The design system lives
//! in [`kit`], the frame in [`shell`], the screens in [`screen`], the
//! controls a clinician types into in [`control`], what they type in
//! [`state`], the token measurements in [`tokens`], and every request the
//! browser makes in [`api`], which is the only module that opens one.

// Leptos's `#[component]` macro emits `pub` items whatever visibility the
// function carries. Nothing in a binary crate is reachable from outside it,
// so the lint has nothing to protect here, and the alternative is the same
// suppression repeated on every component.
#![expect(
    unreachable_pub,
    reason = "the #[component] macro emits pub items inside a binary crate"
)]
// The style guide is the only consumer of the kit affordances #27 will draw
// (the dialog, the table, the tabs, the toasts), so a release bundle, built
// without it, reports twenty-five of them dead. They are not dead; they have
// no product consumer yet, and issue #186 is the record. The default build
// carries the guide and still fails on genuinely dead code.
#![cfg_attr(
    not(feature = "design"),
    allow(
        dead_code,
        reason = "the kit affordances issue #186 tracks, whose only consumer is the style guide"
    )
)]

mod api;
mod app;
mod control;
#[cfg(feature = "design")]
mod design;
mod focus;
mod icon;
mod kit;
mod label;
mod nav;
mod placement;
mod plain;
mod screen;
mod shell;
mod state;
mod storage;
mod theme;
mod tokens;
mod url;
mod zone;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
