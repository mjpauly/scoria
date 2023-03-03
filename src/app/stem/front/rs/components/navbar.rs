//! Bottom navbar component for switching between the main pages.

use crate::router::Route;
use yew::prelude::*;
use yew::MouseEvent;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

/// Convenience wrapper for views that want to include a Navbar.
///
/// It ensures long contents are visible from behind the navbar after scrolling.
/// We add the margin symmetrically to the top so the content remains centered
/// in the screen.
#[function_component]
pub fn NavbarWrapper(props: &NavbarWrapperProps) -> Html {
    html! {
        <>
            <div class="my-32">
                { for props.children.iter() }
            </div>
            <Navbar />
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct NavbarWrapperProps {
    pub children: Children, // the field name `children` is important!
}

/// The Navbar component itself.
///
/// It is available for views that want to customize their top and bottom
/// margins.
#[function_component]
pub fn Navbar() -> Html {
    let navigator = use_navigator().unwrap();
    let curr_route: Option<Route> = use_route();

    // We scale the icons differently since their visual size for the same width
    // is different.
    let view_routes = vec![
        // (route, label, icon, icon_scale)
        (Route::Sense, "Sense", IconId::BootstrapSoundwave, "h-8 w-8"),
        (Route::Analyze, "Analyze", IconId::BootstrapMap, "h-6 w-6"),
        (Route::TestPage, "Test", IconId::BootstrapTools, "h-6 w-6"),
    ];

    // Construct each button to display in the navbar
    let items = view_routes.iter().map(|view| {
        let (route, label, icon, icon_scale) = view;
        // Make the on-click callback to pass to the button element
        let onclick = {
            let navigator = navigator.clone();
            let route = route.clone();
            Callback::from(move |_e: MouseEvent| navigator.push(&route))
        };
        // Extra padding on the bottom to give more room for the home bar
        let mut style = vec!["pt-4 pb-8"];
        if let Some(r) = &curr_route {
            // Style the current button blue if we are on it
            if *route == *r {
                style.push("text-sky-500");
            }
        }
        let icon_style: String = format!("mx-auto mb-1 {}", icon_scale);
        html! {
            <button {onclick} class={style}>
                <Icon icon_id={*icon} class={classes!(icon_style)} />
                { label }
            </button>
        }
    });
    html! {
        <nav class="fixed inset-x-0 bottom-0 bg-neutral-900 \
                    grid grid-cols-3 justify-items-stretch">
            {for items}
        </nav>
    }
}
