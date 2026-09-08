// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Buttons and inputs.
//!
//! Three buttons and no more: a primary that fills with the accent, a
//! secondary that outlines, and a danger. A fourth would be a decision about
//! emphasis that the screen should be making with layout instead.
//!
//! No constant here carries a focus ring. The stylesheet's base layer draws
//! one `:focus-visible` outline for the whole app, so a control that forgets
//! is impossible rather than merely discouraged.

/// The one affirmative action on a screen.
pub(crate) const BTN_PRIMARY: &str = "inline-flex items-center justify-center gap-1.5 \
                                      rounded-control px-3 py-1.5 text-sm font-medium \
                                      bg-accent text-on-accent hover:bg-accent-hover \
                                      disabled:cursor-not-allowed disabled:opacity-60";

/// Everything else.
pub(crate) const BTN_SECONDARY: &str = "inline-flex items-center justify-center gap-1.5 \
                                        rounded-control px-3 py-1.5 text-sm font-medium \
                                        border border-edge-strong bg-raised text-ink \
                                        hover:bg-sunken disabled:cursor-not-allowed \
                                        disabled:opacity-60";

/// An action that destroys something.
pub(crate) const BTN_DANGER: &str = "inline-flex items-center justify-center gap-1.5 \
                                     rounded-control px-3 py-1.5 text-sm font-medium \
                                     border border-danger bg-danger-subtle text-danger \
                                     hover:bg-danger hover:text-raised \
                                     disabled:cursor-not-allowed disabled:opacity-60";

/// A quiet control that lives inside a bar: the theme switch, a row action.
pub(crate) const BTN_QUIET: &str = "inline-flex h-8 w-8 items-center justify-center \
                                    rounded-control text-ink-muted hover:bg-sunken \
                                    hover:text-ink";

/// A text input.
pub(crate) const INPUT: &str = "w-full rounded-control border border-edge-strong bg-raised \
                                px-2.5 py-1.5 text-sm text-ink disabled:cursor-not-allowed \
                                disabled:opacity-60";

/// A select. The same box as an input, so a form row does not step.
pub(crate) const SELECT: &str = "w-full rounded-control border border-edge-strong bg-raised \
                                 px-2.5 py-1.5 text-sm text-ink disabled:cursor-not-allowed \
                                 disabled:opacity-60";

/// A textarea.
pub(crate) const TEXTAREA: &str = "w-full min-h-24 rounded-control border border-edge-strong \
                                   bg-raised px-2.5 py-1.5 text-sm leading-relaxed text-ink \
                                   disabled:cursor-not-allowed disabled:opacity-60";

/// The label above a control.
pub(crate) const LABEL: &str = "mb-1 block text-xs font-medium text-ink-muted";

/// The help text under a control.
pub(crate) const HINT: &str = "mt-1 block text-xs text-ink-muted";

#[cfg(test)]
mod tests {
    use super::{
        BTN_DANGER, BTN_PRIMARY, BTN_QUIET, BTN_SECONDARY, HINT, INPUT, LABEL, SELECT, TEXTAREA,
    };

    /// Every class constant in this module, so a rule can be asserted over
    /// the set rather than one constant at a time.
    const ALL: [&str; 9] = [
        BTN_PRIMARY,
        BTN_SECONDARY,
        BTN_DANGER,
        BTN_QUIET,
        INPUT,
        SELECT,
        TEXTAREA,
        LABEL,
        HINT,
    ];

    #[test]
    fn no_constant_draws_its_own_focus_ring() {
        for class in ALL {
            assert!(
                !class.contains("focus:") && !class.contains("focus-visible:"),
                "the stylesheet's base layer owns the focus indicator: {class}"
            );
            assert!(
                !class.contains("outline-none"),
                "removing the indicator leaves a control with none: {class}"
            );
        }
    }

    #[test]
    fn no_constant_reaches_past_the_semantic_tokens() {
        for class in ALL {
            assert!(!class.contains("dark:"), "dark mode is the tokens: {class}");
            for raw in ["slate-", "rose-", "gray-", "zinc-", "red-", "pink-"] {
                assert!(!class.contains(raw), "raw palette `{raw}` in: {class}");
            }
        }
    }

    #[test]
    fn the_line_continuations_left_no_double_space_in_a_class_list() {
        for class in ALL {
            assert!(!class.contains("  "), "a run of spaces in: {class}");
            assert_eq!(class.trim(), class, "leading or trailing space: {class}");
        }
    }
}
