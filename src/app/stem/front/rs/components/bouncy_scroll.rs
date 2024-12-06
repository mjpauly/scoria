//! A bouncy scrolling container.

use web_sys::HtmlElement;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::ui_state::FrontState;

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
        <BouncyScrollContainerBase class={classes!(
            Classes::from("px-4"),
            p.class.clone()
        )}>
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
            Classes::from("grow overflow-y-scroll h-0 w-screen max-w-prose \
                          mx-auto"),
            p.class.clone()
        )}>
            { for p.children.iter() }
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct BouncySavedScrollContainerProps {
    pub children: Children, // the field name `children` is important!
    #[prop_or_default]
    pub class: Classes,
    pub id: AttrValue,
}

/// Bouncy scroll container with the ability to save its scroll position using a
/// provided id.
#[function_component]
pub fn BouncySavedScrollContainer(p: &BouncySavedScrollContainerProps) -> Html {
    let id = p.id.to_string();

    // save scroll position on scroll
    let dispatch = Dispatch::<FrontState>::new();
    let onscroll = {
        let id = id.clone();
        dispatch.reduce_mut_callback_with(
            move |s: &mut FrontState, e: Event| {
                let elem: HtmlElement = e.target_dyn_into().unwrap();
                s.scroll_positions.insert(id.clone(), elem.scroll_top());
            },
        )
    };

    // restore scroll position, only on first render
    use_effect_with_deps(
        move |_| {
            let pos = dispatch.get().scroll_positions.get(&id).cloned();
            if let Some(pos) = pos {
                let window = web_sys::window().unwrap();
                let element =
                    window.document().unwrap().get_element_by_id(&id).unwrap();
                element.set_scroll_top(pos);
            }
        },
        (), // no deps bc it should only happen on first render
    );
    html! {
        <div
            class={classes!(
                Classes::from("grow overflow-scroll h-0 w-full mx-auto"),
                p.class.clone()
            )}
            onscroll={onscroll}
            id={p.id.clone()}
        >
            { for p.children.iter() }
        </div>
    }
}
