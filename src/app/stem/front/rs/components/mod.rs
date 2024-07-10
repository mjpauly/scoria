//! Module for components that are to be reused throughout the design.

pub mod bouncy_scroll;
pub mod buttons;
pub mod colorbar;
pub mod confirm;
pub mod inputs;
pub mod location_config;
pub mod location_filter_list;
pub mod map_settings;
pub mod map_styler;
pub mod navbar;
pub mod pin_editor;
pub mod select;
pub mod select_points_control;
pub mod settings_card;
pub mod time_preference;
pub mod time_range_picker;
pub mod typography;
pub mod unit_picker;
pub mod warning;

pub use bouncy_scroll::*;
pub use buttons::*;
pub use colorbar::Colorbar;
pub use inputs::*;
pub use location_config::LocationConfigurator;
pub use location_filter_list::LocationFilterList;
pub use map_styler::MapStyler;
pub use navbar::{BottomNav, HomeBarSpacer, TabBar, TopNav};
pub use select::*;
pub use settings_card::*;
pub use time_range_picker::TimeRangePicker;
pub use typography::*;
pub use warning::*;

// For text contents, py-1.5 px-3 is good.
pub static SECONDARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap text-primary bg-neutral-800";
pub static PRIMARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap text-neutral-200 bg-primary";

pub static DATETIME_INPUT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 placeholder:text-neutral-200 \
        bg-neutral-800";
pub static RANGE_INPUT_STYLE: &str =
    "appearance-none bg-neutral-700 h-1 rounded-lg w-40 max-w-[75vw]";

// The `after:` pseudo-element is the dot on the switch that moves back and
// forth.
// Tailwind adds these by default: after:content-[''] checked:after:content-['']
//
// TODO: set z height below that of the plot so it doesn't overlap before plot's
// resize happens
pub static TOGGLE_SWITCH_STYLE: &str = "\
    h-6 \
    w-10 \
    appearance-none \
    rounded-full \
    bg-neutral-600 \
    after:absolute \
    after:ml-0.5 \
    after:mt-0.5 \
    after:h-5 \
    after:w-5 \
    after:rounded-full \
    after:border-none \
    after:bg-neutral-100 \
    checked:bg-primary \
    checked:after:absolute \
    checked:after:ml-[1.125rem] \
    checked:after:mt-0.5 \
    checked:after:h-5 \
    checked:after:w-5 \
    checked:after:rounded-full \
    checked:after:border-none \
    checked:after:bg-neutral-100 \
    hover:cursor-pointer \
    disabled:cursor-default \
    disabled:opacity-60";
