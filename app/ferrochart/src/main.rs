// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The FerroCHART binary.

// A binary writes to stdio; a library does not.
#![expect(
    clippy::print_stdout,
    reason = "the binary reports to a terminal, per .claude/rules/reliability.md"
)]

fn main() {
    println!("FerroCHART {}", env!("CARGO_PKG_VERSION"));
}
