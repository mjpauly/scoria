//! Page router. Shows different pages according to the URL extension.

use crate::pages;
use yew::prelude::*;
use yew_router::prelude::*;

/// The possible routes for our app.
///
/// While users don't interact with the URL on mobile, encoding the different
/// views with it has the benefit of keeping the view the same when trunk
/// reloads the app after changes to the code, improving the development
/// experience.
#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/:scope/")]
    Splash { scope: String },

    #[at("/:scope/sense/")]
    Sense { scope: String },

    #[at("/:scope/analyze/")]
    Analyze { scope: String },

    // Test UI edge cases
    #[at("/:scope/test_page/")]
    TestPage { scope: String },

    #[not_found]
    #[at("/404/")]
    NotFound,
}

/// Switches what is displayed based on the route URL.
pub fn switch(routes: Route) -> Html {
    match routes {
        // Route::Sense => html! { <h1>{ "Home" }</h1> },
        Route::Sense { .. } => html! { <pages::Sense /> },
        Route::Splash { .. } => html! { <pages::Splash /> },
        Route::Analyze { .. } => html! { <pages::Analyze /> },
        Route::TestPage { .. } => html! { <pages::TestPage /> },
        Route::NotFound { .. } => html! {
                <h1 class="text-primary text-3xl mb-6 mt-16">
                    { "Something went wrong" }
                </h1>
        },
    }
}

/// Retrieve the scope from the current url, so that we can pass it to a route.
pub fn get_scope() -> String {
    let location = web_sys::window().unwrap().location();
    let binding = location.pathname().unwrap();
    binding
        .trim_matches('/')
        .split('/')
        .next()
        .unwrap()
        .to_string()
}
