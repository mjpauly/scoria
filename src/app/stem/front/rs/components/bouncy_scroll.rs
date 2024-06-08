//! A bouncy scrolling container.

use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BouncyScrollContainerProps {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes,
}

/// To center content vertically within the bouncy scroll, add "flex flex-col"
/// to the container, then wrap content in a "my-auto" div.
#[function_component]
pub fn BouncyScrollContainer(p: &BouncyScrollContainerProps) -> Html {
    html! {
        <BouncyScrollContainerBase class="px-4">
            { for p.children.iter() }
        </BouncyScrollContainerBase>
    }
}

/// Boundy scroll container without the default px-4 padding.
#[function_component]
pub fn BouncyScrollContainerBase(p: &BouncyScrollContainerProps) -> Html {
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
        <div class={classes!(
            Classes::from("grow overflow-scroll h-0 w-full max-w-prose \
                          mx-auto"),
            p.class.clone()
        )}>
            { for p.children.iter() }
            // {for _big_list}
        </div>
    }
}
