//! Page router. Shows different pages according to the URL extension.

use crate::components::NavbarWrapper;
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
pub fn switch(routes: Route) -> Html {
    match routes {
        // Route::Sense => html! { <h1>{ "Home" }</h1> },
        Route::Sense => html! { <pages::Sense /> },
        Route::Analyze => html! { <pages::Analyze /> },
        Route::TestPage => html! { <pages::TestPage /> },
        Route::NotFound => html! {
            <NavbarWrapper>
                <h1 class="text-sky-500 text-3xl mb-6">
                    { "404" }
                </h1>
            </NavbarWrapper>
        },
    }
}
