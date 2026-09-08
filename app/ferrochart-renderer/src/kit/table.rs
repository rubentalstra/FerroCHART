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
    use super::{TABLE, TABLE_SCROLL};

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
}
