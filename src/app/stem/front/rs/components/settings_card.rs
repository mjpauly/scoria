//! A set of components for a simple settings card.

use std::fmt::{Debug, Display};
use std::str::FromStr;

use yew::prelude::*;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::components::{Select, ShortInput, ToggleSwitch};

// A line on a settings card that is a horizontal flexbox that takes the full
// width and separates the elements to the ends of the line.
const LINE_FLEX_STYLE: &str = "flex items-center justify-between py-2 w-full";

#[derive(Properties, PartialEq)]
pub struct ChildenAndClasses {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes,
}

/// The top-level container for each line. Accepts child elements and css
/// classes.
#[function_component]
pub fn SettingsCard(p: &ChildenAndClasses) -> Html {
    // Turn the children into a keyed list, which is better when manipulating
    // the children with iterators, which are edge cases in yew where things can
    // break down. In this case, not doing this causes the children to be
    // rendered out of order. More info:
    // https://yew.rs/docs/next/concepts/html/lists
    // https://github.com/yewstack/yew/issues/3256#issuecomment-1570335954
    let children = p.children.iter().enumerate().map(|(i, x)| {
        html! { <div key={i}> {x} </div> }
    });
    // intersperce children with <hr> elements to create a separator
    let children = itertools::Itertools::intersperse(
        children,
        html! { <hr class="border-neutral-800" /> },
    );
    html! {
        <div class={classes!(
            Classes::from("bg-neutral-900 rounded-lg px-4"),
            p.class.clone()
        )}>
            { for children }
        </div>
    }
}

/// A paragraph following the settings card.
#[function_component]
pub fn AfterCardParagraph(p: &ChildenAndClasses) -> Html {
    html! {
        <p class={classes!(
                Classes::from("text-neutral-500 text-left mx-2 mt-2"),
                p.class.clone()
            )}>
            { for p.children.iter() }
        </p>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardButtonProps {
    pub onclick: Callback<MouseEvent>,
    pub text: String,
}

/// A generic blue-text button embedded on a settings line.
#[function_component]
pub fn SettingsCardSimpleButton(p: &SettingsCardButtonProps) -> Html {
    html! {
        <SettingsCardButtonWithChildren onclick={p.onclick.clone()} >
            <label class="text-primary">
                { p.text.clone() }
            </label>
        </SettingsCardButtonWithChildren>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardButtonWithChildrenProps {
    pub children: Children, // the field name `children` is important!
    pub onclick: Callback<MouseEvent>,
}

/// A button embedded on a settings line, with contents provided as children.
#[function_component]
pub fn SettingsCardButtonWithChildren(
    p: &SettingsCardButtonWithChildrenProps,
) -> Html {
    html! {
        <button onclick={p.onclick.clone()} class={LINE_FLEX_STYLE} >
            { for p.children.iter() }
        </button>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardPageButtonProps<R: Routable> {
    pub text: String,
    pub route: R,
}

/// A button to go to another page with a piece of text provided.
#[function_component]
pub fn SettingsCardPageButton<R>(p: &SettingsCardPageButtonProps<R>) -> Html
where
    R: Routable + 'static,
{
    html! {
        <SettingsCardPageButtonWithLabel<R> route={p.route.clone()} >
            <label> { p.text.clone() } </label>
        </SettingsCardPageButtonWithLabel<R>>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardPageButtonWithLabelProps<R: Routable> {
    pub children: Children, // the field name `children` is important!
    pub route: R,
}

/// A button to go to another page, but more generic over possible labels, which
/// are supplied as children.
#[function_component]
pub fn SettingsCardPageButtonWithLabel<R>(
    p: &SettingsCardPageButtonWithLabelProps<R>,
) -> Html
where
    R: Routable + 'static,
{
    let navigator = use_navigator().unwrap();
    let onclick = {
        let route = p.route.clone();
        Callback::from(move |_e: MouseEvent| navigator.push(&route))
    };
    html! {
        <SettingsCardButtonWithChildren onclick={onclick} >
            { for p.children.iter() }
            <Icon icon_id={IconId::BootstrapChevronRight}
                class="h-4 w-4 text-neutral-500" />
        </SettingsCardButtonWithChildren>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardExternalLinkProps {
    pub text: String,
    pub href: String,
}

/// A link to an external webpage.
#[function_component]
pub fn SettingsCardExternalLink(p: &SettingsCardExternalLinkProps) -> Html {
    html! {
        <a
            class={LINE_FLEX_STYLE}
            href={p.href.clone()}
        >
            <label>{p.text.clone()}</label>
            <Icon icon_id={IconId::BootstrapBoxArrowUpRight}
                class="h-4 w-4 text-neutral-500" />
        </a>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardSelectProps<C>
where
    C: PartialEq,
{
    pub selection: C,          // the current selection
    pub choices: Vec<C>,       // list of choices to select from in order
    pub onchange: Callback<C>, // the callback to emit with the selection
    pub text: String,          // label text
}

/// A select from multiple options.
#[function_component]
pub fn SettingsCardSelect<C>(p: &SettingsCardSelectProps<C>) -> Html
where
    C: PartialEq + Clone + FromStr + Display + 'static,
    <C as FromStr>::Err: Debug,
{
    html! {
        <label class={LINE_FLEX_STYLE}>
            {p.text.clone()}
            <Select<C>
                selection={p.selection.clone()}
                choices={p.choices.clone()}
                onchange={p.onchange.clone()}
                class="ml-4"
            />
        </label>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardToggleProps {
    pub checked: bool,                 // the current state
    pub onclick: Callback<MouseEvent>, // the callback to emit
    pub text: String,                  // label text
}

#[function_component]
pub fn SettingsCardToggle(p: &SettingsCardToggleProps) -> Html {
    html! {
        <label class={LINE_FLEX_STYLE}>
            {p.text.clone()}
            <ToggleSwitch
                class={"ml-4 mr-1"}
                checked={p.checked}
                onclick={p.onclick.clone()}
            />
        </label>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsCardInputProps {
    pub text: String,               // the label for the input
    pub value: String,              // value contents
    pub onchange: Callback<String>, // the callback to emit
    #[prop_or_default]
    pub class: Classes, // classes to pass to the input itself, e.g. w-24
}

#[function_component]
pub fn SettingsCardInput(p: &SettingsCardInputProps) -> Html {
    html! {
        <label class={LINE_FLEX_STYLE}>
            {p.text.clone()}
            <ShortInput
                class={classes!(
                    Classes::from("ml-4"),
                    p.class.clone()
                )}
                value={p.value.clone()}
                onchange={p.onchange.clone()}
            />
        </label>
    }
}
