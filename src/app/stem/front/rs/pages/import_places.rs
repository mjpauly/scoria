//! Page for importing places from a GeoJSON file.
//!
//! Supports the data as a feature collection of point features.

use yew::prelude::*;
use yewdux::prelude::*;

use crate::components::buttons::{DoneButton, MainSettingsButton};
use crate::components::pin_editor::PinDetailView;
use crate::components::{
    AfterCardParagraph, BouncyScrollContainer, HomeBarSpacer, SettingsCard,
    SettingsCardSimpleButton, TopNav, WarningMessage, H1,
};
use crate::swift_poke;
use crate::ui_state::FrontState;
use crate::websocket::{ToBack, WebsocketService};

#[function_component]
pub fn ImportPlaces() -> Html {
    // use the data in the current pin as the default values
    let pin = use_selector(|s: &FrontState| s.map.current_pin.clone());
    let wss = use_context::<WebsocketService>().unwrap();
    let import_onclick = Callback::from(move |_e: MouseEvent| {
        wss.send_msg(ToBack::ImportPlaces);
        swift_poke::poke();
    });
    html! {
        <>
            <TopNav>
                <MainSettingsButton />
                <DoneButton />
            </TopNav>

            <BouncyScrollContainer class="pb-8">
                <H1> {"Import Places"} </H1>

                <p class="mt-2 mx-2 text-left">
                    {"Import places from a GeoJSON file containing a
                    FeatureCollection of Features with Point geometries. Feature
                    properties are saved as tags."}
                </p>

                <p class="mt-4 mb-4 mx-2 text-left">
                    {"The last viewed or edited place data sets the default
                    values. The icon, lists, and tags are copied over. The name
                    and location will only use the defaults if they are not
                    found. The current defaults are:"}
                </p>

                <div class="text-left max-h-[50lvh] overflow-scroll">
                <div class="w-full bg-neutral-900 rounded-lg h-full \
                    px-4 py-2 flex flex-col gap-2 max-w-prose my-1"
                >
                    <PinDetailView pin={(*pin).clone()} />
                </div>
                </div>

                <AfterCardParagraph>
                    {"To change the defaults,
                    press and hold on the map to create a new place
                    and edit it. edit it, and proceed with importing (saving is
                    optional). If the location is not found, the icon is also
                    set to ❓."}
                </AfterCardParagraph>

                if !pin.lists.is_empty() || !pin.tags.is_empty() {
                    <WarningMessage class="mb-4 mt-4" >
                        {"The default pin values contain lists and/or tags that
                        will be added to each imported place."}
                    </WarningMessage>
                }

                <SettingsCard class="mt-4">
                    <SettingsCardSimpleButton
                        text="Import GeoJSON"
                        onclick={import_onclick}
                    />
                </SettingsCard>

            </BouncyScrollContainer>

            <HomeBarSpacer />
        </>

    }
}
