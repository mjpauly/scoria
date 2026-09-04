//! Navigation components for switching between pages. Buffer for the device's
//! home bar and top notch comes from the --safe-area-* insets (styles.css).
//! On Android the WebView's margins already avoid the system navigation bar,
//! so its injected bottom inset is effectively zero.

use yew::prelude::*;
use yew::MouseEvent;
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::router::Route;
use crate::ui_state::FrontState;

// Spacing for bottom navigation (tabs or forward/backwards buttons) to not
// impinge on the homebar.
const BOTTOM_NAV_SPACE: &str = "h-[var(--safe-area-bottom)]";
const TAB_BAR_SPACE: &str = "pb-[var(--safe-area-bottom)]";

/// Top navigation bar, with room for the device's top notch. Buttons are passed
/// in as props.
#[function_component]
pub fn TopNav(props: &NavProps) -> Html {
    html! {
        <nav class="sticky top-0 backdrop-blur-xl bg-black/20 z-20 \
            flex flex-col">
            // ensures the menu items are below the device's top notch
            <div class="h-[var(--safe-area-top)]"></div>
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
            <div class={BOTTOM_NAV_SPACE}></div>
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

    // set globe icon irientation to current map view position
    let viewing_lng = use_selector(|s: &FrontState| s.map.view_pos.center.lng);
    let globe_icon = if *viewing_lng < -27.0 {
        IconId::BootstrapGlobeAmericas
    } else if *viewing_lng < 51.0 {
        IconId::BootstrapGlobeEuropeAfrica
    } else if *viewing_lng < 89.0 {
        IconId::BootstrapGlobeCentralSouthAsia
    } else {
        IconId::BootstrapGlobeAsiaAustralia
    };

    // We scale the icons differently since their visual size for the same width
    // is sometimes different.
    let default_hw = "h-5 w-5";
    let view_routes = vec![
        // (route, label, icon, icon_scale)
        (
            Route::Sense,
            "Log",
            IconId::BootstrapGearWideConnected,
            default_hw,
        ),
        (Route::Analyze, "Map", globe_icon, default_hw),
        (Route::Places, "Places", IconId::BootstrapGeoAlt, default_hw),
        (
            Route::Metrics,
            "Stats",
            IconId::BootstrapClipboard2Pulse,
            default_hw,
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
        let mut style = vec!["pt-2", TAB_BAR_SPACE];
        if let Some(r) = &curr_route {
            // Style the current button blue if we are on it
            if *route == *r {
                style.push("text-primary");
            }
        }
        let icon_style: String = format!("mx-auto mb-px {}", icon_scale);
        html! {
            // id makes it easier to automatically select the buttons in
            // integration testing
            <button {onclick} class={style} id={label.to_string()}>
                <Icon icon_id={*icon} class={classes!(icon_style)} />
                <span class="text-sm"> { label } </span>
            </button>
        }
    });
    html! {
        <nav class="sticky bottom-0 backdrop-blur-xl bg-black/20 z-20 \
            grid grid-cols-4 justify-items-stretch">
            {for items}
        </nav>
    }
}

/// Spacer for room for the homebar to add to a flex-col layout. Ensures there's
/// enough space at the bottom of a page for the homebar to not cover elements.
#[function_component]
pub fn HomeBarSpacer() -> Html {
    html! {
        <div class="h-[var(--safe-area-bottom)]">
        </div>
    }
}
