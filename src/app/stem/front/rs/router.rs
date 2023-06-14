//! Page router. Shows different pages according to the URL extension.

use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::pages::{self, intro::INTRO_VERSION};
use crate::ui_state::FrontState;
use common::state::{PersistedRoute, PersistedSettingsRoute};

/// The possible routes for our app.
///
/// While users don't interact with the URL on mobile, encoding the different
/// views with it has the benefit of keeping the view the same when trunk
/// reloads the app after changes to the code, improving the development
/// experience.
#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Splash,
    #[at("/sense")]
    Sense,
    #[at("/analyze")]
    Analyze,
    #[at("/settings")]
    SettingsRoot,
    #[at("/settings/*")]
    SettingsSubpage,
    #[at("/intro")]
    Intro,
    #[at("/test_page")]
    TestPage,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(Clone, Routable, PartialEq)]
pub enum SettingsRoute {
    #[at("/settings")]
    Root,
    #[at("/settings/general")]
    General,
    #[at("/settings/data")]
    Data,
    #[not_found]
    #[at("/settings/404")]
    NotFound,
}

/// Switches what is displayed based on the route URL.
pub fn switch(route: Route) -> Html {
    // save the route in the app state, so it can be retrieved at startup again
    let persist_route = || {
        let dispatch = Dispatch::<FrontState>::new();
        dispatch.reduce_mut(|state| state.route = route.as_persisted_route());
    };
    // Don't persist certain special routes
    match route {
        Route::NotFound => (),
        Route::Splash { .. } => (),
        Route::TestPage { .. } => (),
        _ => persist_route(),
    }
    match route {
        Route::Sense => html! { <pages::Sense /> },
        Route::Splash => html! { <pages::Splash /> },
        Route::Analyze => html! { <pages::Analyze /> },
        Route::SettingsRoot | Route::SettingsSubpage => html! {
            <Switch<SettingsRoute> render={switch_settings} />
        },
        Route::Intro => html! { <pages::Intro /> },
        Route::TestPage => html! { <pages::TestPage /> },
        Route::NotFound => html! {
                <h1 class="text-primary text-3xl mb-6 mt-16">
                    { "Something went wrong" }
                </h1>
        },
    }
}

fn switch_settings(route: SettingsRoute) -> Html {
    let persist_route = || {
        let dispatch = Dispatch::<FrontState>::new();
        dispatch.reduce_mut(|state| {
            state.settings_route = route.as_persisted_route()
        });
    };
    match route {
        SettingsRoute::NotFound => (),
        _ => persist_route(),
    }
    match route {
        SettingsRoute::Root => html! { <pages::Settings /> },
        SettingsRoute::General => html! { <pages::General /> },
        SettingsRoute::Data => html! { <pages::DataSettings /> },
        SettingsRoute::NotFound => html! {
            <Redirect<Route> to={Route::NotFound}/>
        },
    }
}

/// PersistedRoute defines the route that is persisted between app launches.
/// Certain routes are not be persisted in the backend (Splash, NotFound).
impl Route {
    pub fn from_persisted_route(r: &PersistedRoute) -> Self {
        match *r {
            PersistedRoute::Sense => Self::Sense,
            PersistedRoute::Analyze => Self::Analyze,
            PersistedRoute::SettingsRoot => Self::SettingsRoot,
            PersistedRoute::SettingsSubpage => Self::SettingsSubpage,
            PersistedRoute::Intro => Self::Intro,
        }
    }

    fn as_persisted_route(&self) -> PersistedRoute {
        match self {
            Self::Sense => PersistedRoute::Sense,
            Self::Analyze => PersistedRoute::Analyze,
            Self::SettingsRoot => PersistedRoute::SettingsRoot,
            Self::SettingsSubpage => PersistedRoute::SettingsSubpage,
            Self::Intro => PersistedRoute::Intro,
            // should not get here, but if we do, persist Sense
            _ => PersistedRoute::Sense,
        }
    }
}

impl SettingsRoute {
    pub fn from_persisted_route(r: &PersistedSettingsRoute) -> Self {
        match *r {
            PersistedSettingsRoute::Root => Self::Root,
            PersistedSettingsRoute::General => Self::General,
            PersistedSettingsRoute::Data => Self::Data,
        }
    }

    fn as_persisted_route(&self) -> PersistedSettingsRoute {
        match self {
            Self::Root => PersistedSettingsRoute::Root,
            Self::General => PersistedSettingsRoute::General,
            Self::Data => PersistedSettingsRoute::Data,
            _ => PersistedSettingsRoute::Root,
        }
    }
}

/// On startup, navigate to the last page we were on. Some exceptions apply,
/// like when the app's intro has updated such that is should be shown.
pub fn navigate_to_last_page(
    front_state: &Option<common::FrontState>,
    navigator: &yew_router::navigator::Navigator,
) {
    if let Some(s) = front_state {
        if s.last_viewed_intro_version < INTRO_VERSION {
            // new intro to view -> show intro on startup
            navigator
                .push(&Route::from_persisted_route(&PersistedRoute::Intro));
        } else if s.route == PersistedRoute::SettingsSubpage {
            // was on a settings page -> go to persisted settings page
            navigator
                .push(&SettingsRoute::from_persisted_route(&s.settings_route));
        } else {
            // was on a top-level page -> go to it
            navigator.push(&Route::from_persisted_route(&s.route));
        }
    } else {
        // no persisted state -> show intro
        navigator.push(&Route::from_persisted_route(&PersistedRoute::Intro));
    };
}

/// Retrieve the scope from the current url, so that we can use it as the
/// basename for the router
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
