//! Module for components that are to be reused throughout the design.

pub mod navbar;

pub use navbar::{Navbar, NavbarWrapper};

// testing:

// mod capped_input;
// mod list;

// Testing out Yew
// <div class="py-4" />
// <CappedInputComponent min_value={0} max_value={20}/>
// <CappedInputComponent min_value={5} max_value={30}/>
// <ListComponent />
