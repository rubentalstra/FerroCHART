// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a piece of state says, and the one set of colours that says it.
//!
//! One definition, shared by the badge, the notice, and the toast. A tone
//! declared once is what keeps "this failed" the same red in a pill, in a
//! banner, and in a toast; a second table would drift the first time one of
//! the three was restyled.

use icondata_core::IconData;

/// What a piece of state says.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tone {
    /// Neither good nor bad: a count, a category, a plain note.
    Neutral,
    /// The one brand accent: a selection, or the state a reader is in.
    Accent,
    /// A first-class state.
    Ok,
    /// Something a person should look at.
    Warn,
    /// A failure.
    Danger,
}

impl Tone {
    /// The tinted fill, its hairline, and the text that sits on it.
    pub(crate) const fn subtle(self) -> &'static str {
        match self {
            Self::Neutral => "border-edge bg-sunken text-ink-muted",
            Self::Accent => "border-accent bg-accent-subtle text-accent-ink",
            Self::Ok => "border-ok bg-ok-subtle text-ok",
            Self::Warn => "border-warn bg-warn-subtle text-warn",
            Self::Danger => "border-danger bg-danger-subtle text-danger",
        }
    }

    /// The glyph that carries the tone where the text alone would not.
    pub(crate) const fn icon(self) -> &'static IconData {
        match self {
            Self::Neutral => icondata_lu::LuInfo,
            Self::Accent => icondata_lu::LuSparkles,
            Self::Ok => icondata_lu::LuCircleCheck,
            Self::Warn => icondata_lu::LuTriangleAlert,
            Self::Danger => icondata_lu::LuCircleX,
        }
    }

    /// The word a screen reader hears in place of the glyph.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Neutral => "Note",
            Self::Accent => "Selected",
            Self::Ok => "Accepted",
            Self::Warn => "Check this",
            Self::Danger => "Refused",
        }
    }

    /// The WAI-ARIA live role the tone is announced under.
    ///
    /// A failure interrupts, so it is an `alert`; everything else is a
    /// `status`, which is announced when the reader is next idle.
    pub(crate) const fn live_role(self) -> &'static str {
        match self {
            Self::Danger => "alert",
            Self::Neutral | Self::Accent | Self::Ok | Self::Warn => "status",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tone;

    /// Every tone, so a rule is asserted over the set rather than one at a
    /// time.
    const ALL: [Tone; 5] = [
        Tone::Neutral,
        Tone::Accent,
        Tone::Ok,
        Tone::Warn,
        Tone::Danger,
    ];

    #[test]
    fn only_a_failure_interrupts_a_screen_reader() {
        for tone in ALL {
            let expected = if tone == Tone::Danger {
                "alert"
            } else {
                "status"
            };
            assert_eq!(tone.live_role(), expected, "{tone:?}");
        }
    }

    #[test]
    fn every_tone_names_a_fill_a_hairline_and_its_text() {
        for tone in ALL {
            let classes = tone.subtle();
            for part in ["border-", "bg-", "text-"] {
                assert!(classes.contains(part), "{tone:?} has no {part} class");
            }
            assert!(
                !classes.contains("dark:"),
                "dark mode is the tokens: {tone:?}"
            );
        }
    }

    #[test]
    fn every_tone_carries_a_word_a_screen_reader_can_read() {
        for tone in ALL {
            assert!(!tone.name().is_empty(), "{tone:?}");
        }
    }
}
