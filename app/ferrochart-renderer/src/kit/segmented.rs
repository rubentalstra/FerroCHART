// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The segmented control: a short, closed set of which exactly one is chosen.
//!
//! One definition. It is a real radio group in a `<fieldset>`, so the arrow
//! keys move the choice, Tab enters and leaves the group once, and the
//! stylesheet's one `:focus-visible` rule draws the indicator on a control the
//! reader can see. A row of buttons with `aria-checked` would have to
//! re-implement all three.
//!
//! Use it up to about four options. Past that a `<select>` is faster to
//! operate and does not wrap.

use leptos::prelude::*;

/// One segment.
#[derive(Clone, Debug)]
pub(crate) struct Segment {
    /// What the segment stands for, which is what the callback receives.
    pub(crate) value: String,
    /// What the segment is called.
    pub(crate) label: String,
    /// Whether the segment is offered at all.
    ///
    /// A segment the constraint refuses is drawn disabled rather than
    /// dropped, so a reader sees that the template decided it.
    pub(crate) disabled: bool,
}

impl Segment {
    /// A segment a reader may choose.
    pub(crate) fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// This segment, drawn but refused.
    pub(crate) fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// A closed set of which exactly one is chosen.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Leptos component owns its props, and this one hands clones of them to the closures it draws"
)]
pub(crate) fn Segmented(
    /// The radio group's name, unique on the page.
    #[prop(into)]
    name: String,
    /// What the group as a whole is called.
    #[prop(into)]
    legend: String,
    /// Whether the legend is drawn, or only read aloud.
    #[prop(optional)]
    show_legend: bool,
    /// The segments, in the order they are drawn.
    segments: Vec<Segment>,
    /// Which segment is chosen, by its value.
    chosen: Signal<Option<String>>,
    /// What to do when a reader chooses one.
    on_choose: Callback<String>,
) -> impl IntoView {
    let group = name.clone();
    let options: Vec<_> = segments
        .into_iter()
        .map(|segment| {
            let value = segment.value.clone();
            let picked = segment.value.clone();
            let is_on = Signal::derive(move || chosen.get().as_deref() == Some(value.as_str()));
            let sent = segment.value.clone();
            view! {
                <label
                    class="inline-flex items-center gap-1.5 rounded-control border px-2.5 py-1.5 \
                    text-sm"
                    class=(["border-accent", "bg-accent-subtle", "text-accent-ink"], is_on)
                    class=(["border-edge-strong", "bg-raised", "text-ink"], move || !is_on.get())
                    class=("opacity-60", segment.disabled)
                >
                    <input
                        type="radio"
                        class="h-3.5 w-3.5 accent-accent"
                        name=group.clone()
                        value=picked
                        disabled=segment.disabled
                        prop:checked=is_on
                        on:change={
                            let sent = sent.clone();
                            move |_| on_choose.run(sent.clone())
                        }
                    />
                    {segment.label}
                </label>
            }
        })
        .collect();

    view! {
        <fieldset class="min-w-0">
            <legend class=if show_legend {
                "mb-1 text-xs font-medium text-ink-muted"
            } else {
                "sr-only"
            }>{legend}</legend>
            <div class="flex flex-wrap gap-1.5">{options}</div>
        </fieldset>
    }
}
