//! Bottom navbar component for switching between the main pages.

use yew::prelude::*;
use yew::MouseEvent;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::router::Route;

/// Convenience wrapper for views that want to include a Navbar.
///
/// It ensures long contents are visible from behind the navbar after scrolling.
/// We add the margin symmetrically to the top so the content remains centered
/// in the screen.
#[function_component]
pub fn NavbarWrapper(props: &NavbarWrapperProps) -> Html {
    html! {
        <div class="flex flex-col h-screen">
            // flex-1: take full height, pushing navbar to the bottom
            // overflow-y-auto: overflow this div's content (would otherwise
            //          overflow the navbar too)
            <div class="flex-1 overflow-y-auto">
                    { for props.children.iter() }
            </div>
            <Navbar />
        </div>
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

    // Retrieve the scope from the url
    let scope = Route::get_scope();

    // We scale the icons differently since their visual size for the same width
    // is sometimes different.
    let view_routes = vec![
        // (route, label, icon, icon_scale)
        (
            Route::Sense {
                scope: scope.clone(),
            },
            "Log",
            IconId::BootstrapJournalText,
            "h-6 w-6",
        ),
        (
            Route::Analyze { scope },
            "Map",
            IconId::BootstrapGlobeAmericas,
            "h-6 w-6",
        ),
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
        let mut style = vec!["pt-2 pb-6"];
        if let Some(r) = &curr_route {
            // Style the current button blue if we are on it
            if *route == *r {
                style.push("text-primary");
            }
        }
        let icon_style: String = format!("mx-auto mb-1 {}", icon_scale);
        html! {
            // id makes it easier to automatically select the buttons in
            // integration testing
            <button {onclick} class={style} id={label.to_string()}>
                <Icon icon_id={*icon} class={classes!(icon_style)} />
                { label }
            </button>
        }
    });
    html! {
        <div class="flex-none grid grid-cols-2 justify-items-stretch">
            {for items}
        </div>
    }
}
