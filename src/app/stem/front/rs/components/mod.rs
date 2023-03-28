//! Module for components that are to be reused throughout the design.

pub mod navbar;
pub mod time_range_picker;

pub use navbar::{Navbar, NavbarWrapper};
pub use time_range_picker::TimeRangePicker;

pub static SECONDARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-sky-500 bg-neutral-800";
#[allow(dead_code)]
pub static PRIMARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-neutral-200 bg-sky-500";
pub static DATETIME_INPUT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 placeholder:text-neutral-200 bg-neutral-800";
