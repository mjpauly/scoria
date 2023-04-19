//! Module for components that are to be reused throughout the design.

pub mod location_config;
pub mod map_styler;
pub mod navbar;
pub mod time_range_picker;

pub use location_config::LocationConfigurator;
pub use map_styler::MapStyler;
pub use navbar::{Navbar, NavbarWrapper};
pub use time_range_picker::TimeRangePicker;

pub static SECONDARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-primary bg-neutral-800";
pub static PRIMARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-neutral-200 bg-primary";
pub static DATETIME_INPUT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 placeholder:text-neutral-200 \
        bg-neutral-800";
pub static SELECT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-neutral-200 bg-neutral-800 \
        appearance-none";
pub static RANGE_INPUT_STYLE: &str =
    "appearance-none bg-neutral-800 h-1 rounded-lg w-40";

// The `after:` pseudo-element is the dot on the switch that moves back and
// forth.
// Tailwind adds these by default: after:content-[''] checked:after:content-['']
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
    after:transition-[background-color_0.2s,transform_0.2s] \
    checked:bg-primary \
    checked:after:absolute \
    checked:after:ml-[1.125rem] \
    checked:after:mt-0.5 \
    checked:after:h-5 \
    checked:after:w-5 \
    checked:after:rounded-full \
    checked:after:border-none \
    checked:after:bg-neutral-100 \
    checked:after:transition-[background-color_0.2s,transform_0.2s] \
    hover:cursor-pointer \
    disabled:cursor-default \
    disabled:opacity-60";
