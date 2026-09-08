// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The block a screen says something went wrong in.
//!
//! `docs/architecture.md` section 10.1: a pure READ never toasts and renders
//! an inline error instead, because a toast leaves nothing on the screen that
//! says why the panel below it is empty. A mutation reports both outcomes and
//! toasts, and that surface lands with the control that makes one.
//!
//! One definition, three tones. A screen that writes its own coloured box is
//! how two failures start looking like different kinds of failure.

use leptos::prelude::*;

/// How loud a notice is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    /// Something failed.
    Danger,
    /// Something needs attention and nothing failed.
    Warn,
    /// Something worth saying that is neither.
    Info,
}

impl Tone {
    /// The tinted box this tone draws.
    ///
    /// Every value is a semantic token, so both themes move together and
    /// `scripts/checks/palette-utilities.sh` has nothing to catch.
    const fn class(self) -> &'static str {
        match self {
            Self::Danger => "rounded-card border border-danger bg-danger-subtle p-3 text-danger",
            Self::Warn => "rounded-card border border-warn bg-warn-subtle p-3 text-warn",
            Self::Info => "rounded-card border border-edge bg-sunken p-3 text-ink-muted",
        }
    }

    /// The ARIA role this tone announces itself with.
    ///
    /// WAI-ARIA 1.2 section 5.3.2 gives `alert` an implicit assertive live
    /// region and `status` a polite one, so a failure interrupts and
    /// everything else waits for a pause.
    const fn role(self) -> &'static str {
        match self {
            Self::Danger => "alert",
            Self::Warn | Self::Info => "status",
        }
    }
}

/// The heading line of a notice.
const NOTICE_TITLE: &str = "text-sm font-semibold";

/// The detail under it.
pub(crate) const NOTICE_DETAIL: &str = "mt-1 text-xs";

/// An inline notice.
#[component]
pub(crate) fn Notice(
    /// How loud it is.
    tone: Tone,
    /// The one line a reader takes away.
    #[prop(into)]
    title: String,
    /// The detail under the line, where there is any.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=tone.class() role=tone.role()>
            <p class=NOTICE_TITLE>{title}</p>
            {children.map(|detail| view! { <div class=NOTICE_DETAIL>{detail()}</div> })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{NOTICE_DETAIL, Tone};

    /// Every tone, so a rule is asserted over the set.
    const ALL: [Tone; 3] = [Tone::Danger, Tone::Warn, Tone::Info];

    #[test]
    fn every_tone_draws_a_box_of_the_same_shape() {
        for tone in ALL {
            let class = tone.class();
            assert!(class.contains("rounded-card"), "{class}");
            assert!(class.contains("border"), "{class}");
            assert!(class.contains("p-3"), "{class}");
        }
    }

    #[test]
    fn no_tone_reaches_past_the_semantic_tokens() {
        for tone in ALL {
            let class = tone.class();
            assert!(!class.contains("dark:"), "dark mode is the tokens: {class}");
            for raw in ["slate-", "rose-", "gray-", "zinc-", "red-", "amber-"] {
                assert!(!class.contains(raw), "raw palette `{raw}` in: {class}");
            }
        }
    }

    #[test]
    fn no_two_tones_look_alike() {
        let mut seen: Vec<&str> = ALL.iter().map(|tone| tone.class()).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "two tones draw the same box");
    }

    #[test]
    fn only_a_failure_interrupts_a_reader() {
        assert_eq!(Tone::Danger.role(), "alert");
        assert_eq!(Tone::Warn.role(), "status");
        assert_eq!(Tone::Info.role(), "status");
    }

    #[test]
    fn the_detail_sits_under_the_line_rather_than_beside_it() {
        assert!(NOTICE_DETAIL.starts_with("mt-"), "{NOTICE_DETAIL}");
    }
}
