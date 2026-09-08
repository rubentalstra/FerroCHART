// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The contrast gate over `style/tailwind.css`.
//!
//! No specification governs the token set: our own design. The bars are WCAG
//! 2.2, which `docs/architecture.md` section 10 commits to at level AA:
//! 1.4.3 Contrast (Minimum) asks 4.5:1 of body text, 1.4.11 Non-text Contrast
//! asks 3:1 of a user interface component's boundary, and 1.4.3 exempts an
//! inactive control.
//!
//! The stylesheet is the one place a colour is written. This module parses it
//! and measures every pair the design depends on, in BOTH themes, so a
//! ratio in a comment there is checked rather than claimed and a hand-tuned
//! hex cannot quietly drop a pair below its bar.
//!
//! <https://www.w3.org/TR/WCAG22/#contrast-minimum>
//! <https://www.w3.org/TR/WCAG22/#non-text-contrast>

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    /// The stylesheet, read at compile time so the test cannot miss a build
    /// that moved it.
    const STYLESHEET: &str = include_str!("../style/tailwind.css");

    /// Body text, WCAG 2.2 success criterion 1.4.3.
    const TEXT: f64 = 4.5;
    /// A control boundary or a meaningful graphic, criterion 1.4.11.
    const GRAPHICS: f64 = 3.0;

    /// The three grounds a foreground token can land on.
    const GROUNDS: [&str; 3] = ["surface", "raised", "sunken"];

    /// Reads the custom properties out of one rule.
    ///
    /// The parse is deliberately literal: it takes the text between the
    /// selector and the first `}`, and keeps `--name: #hex` lines. A token
    /// written as anything but a six-digit hex is skipped, which is why
    /// `--scrim` never appears in a pair below.
    fn tokens(selector: &str) -> BTreeMap<String, String> {
        let body = STYLESHEET
            .split_once(selector)
            .and_then(|(_, rest)| rest.split_once('{'))
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(body, _)| body)
            .unwrap_or_default();
        body.lines()
            .filter_map(|line| line.trim().strip_prefix("--"))
            .filter_map(|line| line.split_once(':'))
            .filter_map(|(name, value)| {
                // The declaration carries its measured ratio in a trailing
                // comment, which is the whole point of the file, so the
                // comment comes off before the value is read.
                let value = value.split("/*").next()?;
                let value = value.trim().trim_end_matches(';').trim();
                let hex = value.strip_prefix('#')?;
                (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
                    .then(|| (name.trim().to_owned(), value.to_owned()))
            })
            .collect()
    }

    /// The light theme's tokens.
    fn light() -> BTreeMap<String, String> {
        tokens("\n:root ")
    }

    /// The dark theme's tokens.
    fn dark() -> BTreeMap<String, String> {
        tokens("\n.dark ")
    }

    /// One channel, linearised as WCAG 2.2 defines relative luminance.
    fn channel(value: f64) -> f64 {
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    /// The relative luminance of a `#rrggbb` string.
    fn luminance(hex: &str) -> f64 {
        let digits = hex.trim_start_matches('#');
        let byte = |at: usize| {
            let pair = digits.get(at..at + 2).unwrap_or("00");
            f64::from(u8::from_str_radix(pair, 16).unwrap_or_default()) / 255.0
        };
        0.2126 * channel(byte(0)) + 0.7152 * channel(byte(2)) + 0.0722 * channel(byte(4))
    }

    /// The contrast ratio between two colours, WCAG 2.2's definition.
    fn ratio(one: &str, other: &str) -> f64 {
        let (a, b) = (luminance(one), luminance(other));
        let (high, low) = if a > b { (a, b) } else { (b, a) };
        (high + 0.05) / (low + 0.05)
    }

    /// Looks a token up, failing the test rather than defaulting when the
    /// stylesheet does not define it.
    fn token(theme: &BTreeMap<String, String>, name: &str) -> String {
        theme
            .get(name)
            .unwrap_or_else(|| panic!("`--{name}` is not defined in this theme"))
            .clone()
    }

    /// Asserts a foreground clears its bar against every ground it can land
    /// on, in both themes.
    fn on_every_ground(name: &str, bar: f64) {
        for (theme_name, theme) in [("light", light()), ("dark", dark())] {
            let foreground = token(&theme, name);
            for ground in GROUNDS {
                let measured = ratio(&foreground, &token(&theme, ground));
                assert!(
                    measured >= bar,
                    "{theme_name}: --{name} on --{ground} is {measured:.2}, under {bar:.1}"
                );
            }
        }
    }

    /// Asserts one declared pair clears its bar in both themes.
    fn pair(foreground: &str, background: &str, bar: f64) {
        for (theme_name, theme) in [("light", light()), ("dark", dark())] {
            let measured = ratio(&token(&theme, foreground), &token(&theme, background));
            assert!(
                measured >= bar,
                "{theme_name}: --{foreground} on --{background} is {measured:.2}, \
                 under {bar:.1}"
            );
        }
    }

    #[test]
    fn the_parse_finds_both_themes_and_they_declare_the_same_tokens() {
        let (light, dark) = (light(), dark());
        assert!(light.len() >= 18, "the light theme parsed as {light:?}");
        let light_names: Vec<&String> = light.keys().collect();
        let dark_names: Vec<&String> = dark.keys().collect();
        assert_eq!(
            light_names, dark_names,
            "a token defined in one theme and not the other leaves a screen half themed"
        );
    }

    #[test]
    fn the_measurement_agrees_with_the_two_ratios_wcag_states_itself() {
        assert!((ratio("#ffffff", "#000000") - 21.0).abs() < 0.01);
        assert!((ratio("#ffffff", "#ffffff") - 1.0).abs() < 0.01);
    }

    #[test]
    fn every_text_token_carries_text_on_every_ground_it_can_land_on() {
        on_every_ground("ink", TEXT);
        on_every_ground("ink-muted", TEXT);
        on_every_ground("accent", TEXT);
        on_every_ground("accent-hover", TEXT);
        on_every_ground("ok", TEXT);
        on_every_ground("warn", TEXT);
        on_every_ground("danger", TEXT);
    }

    #[test]
    fn a_control_boundary_and_a_disabled_value_still_read() {
        // 1.4.11: an input's border is the only thing drawing the control.
        on_every_ground("edge-strong", GRAPHICS);
        // 1.4.3 exempts an inactive control, so this one is legibility
        // rather than conformance, and the bar is the graphics one.
        on_every_ground("ink-faint", GRAPHICS);
    }

    #[test]
    fn text_on_a_tinted_fill_carries_text() {
        pair("accent-ink", "accent-subtle", TEXT);
        pair("ok", "ok-subtle", TEXT);
        pair("warn", "warn-subtle", TEXT);
        pair("danger", "danger-subtle", TEXT);
    }

    #[test]
    fn text_on_a_solid_accent_carries_text() {
        pair("on-accent", "accent", TEXT);
    }

    #[test]
    fn the_two_themes_do_not_share_a_ground() {
        let (light, dark) = (light(), dark());
        for ground in GROUNDS {
            assert_ne!(
                token(&light, ground),
                token(&dark, ground),
                "--{ground} is the same colour in both themes"
            );
        }
    }
}
