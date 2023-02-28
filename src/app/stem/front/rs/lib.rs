//! Frontend library containing all components. A small main.rs file starts up
//! the app.

use yew::prelude::*;
use yew::MouseEvent;
use yew_router::prelude::*;

use analyze::Analyze;
use sense::Sense;
use test_page::TestPage;

mod analyze;
mod components;
mod sense;
mod test_page;

/// Top level App component for the UI.
#[function_component]
pub fn App() -> Html {
    html! {
        <BrowserRouter>
            // default parent style for the UI
            <div class="place-content-center text-center flex flex-col \
                        min-h-screen bg-neutral-900 \
                        font-light text-neutral-100">
                // child views implement the navbar for switching views
                <Switch<Route> render={switch} />
            </div>
        </BrowserRouter>
    }
}

/// The possible routes for our app.
///
/// While users don't interact with the URL on mobile, encoding the different
/// views with it has the benefit of keeping the view the same when trunk
/// reloads the app after changes to the code, improving the development
/// experience.
#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Sense,

    #[at("/analyze")]
    Analyze,

    // Test UI edge cases
    #[at("/test_page")]
    TestPage,

    #[not_found]
    #[at("/404")]
    NotFound,
}

/// Switches what is displayed based on the route URL.
fn switch(routes: Route) -> Html {
    match routes {
        // Route::Sense => html! { <h1>{ "Home" }</h1> },
        Route::Sense => html! { <Sense /> },
        Route::Analyze => html! { <Analyze /> },
        Route::TestPage => html! {
            <TestPage />
        },
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}

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
/// It is available for views that want to customize their margins.
#[function_component]
pub fn Navbar() -> Html {
    let navigator = use_navigator().unwrap();
    let curr_route: Option<Route> = use_route();

    let view_routes = vec![
        (Route::Sense, "Sense"),
        (Route::Analyze, "Analyze"),
        (Route::TestPage, "Test"),
    ];

    // Construct each button to display in the navbar
    let items = view_routes.iter().map(|view| {
        let (route, name) = view;
        // Make the on-click callback to pass to the button element
        let onclick = {
            let navigator = navigator.clone();
            let route = route.clone();
            Callback::from(move |_e: MouseEvent| navigator.push(&route))
        };
        // Style the current button blue if we are on it
        let mut style = vec!["py-8"];
        if let Some(r) = &curr_route {
            if *route == *r {
                style.push("text-sky-500");
            }
        }
        html! { <button {onclick} class={style}> { name } </button> }
    });
    html! {
        <nav class="fixed inset-x-0 bottom-0 bg-neutral-900 \
                    grid grid-cols-3 justify-items-stretch">
            {for items}
        </nav>
    }
}

// use stylist::{css, style, yew::styled_component};

// use components::CappedInputComponent;
// use components::ListComponent;
//
// Testing out Yew
// <div class="py-4" />
// <CappedInputComponent min_value={0} max_value={20}/>
// <CappedInputComponent min_value={5} max_value={30}/>
// <ListComponent />
