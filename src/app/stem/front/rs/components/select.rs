//! Select component for choosing between multiple predefined options.

use std::fmt::{Debug, Display};
use std::str::FromStr;

use web_sys::HtmlSelectElement;
use yew::prelude::*;

pub const SELECT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-neutral-200 bg-neutral-800 \
        appearance-none";

#[derive(Properties, PartialEq)]
pub struct SelectProps<C>
where
    C: PartialEq,
{
    pub selection: C,          // the current selection
    pub choices: Vec<C>,       // list of choices to select from in order
    pub onchange: Callback<C>, // the callback to emit with the selection
    #[prop_or_default]
    pub class: Classes,
}

/// A select from multiple options. Wrap with a \<label\> tag to make the name
/// text clickable. We have the parent specify which choices to allow and what
/// order they should be in, so it's possible to have a subset of enum variants
/// to choose from instead of all of them.
#[function_component]
pub fn Select<C>(p: &SelectProps<C>) -> Html
where
    C: PartialEq + Clone + FromStr + Display + 'static,
    <C as FromStr>::Err: Debug, // needed for the unwrap()
{
    let onchange = {
        let callback = p.onchange.clone();
        Callback::from(move |e: Event| {
            let elem: HtmlSelectElement = e.target_dyn_into().unwrap();
            let val: &str = &elem.value();
            let new_choice = C::from_str(val).unwrap();
            callback.emit(new_choice);
        })
    };
    let choices = p.choices.iter().map(|x| {
        html! { <option> {x.to_string()} </option> }
    });
    let node_ref = use_node_ref();
    {
        let node_ref = node_ref.clone();
        // need to manually set select element value after render
        use_effect_with_deps(
            move |selection| {
                let e = node_ref.cast::<HtmlSelectElement>().unwrap();
                e.set_value(&(selection.to_string()));
            },
            p.selection.clone(),
        )
    }
    html! {
        <select
            class={classes!(
                Classes::from(SELECT_STYLE),
                p.class.clone()
            )}
            ref={node_ref}
            onchange={onchange}
        >
            { for choices }
        </select>
    }
}
