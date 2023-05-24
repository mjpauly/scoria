//! Page router. Shows different pages according to the URL extension.

use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::pages;
use crate::ui_state::FrontState;
use common::state::PersistedRoute;

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
pub fn switch(route: Route) -> Html {
    // save the route in the app state, so it can be retrieved at startup again
    let persist_route = || {
        let dispatch = Dispatch::<FrontState>::new();
        dispatch.reduce_mut(|state| state.route = route.as_persisted_route());
    };
    match route {
        Route::Sense { .. } => persist_route(),
        Route::Analyze { .. } => persist_route(),
        _ => (),
    }
    match route {
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

/// PersistedRoute defines the route that is persisted between app launches.
/// Certain routes are not be persisted in the backend (Splash, NotFound,
/// TestPage).
impl Route {
    pub fn from_persisted_route(r: &PersistedRoute) -> Self {
        let scope = get_scope();
        match *r {
            PersistedRoute::Sense => Self::Sense { scope },
            PersistedRoute::Analyze => Self::Analyze { scope },
        }
    }

    fn as_persisted_route(&self) -> PersistedRoute {
        match self {
            Route::Sense { .. } => PersistedRoute::Sense,
            Route::Analyze { .. } => PersistedRoute::Analyze,
            // should not get here, but if we do, persist Sense
            _ => PersistedRoute::Sense,
        }
    }
}
