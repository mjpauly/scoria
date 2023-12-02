//! Components for typography: H1, H2, etc.

use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeadingProps {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes,
}

#[function_component]
pub fn H1(p: &HeadingProps) -> Html {
    html! {
        <h1
            class={classes!(
                Classes::from("font-bold text-3xl text-left mx-2 mt-8 mb-6"),
                p.class.clone()
            )}
        >
            { for p.children.iter() }
        </h1>
    }
}

#[function_component]
pub fn H2(p: &HeadingProps) -> Html {
    html! {
        <h2
            class={classes!(
                Classes::from("font-bold text-2xl text-left mx-2 mt-6 mb-4"),
                p.class.clone()
            )}
        >
            { for p.children.iter() }
        </h2>
    }
}
