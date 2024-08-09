//! Toast messages to show over any part of the UI, usually messages from the
//! backend.

use std::time::Duration;

use yew::prelude::*;
use yew_icons::{Icon, IconId};

use common::popups::*;

use crate::components::warning::*;
use crate::websocket::{use_backend_event_with_deps, ToFront};

#[function_component]
pub fn Toast() -> Html {
    let message = use_state(|| Option::<PopUp>::None);
    let counter = use_state(|| 0); // tracks message number
    let expiry_counter = use_state(|| 0); // expired message number

    let on_message = {
        let message = message.clone();
        let counter = counter.clone();
        let expiry_counter = expiry_counter.clone();
        move |msg: &ToFront| {
            if let ToFront::PopUp(popup) = msg {
                // show the new message
                message.set(None);
                let popup = popup.clone();
                let message = message.clone();
                yew::platform::spawn_local(async move {
                    // re-add the new message after a short delay so the build-
                    // in animation runs again
                    yew::platform::time::sleep(Duration::from_millis(10)).await;
                    message.set(Some(popup));
                });
                // update showed message counter
                let new_counter = *counter + 1;
                counter.set(new_counter);
                let expiry_counter = expiry_counter.clone();
                yew::platform::spawn_local(async move {
                    // only one duration supported, since it's unclear how to
                    // easily get updated state here in the detached future
                    yew::platform::time::sleep(Duration::from_millis(4000))
                        .await;
                    expiry_counter.set(new_counter);
                });
            }
        }
    };
    use_backend_event_with_deps(on_message, (message.clone(), counter.clone()));
    let toast = message.as_ref().map(|m| {
        let inner = html! {
            <div class="flex gap-4">
                <span> {m.msg.clone()} </span>
                <Icon
                    icon_id={IconId::BootstrapX}
                    class="h-5 w-5 my-auto"
                />
            </div>
        };
        let class = "animate-appear2";
        match m.kind {
            PopUpKind::Error => {
                html! {
                    <ErrorMessage class={class}>
                        {inner}
                    </ErrorMessage>
                }
            }
            PopUpKind::Warn => {
                html! {
                    <WarningMessage class={class}>
                        {inner}
                    </WarningMessage>
                }
            }
            PopUpKind::Info => html! {
                <InfoMessage class={class}>
                    {inner}
                </InfoMessage>
            },
            PopUpKind::Success => {
                html! {
                    <SuccessMessage class={class}>
                        {inner}
                    </SuccessMessage>
                }
            }
        }
    });
    let onclick = {
        let message = message.clone();
        Callback::from(move |_: MouseEvent| {
            message.set(None);
        })
    };
    html! {
        <div
            class="fixed left-1/2 top-12 -translate-x-1/2 w-max max-w-[90%] \
                z-50"
            onclick={onclick}
        >
            if *expiry_counter != *counter {
                {toast}
            }
        </div>
    }
}
