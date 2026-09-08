// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The table shell: one set of classes every grid in this app is drawn with.
//!
//! One definition. A table is markup rather than a component, because the
//! cells differ on every screen and a component that took them as props would
//! be a worse `<table>`. What is shared is the shell: the scroll container, the
//! sunken header, the hairline between rows.
//!
//! The scroll container is not decoration. A form definition holds archetype
//! paths that do not wrap, and a grid that overflows its column pushes the
//! whole page sideways.

/// The container. A grid wider than its column scrolls inside this rather
/// than pushing the page.
pub(crate) const TABLE_SCROLL: &str = "overflow-x-auto rounded-card border border-edge";

/// The table itself.
pub(crate) const TABLE: &str = "w-full border-collapse text-left text-sm";

/// The header row.
pub(crate) const TABLE_HEAD: &str = "bg-sunken";

/// A header cell.
pub(crate) const TABLE_HEAD_CELL: &str = "whitespace-nowrap px-3 py-2 text-xs font-semibold \
                                          uppercase tracking-wide text-ink-muted";

/// A body row.
pub(crate) const TABLE_ROW: &str = "border-t border-edge";

/// A body cell.
pub(crate) const TABLE_CELL: &str = "px-3 py-2 align-top text-ink";

#[cfg(test)]
mod tests {
    use super::{TABLE, TABLE_CELL, TABLE_HEAD, TABLE_HEAD_CELL, TABLE_ROW, TABLE_SCROLL};

    #[test]
    fn a_wide_grid_scrolls_inside_its_own_container() {
        assert!(
            TABLE_SCROLL.contains("overflow-x-auto"),
            "a grid wider than its column has to scroll rather than push the page"
        );
        assert!(
            TABLE.contains("w-full"),
            "the grid fills the container it scrolls in"
        );
    }

    #[test]
    fn no_constant_reaches_past_the_semantic_tokens() {
        for class in [
            TABLE_SCROLL,
            TABLE,
            TABLE_HEAD,
            TABLE_HEAD_CELL,
            TABLE_ROW,
            TABLE_CELL,
        ] {
            assert!(!class.contains("dark:"), "dark mode is the tokens: {class}");
            assert!(
                !class.contains("focus:") && !class.contains("focus-visible:"),
                "the stylesheet owns the focus indicator: {class}"
            );
            assert!(!class.contains("  "), "a run of spaces in: {class}");
            assert_eq!(class.trim(), class, "leading or trailing space: {class}");
        }
    }
}
