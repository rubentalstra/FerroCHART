// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The rail's entries, and the pure function that decides which one is on.
//!
//! No specification governs this: our own design. The slot table is the ONLY
//! place the rail's order is declared, and [`section`] is a pure map from a
//! path to the entry that owns it, so a deep route still marks its section
//! and a test can prove every entry marks itself.

use icondata_core::IconData;

/// The path every route hangs off, so the renderer can be mounted beside the
/// server's own surface rather than at the origin's root.
pub(crate) const BASE: &str = "/ui";

/// The forms library, and the rail's first entry.
pub(crate) const FORMS: &str = "forms";
/// The template library.
pub(crate) const TEMPLATES: &str = "templates";
/// The layout overlay authoring surface.
pub(crate) const LAYOUT: &str = "layout";
/// The commit log.
pub(crate) const COMMITS: &str = "commits";
/// The settings screen.
pub(crate) const SETTINGS: &str = "settings";
/// The living style guide.
pub(crate) const DESIGN: &str = "design";

/// One position in the rail.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Slot {
    /// An entry: the section it owns, its label, and its Lucide glyph.
    Item(&'static str, &'static str, &'static IconData),
    /// The hairline between the work group and the meta group.
    Divider,
}

/// The rail, in order. Adding a screen means adding a line here and nothing
/// else.
pub(crate) const SLOTS: [Slot; 7] = [
    Slot::Item(FORMS, "Forms", icondata_lu::LuClipboardList),
    Slot::Item(TEMPLATES, "Templates", icondata_lu::LuFileCode2),
    Slot::Item(LAYOUT, "Layout", icondata_lu::LuLayoutTemplate),
    Slot::Item(COMMITS, "Commits", icondata_lu::LuGitCommitHorizontal),
    Slot::Divider,
    Slot::Item(SETTINGS, "Settings", icondata_lu::LuSettings),
    Slot::Item(DESIGN, "Design", icondata_lu::LuPalette),
];

/// The section a path belongs to, or `None` when no entry owns it.
///
/// `None` rather than a default entry: a route off the rail leaves every
/// entry unmarked, where a fallback would light one the reader is not on.
pub(crate) fn section(path: &str) -> Option<&'static str> {
    let mut segments = path.trim_start_matches('/').split('/');
    let base = BASE.trim_start_matches('/');
    if segments.next()? != base {
        return None;
    }
    let head = segments.next()?.split('?').next()?;
    SLOTS.iter().find_map(|slot| match slot {
        Slot::Item(section, _, _) if *section == head => Some(*section),
        Slot::Item(..) | Slot::Divider => None,
    })
}

/// The href the rail renders for a section.
pub(crate) fn href(section: &str) -> String {
    format!("{BASE}/{section}")
}

#[cfg(test)]
mod tests {
    use super::{BASE, FORMS, SLOTS, Slot, href, section};

    fn entries() -> Vec<&'static str> {
        SLOTS
            .iter()
            .filter_map(|slot| match slot {
                Slot::Item(section, _, _) => Some(*section),
                Slot::Divider => None,
            })
            .collect()
    }

    #[test]
    fn every_entry_marks_itself_when_a_reader_is_on_it() {
        for entry in entries() {
            assert_eq!(
                section(&href(entry)),
                Some(entry),
                "the entry for `{entry}` marks some other screen"
            );
        }
    }

    #[test]
    fn a_deep_route_still_marks_the_section_that_owns_it() {
        assert_eq!(section("/ui/templates/vitals.v1"), Some("templates"));
        assert_eq!(section("/ui/forms/abc/fields/7"), Some(FORMS));
        assert_eq!(section("/ui/forms?page=2"), Some(FORMS));
    }

    #[test]
    fn a_path_off_the_rail_marks_nothing() {
        assert_eq!(section("/ui"), None);
        assert_eq!(section("/ui/"), None);
        assert_eq!(section("/ui/unknown"), None);
        assert_eq!(section("/forms"), None);
        assert_eq!(section("/"), None);
        assert_eq!(section(""), None);
    }

    #[test]
    fn no_two_entries_lead_to_the_same_screen() {
        let mut seen = entries();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "two rail entries lead to one screen");
    }

    #[test]
    fn the_divider_is_unique_and_stands_between_two_groups() {
        let dividers = SLOTS
            .iter()
            .filter(|slot| matches!(slot, Slot::Divider))
            .count();
        assert_eq!(dividers, 1, "a second hairline draws a group of one");
        assert!(
            !matches!(SLOTS.first(), Some(Slot::Divider)),
            "a hairline at the top draws a group with nothing in it"
        );
        assert!(
            !matches!(SLOTS.last(), Some(Slot::Divider)),
            "a hairline at the bottom draws a group with nothing in it"
        );
    }

    #[test]
    fn every_entry_has_a_label_and_hangs_off_the_base() {
        for slot in &SLOTS {
            if let Slot::Item(section, label, _) = slot {
                assert!(!label.is_empty(), "`{section}` has no label");
                assert!(!section.contains('/'), "`{section}` is not one segment");
                assert!(href(section).starts_with(BASE), "`{section}`");
            }
        }
    }
}
