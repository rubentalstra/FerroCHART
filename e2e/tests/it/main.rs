// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1
//! The one integration-test binary for the browser journeys, one module per
//! topic over a shared harness.
//!
//! `scripts/ui-e2e.sh` stands up the renderer, the server behind it and the
//! browser, and sets the variables the harness reads. Without them there is
//! nothing to drive and every journey skips, saying so; the `ui-e2e` CI job
//! always runs that script, so a skip is never a pass.
//!
//! `docs_shots` is a capture rather than a journey: it writes the book's
//! screenshots and runs only when the script is asked for it, so an ordinary
//! run rewrites no tracked file.
//!
//! The renderer is a client-side WebAssembly bundle, so the document is an
//! empty `<body>` until it boots. Every assertion therefore waits for a drawn
//! element, and nothing here reads the page source: inert markup satisfies a
//! source assertion even when the control it describes is unreachable, which
//! is the failure a rendering test exists to catch.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test assertions, the notices a skipped journey prints, and the line a capture prints per image"
)]

mod docs_shots;
mod harness;
mod journeys;
mod screens;
