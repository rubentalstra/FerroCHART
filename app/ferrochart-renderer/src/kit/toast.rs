// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The toast: what the app says about something that already happened.
//!
//! One helper and one host. [`Toasts`] is the queue a screen pushes onto, and
//! [`ToastHost`] is the one live region that draws it. A screen that rolled
//! its own would be a second live region, and two live regions talk over each
//! other.
//!
//! Nothing dismisses itself on a timer. A toast that vanishes after four
//! seconds is unreadable to anyone who reads slowly, so each one carries a
//! dismiss button and stays until it is pressed.

use leptos::prelude::*;

use crate::icon::{Decoration, Glyph, Size};
use crate::kit::field::BTN_QUIET;
use crate::kit::tone::Tone;

/// One thing the app has to say.
#[derive(Clone, Debug)]
struct Toast {
    /// What tells this toast from the others in the queue.
    id: usize,
    /// What it says about what happened.
    tone: Tone,
    /// The message itself.
    text: String,
}

/// The queue of things the app has to say.
#[derive(Clone, Copy)]
pub(crate) struct Toasts {
    items: RwSignal<Vec<Toast>>,
    next: RwSignal<usize>,
}

impl Toasts {
    /// An empty queue.
    pub(crate) fn new() -> Self {
        Self {
            items: RwSignal::new(Vec::new()),
            next: RwSignal::new(0),
        }
    }

    /// Says one thing, which stays until a reader dismisses it.
    pub(crate) fn push(self, tone: Tone, text: impl Into<String>) {
        let id = self.next.get_untracked();
        self.next.set(id.saturating_add(1));
        self.items.update(|queue| {
            queue.push(Toast {
                id,
                tone,
                text: text.into(),
            });
        });
    }

    /// Forgets one message.
    fn dismiss(self, id: usize) {
        self.items
            .update(|queue| queue.retain(|toast| toast.id != id));
    }

    /// How many messages are showing.
    pub(crate) fn len(self) -> usize {
        self.items.read().len()
    }
}

/// The one live region every toast is drawn in.
#[component]
pub(crate) fn ToastHost(
    /// The queue to draw.
    toasts: Toasts,
) -> impl IntoView {
    view! {
        <div
            aria-live="polite"
            aria-label="Notifications"
            class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 max-w-full flex-col gap-2"
        >
            <For each=move || toasts.items.get() key=|toast| toast.id let(toast)>
                <div
                    role=toast.tone.live_role()
                    class=format!(
                        "pointer-events-auto flex items-start gap-2 rounded-card border px-3 py-2 \
                        text-sm shadow-card {}",
                        toast.tone.subtle(),
                    )
                >
                    <span class="mt-0.5">
                        <Glyph icon=toast.tone.icon() size=Size::Inline label=toast.tone.name() />
                    </span>
                    <p class="min-w-0 flex-1">{toast.text.clone()}</p>
                    <button
                        type="button"
                        class=format!("{BTN_QUIET} h-6 w-6")
                        aria-label="Dismiss"
                        on:click=move |_| toasts.dismiss(toast.id)
                    >
                        <Decoration icon=icondata_lu::LuX size=Size::Inline />
                    </button>
                </div>
            </For>
        </div>
    }
}
