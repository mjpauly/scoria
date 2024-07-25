//! Macros to unwrap options and results if Some and Ok, and otherwise log
//! an error and return early.
//!
//! Used to gain visibility into cases where an invariant is not upheld as
//! expected.
//!
//! Useful for onclick closures which often do Html element casting and
//! number parsing.

macro_rules! unwrap_option_or_log {
    ($opt:expr) => {
        match $opt {
            Some(v) => v,
            None => {
                tracing::error!("Tried to unwrap None.");
                return;
            }
        }
    };
}

macro_rules! unwrap_result_or_log {
    ($res:expr) => {
        match $res {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Tried to unwrap Err: {}", e);
                return;
            }
        }
    };
}

macro_rules! unwrap_js_result_or_log {
    ($res:expr) => {
        match $res {
            Ok(v) => v,
            Err(error) => {
                let Some(error) = error.dyn_ref::<js_sys::Error>() else {
                    tracing::error!(
                        "Tried to unwrap a JS error, but could not cast \
                        the JsValue to a JS error to to read it."
                    );
                    return;
                };
                let error_string = error.to_string().to_string();
                tracing::error!("Tried to unwrap JS Err: {}", error_string);
                return;
            }
        }
    };
}

pub(crate) use unwrap_js_result_or_log;
pub(crate) use unwrap_option_or_log;
pub(crate) use unwrap_result_or_log;
