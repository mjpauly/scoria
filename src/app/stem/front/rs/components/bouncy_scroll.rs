//! A bouncy scrolling container.

use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BouncyScrollContainerProps {
    pub children: Children, // the field name `children` is important!
}

#[function_component]
pub fn BouncyScrollContainer(props: &BouncyScrollContainerProps) -> Html {
    // let _big_list = (1..41).map(|i| html! { <div>{format!("{}", i)}</div> });
    html! {
        // Centered, width-limited content container.
        //
        // "grow overflow-scroll h-0" allow this element to elastically
        // scroll if it's too long to fully show.
        //
        // If we just do "flex-1" instead, we'll get the transparency effect
        // on sticky elements, but the scrolling won't be elastic. We could
        // have bouncy scrolling everywhere, but then that's unnatural for
        // elements that are supposed to by fixed/sticky (though it is the
        // norm for all mobile websites).
        <div class="grow overflow-scroll h-0 px-4 w-full max-w-prose mx-auto">
            { for props.children.iter() }
            // {for _big_list}
        </div>
    }
}
