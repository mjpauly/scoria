//! A persistent warning message.
//!
//! A good source of examples:
//! <https://tailwindui.com/components/application-ui/feedback/alerts>

use yew::prelude::*;
use yew_icons::{Icon, IconId};

const CONTAINER_STYLE: &str =
    "flex items-start justify-start px-4 py-2 rounded-xl";
const ICON_STYLE: &str = "h-4 w-4 mr-5 ml-1 my-auto";

// color of button in an info message
pub const INFO_BUTTON_BG: &str = "bg-[rgb(10,45,81)]";

#[derive(Properties, PartialEq)]
pub struct ChildrenAndClasses {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes,
}

#[function_component]
pub fn WarningMessage(p: &ChildrenAndClasses) -> Html {
    html! {
        <IconMessage
            class={p.class.clone()}
            bg_color="bg-[rgb(47,43,2)]"
            icon_color="text-yellow-400"
            text_color="text-yellow-500"
            icon={IconId::BootstrapExclamationTriangleFill}
        >
            { for p.children.iter() }
        </IconMessage>

    }
}

#[function_component]
pub fn ErrorMessage(p: &ChildrenAndClasses) -> Html {
    html! {
        <IconMessage
            class={p.class.clone()}
            bg_color="bg-[rgb(55,4,4)]"
            icon_color="text-red-400"
            text_color="text-red-500"
            icon={IconId::BootstrapXCircleFill}
        >
            { for p.children.iter() }
        </IconMessage>

    }
}

#[function_component]
pub fn SuccessMessage(p: &ChildrenAndClasses) -> Html {
    html! {
        <IconMessage
            class={p.class.clone()}
            bg_color="bg-[rgb(7,54,30)]"
            icon_color="text-green-400"
            text_color="text-green-500"
            icon={IconId::BootstrapCheckCircleFill}
        >
            { for p.children.iter() }
        </IconMessage>

    }
}

#[function_component]
pub fn InfoMessage(p: &ChildrenAndClasses) -> Html {
    html! {
        <IconMessage
            class={p.class.clone()}
            bg_color="bg-[rgb(7,30,54)]"
            icon_color="text-blue-400"
            text_color="text-blue-500"
            icon={IconId::BootstrapInfoCircleFill}
        >
            { for p.children.iter() }
        </IconMessage>

    }
}

/// colors are specified as Tailwind CSS classes
#[derive(Properties, PartialEq)]
pub struct IconMessageProps {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes, // classes to apply to the top-level container
    pub bg_color: Classes,
    pub text_color: Classes,
    pub icon_color: Classes,
    pub icon: IconId,
}

#[function_component]
pub fn IconMessage(p: &IconMessageProps) -> Html {
    html! {
        <div
            class={classes!(
                Classes::from(CONTAINER_STYLE),
                p.bg_color.clone(),
                p.class.clone()
            )}
        >
            <Icon
                icon_id={p.icon}
                class={classes!(
                    Classes::from(ICON_STYLE),
                    p.icon_color.clone(),
                )}
            />
            <div class={classes!("text-left", p.text_color.clone())}>
                {for p.children.iter()}
            </div>
        </div>
    }
}
