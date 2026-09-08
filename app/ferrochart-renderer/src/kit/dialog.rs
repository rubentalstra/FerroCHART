// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The confirm dialog: the one modal this app opens, for the one question it
//! asks before destroying something.
//!
//! One definition. It is a `role="dialog"` with `aria-modal`, labelled by its
//! own heading and described by its own body. Opening it moves focus to the
//! cancel button, Escape closes it, Tab cannot leave it, and closing it puts
//! focus back on the control that opened it. Those four are the whole reason
//! a modal is hard, and doing them once here is why a screen never does them
//! again.

use leptos::ev::KeyboardEvent;
use leptos::html;
use leptos::prelude::*;

use crate::focus::{focus, focus_first, focused, trap};
use crate::kit::field::{BTN_DANGER, BTN_SECONDARY};

/// A question asked before something is destroyed.
#[component]
pub(crate) fn ConfirmDialog(
    /// Whether the dialog is showing.
    open: RwSignal<bool>,
    /// The question, as a heading.
    #[prop(into)]
    title: Signal<String>,
    /// What confirming does.
    #[prop(into)]
    body: Signal<String>,
    /// What the confirming button is called.
    #[prop(into)]
    confirm_label: String,
    /// What to do when the reader confirms.
    on_confirm: Callback<()>,
) -> impl IntoView {
    let panel: NodeRef<html::Div> = NodeRef::new();
    let opener = StoredValue::new_local(None::<web_sys::HtmlElement>);

    Effect::new(move |_| {
        if open.get() {
            opener.set_value(focused());
            focus_first(panel.get().map(Into::into));
        }
    });

    let close = move || {
        open.set(false);
        if let Some(element) = opener.get_value() {
            focus(&element);
        }
    };

    let on_key = move |event: KeyboardEvent| match event.key().as_str() {
        "Escape" => {
            event.prevent_default();
            close();
        }
        "Tab" => trap(&event, panel.get().map(Into::into)),
        _ => {}
    };

    view! {
        <Show when=move || open.get()>
            <div
                class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
                on:keydown=on_key
            >
                <div
                    node_ref=panel
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="confirm-title"
                    aria-describedby="confirm-body"
                    class="w-full max-w-sm rounded-card border border-edge bg-raised p-4 shadow-card"
                >
                    <h2 id="confirm-title" class="text-sm font-semibold text-ink">
                        {move || title.get()}
                    </h2>
                    <p id="confirm-body" class="mt-2 text-sm text-ink-muted">
                        {move || body.get()}
                    </p>
                    <div class="mt-4 flex justify-end gap-2">
                        <button type="button" class=BTN_SECONDARY on:click=move |_| close()>
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class=BTN_DANGER
                            on:click=move |_| {
                                on_confirm.run(());
                                close();
                            }
                        >
                            {confirm_label.clone()}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
