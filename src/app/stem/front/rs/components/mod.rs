//! Module for components that are to be reused throughout the design.

pub mod navbar;

pub use navbar::{Navbar, NavbarWrapper};

pub static SECONDARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-sky-500 bg-neutral-800";
pub static PRIMARY_BUTTON_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 text-neutral-200 bg-sky-500";
pub static DATETIME_INPUT_STYLE: &str =
    "rounded-lg whitespace-nowrap py-1.5 px-3 placeholder:text-neutral-200 bg-neutral-800";

// testing:

// mod capped_input;
// mod list;

// Testing out Yew
// <div class="py-4" />
// <CappedInputComponent min_value={0} max_value={20}/>
// <CappedInputComponent min_value={5} max_value={30}/>
// <ListComponent />
