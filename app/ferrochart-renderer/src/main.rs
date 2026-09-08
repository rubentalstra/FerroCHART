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
//! in [`kit`], the frame in [`shell`], and the token measurements in
//! [`tokens`].

// Leptos's `#[component]` macro emits `pub` items whatever visibility the
// function carries. Nothing in a binary crate is reachable from outside it,
// so the lint has nothing to protect here, and the alternative is the same
// suppression repeated on every component.
#![expect(
    unreachable_pub,
    reason = "the #[component] macro emits pub items inside a binary crate"
)]

mod app;
mod design;
mod icon;
mod kit;
mod nav;
mod shell;
mod storage;
mod theme;
mod tokens;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
