//! Navigation components for switching between pages. px are used where buffer
//! is created for the device's home bar and top notch.

use yew::prelude::*;
use yew::MouseEvent;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

use crate::router::Route;

/// Top navigation bar, with room for the device's top notch. Buttons are passed
/// in as props.
#[function_component]
pub fn TopNav(props: &NavProps) -> Html {
    html! {
        <nav class="sticky top-0 backdrop-blur-xl bg-black/20 z-20 \
            flex flex-col">
            // px don't scale with dynamic font size, which is good since
            // this div is just there to ensure the menu items are below
            // the device's top notch
            //
            // 20px is the buffer for square screen sizes, like the iPhone SE,
            // and 48 for rounded screens like the other iPhone models. See
            // the tailwind config for the definition.
            <div class="h-[20px] tall:h-[48px]"></div>
            <div class="flex justify-between w-full">
                { for props.children.iter() }
            </div>
        </nav>
    }
}

/// Bottom navigation bar, with room for the device's home bar. Buttons are
/// passed in as props.
#[function_component]
pub fn BottomNav(props: &NavProps) -> Html {
    html! {
        <nav class="sticky bottom-0 backdrop-blur-xl bg-black/20 z-20 \
            flex flex-col">
            <div class="flex justify-between w-full">
                { for props.children.iter() }
            </div>
            <div class="tall:h-[24px]"></div>
        </nav>
    }
}

#[derive(Properties, PartialEq)]
pub struct NavProps {
    pub children: Children, // the field name `children` is important!
}

/// The main tab navigation bar at the bottom of the screen.
#[function_component]
pub fn TabBar() -> Html {
    let navigator = use_navigator().unwrap();
    let curr_route: Option<Route> = use_route();

    // We scale the icons differently since their visual size for the same width
    // is sometimes different.
    let view_routes = vec![
        // (route, label, icon, icon_scale)
        (Route::Sense, "Log", IconId::BootstrapJournalText, "h-6 w-6"),
        (
            Route::Analyze,
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
        // bottom padding is not scaled with rem since we don't need it to
        // change with font size
        let mut style = vec!["pt-2 tall:pb-[24px]"];
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
        <nav class="sticky bottom-0 backdrop-blur-xl bg-black/20 z-20 \
            grid grid-cols-2 justify-items-stretch">
            {for items}
        </nav>
    }
}

/// Spacer for room for the homebar to add to a flex-col layout. Ensures there's
/// enough space at the bottom of a page for the homebar to not cover elements.
#[function_component]
pub fn HomeBarSpacer() -> Html {
    html! {
        <div class="tall:h-[30px]">
        </div>
    }
}
