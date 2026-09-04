//! App-wide banner shown while a database is still opening.

use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::mounted_db_list::opening_text;
use crate::ui_state::{DerivedState, FrontState};
use common::mounted::EnabledDBs;

/// Banner over the page while any enabled database is still opening (a
/// migration on a large database can take a while; pages show what has
/// opened so far). Rendered once at the app root so every page gets it.
#[function_component]
pub fn DbLoadingBanner() -> Html {
    let dbs_on_disk =
        use_selector(|s: &DerivedState| s.mounted_dbs_on_disk.clone());
    let enabled =
        use_selector(|s: &FrontState| s.mounted_db_settings.enabled_dbs());
    let mut opening =
        enabled.iter().filter_map(|id| match dbs_on_disk.get(id) {
            Some(common::state::DbStatus::Opening { stage, progress }) => {
                Some(opening_text(stage, *progress))
            }
            _ => None,
        });
    let Some(text) = opening.next() else {
        return html! {};
    };
    html! {
        <div class="fixed \
            top-[max(var(--safe-area-top),0.5rem)] \
            mt-2 left-1/2 -translate-x-1/2 z-40 \
            px-3 py-1 rounded-lg bg-black opacity-70 text-sm text-neutral-200 \
            whitespace-nowrap pointer-events-none">
            {text}
        </div>
    }
}
