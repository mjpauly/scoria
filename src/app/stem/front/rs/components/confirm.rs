//! An html-only version of window.confirm()

use yew::prelude::*;
// use yew_icons::{Icon, IconId};

#[derive(Properties, PartialEq)]
pub struct ConfirmActionProps {
    pub title: AttrValue,
    pub visible: UseStateHandle<bool>,
    pub callback: Callback<()>,
}

/// Confirm that avoids a little more boilerplate. Visible is the state handle
/// for whether the Confirm should be shown, which must be set to true by the
/// parent. It will be hidden automatically when the user takes action. The
/// callback is emitted when the user selects "OK".
#[function_component]
pub fn ConfirmAction(p: &ConfirmActionProps) -> Html {
    let ok_onclick = {
        let cb = p.callback.clone();
        let visible = p.visible.clone();
        Callback::from(move |_e| {
            cb.emit(());
            visible.set(false);
        })
    };
    let cancel_onclick = {
        let visible = p.visible.clone();
        Callback::from(move |_e| {
            visible.set(false);
        })
    };

    html! {
        if *p.visible {
            <Confirm
                title={&p.title}
                ok={ok_onclick}
                cancel={cancel_onclick}
            />
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ConfirmProps {
    pub title: AttrValue,
    pub cancel: Callback<MouseEvent>,
    pub ok: Callback<MouseEvent>,
}

#[function_component]
pub fn Confirm(p: &ConfirmProps) -> Html {
    html! {
        <div
            class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 \
            flex flex-col bg-neutral-900 rounded-lg px-8 py-4 gap-4 \
            animate-appear border border-neutral-700"
        >
            <span class="text-center text-lg font-normal">
                {p.title.clone()}
            </span>
            <div
                class="flex justify-between gap-16"
            >
                <button class="px-4 py-2 text-primary rounded-lg \
                    bg-neutral-800 font-medium active:bg-neutral-700"
                    onclick={p.cancel.clone()}
                >
                    {"Cancel"}
                </button>
                <button class="px-4 py-2 text-primary rounded-lg \
                    bg-neutral-800 active:bg-neutral-700"
                    onclick={p.ok.clone()}
                >
                    {"OK"}
                </button>
            </div>
        </div>
    }
}
