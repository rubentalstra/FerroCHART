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
//! in [`kit`], the frame in [`shell`], the screens in [`screen`], the token
//! measurements in [`tokens`], and every request the browser makes in
//! [`api`], which is the only module that opens one.

// Leptos's `#[component]` macro emits `pub` items whatever visibility the
// function carries. Nothing in a binary crate is reachable from outside it,
// so the lint has nothing to protect here, and the alternative is the same
// suppression repeated on every component.
#![expect(
    unreachable_pub,
    reason = "the #[component] macro emits pub items inside a binary crate"
)]

mod api;
mod app;
mod design;
mod icon;
mod kit;
mod label;
mod nav;
mod placement;
mod screen;
mod shell;
mod storage;
mod theme;
mod tokens;
mod url;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
