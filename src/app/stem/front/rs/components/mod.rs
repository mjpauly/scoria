//! Module for components that are to be reused throughout the design.

pub mod map_styler;
pub mod navbar;
pub mod time_range_picker;

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
