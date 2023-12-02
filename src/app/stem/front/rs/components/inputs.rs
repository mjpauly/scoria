//! Input components

use yew::prelude::*;

use crate::components::TOGGLE_SWITCH_STYLE;

pub const SHORT_INPUT_STYLE: &str =
    "rounded bg-black border border-neutral-700";

#[derive(Properties, PartialEq)]
pub struct ToggleSwitchProps {
    pub checked: bool,                 // the current state
    pub onclick: Callback<MouseEvent>, // the callback to emit
    #[prop_or_default]
    pub class: Classes,
}

#[function_component]
pub fn ToggleSwitch(p: &ToggleSwitchProps) -> Html {
    html! {
        <div
            class={classes!(
                Classes::from("relative h-6"),
                p.class.clone()
            )}
        >
            <input
                type="checkbox"
                checked={p.checked}
                onclick={p.onclick.clone()}
                class={TOGGLE_SWITCH_STYLE}
            />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ShortInputProps {
    pub value: String,              // value contents
    pub onchange: Callback<String>, // the callback to emit
    #[prop_or_default]
    pub class: Classes,
}

#[function_component]
pub fn ShortInput(p: &ShortInputProps) -> Html {
    let callback = {
        let onchange = p.onchange.clone();
        let value = p.value.clone();
        Callback::from(move |e: Event| {
            let elem: web_sys::HtmlInputElement = e.target_dyn_into().unwrap();
            onchange.emit(elem.value());
            // reset to previous value by default, if the new change is valid,
            // the element will be re-rendered with it.
            elem.set_value(&value);
        })
    };
    html! {
        <input
            class={classes!(
                Classes::from(SHORT_INPUT_STYLE),
                p.class.clone()
            )}
            onchange={callback}
            value={p.value.clone()}
        />
    }
}
