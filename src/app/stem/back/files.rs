//! Static files served to the frontend.
//!
//! During dev, static files are served at runtime as runfiles, reducing
//! combile times. For release, static files are compiled into the `stem`
//! binary.

pub use inner::*;

#[cfg(not(any(feature = "ios_config", feature = "android_config")))]
compile_error!(
    "Either feature \"ios_config\" or \"android_config\" must be enabled."
);

/// Release mode compiles in each static file
#[cfg(not(feature = "runfiles"))]
mod inner {
    // static files to serve (env vars are set by bazel and poin to file path)
    pub fn get_index() -> &'static str {
        include_str!(env!("INDEX_FILE"))
    }
    pub fn get_wasm() -> &'static [u8] {
        include_bytes!(env!("WASM_FILE"))
    }
    pub fn get_js_launcher() -> &'static str {
        include_str!(env!("JS_FILE"))
    }
    pub fn get_tailwind() -> &'static str {
        include_str!(env!("TAILWIND_FILE"))
    }
    pub fn get_plotly() -> &'static str {
        include_str!(env!("PLOTLY_FILE"))
    }
    pub fn get_maplibre() -> &'static str {
        include_str!(env!("MAPLIBRE_FILE"))
    }
    pub fn get_maplibre_css() -> &'static str {
        include_str!(env!("MAPLIBRE_CSS"))
    }

    #[cfg(feature = "ios_config")]
    pub fn get_when_in_use_auth() -> &'static [u8] {
        include_bytes!(env!("WHEN_IN_USE_AUTH_IOS_PNG"))
    }
    #[cfg(feature = "android_config")]
    pub fn get_when_in_use_auth() -> &'static [u8] {
        include_bytes!(env!("WHEN_IN_USE_AUTH_ANDROID_PNG"))
    }

    #[cfg(feature = "ios_config")]
    pub fn get_always_auth() -> &'static [u8] {
        include_bytes!(env!("ALWAYS_AUTH_PNG"))
    }

    pub fn get_blank_png() -> &'static [u8] {
        include_bytes!(env!("BLANK_PNG"))
    }
}

/// Dev uses bazel runfiles to break the `stem` dependency on the frontend.
#[cfg(feature = "runfiles")]
mod inner {
    use std::fs;

    use once_cell::sync::Lazy;

    pub fn get_index() -> &'static str {
        static INDEX_FILE: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/index.html").unwrap()
        });
        &INDEX_FILE
    }
    pub fn get_wasm() -> &'static [u8] {
        static WASM_FILE: Lazy<Vec<u8>> = Lazy::new(|| {
            fs::read("src/app/stem/front/front_wasm_bg.wasm").unwrap()
        });
        &WASM_FILE
    }
    pub fn get_js_launcher() -> &'static str {
        static JS_FILE: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/front_wasm.js").unwrap()
        });
        &JS_FILE
    }
    pub fn get_tailwind() -> &'static str {
        static TAILWIND_FILE: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/tailwind.css").unwrap()
        });
        &TAILWIND_FILE
    }
    pub fn get_plotly() -> &'static str {
        static PLOTLY_FILE: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/static/plotly.min.js")
                .unwrap()
        });
        &PLOTLY_FILE
    }
    pub fn get_maplibre() -> &'static str {
        static MAPLIBRE_FILE: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/static/maplibre-gl.js")
                .unwrap()
        });
        &MAPLIBRE_FILE
    }
    pub fn get_maplibre_css() -> &'static str {
        static MAPLIBRE_CSS: Lazy<String> = Lazy::new(|| {
            fs::read_to_string("src/app/stem/front/static/maplibre-gl.css")
                .unwrap()
        });
        &MAPLIBRE_CSS
    }

    #[cfg(feature = "ios_config")]
    pub fn get_when_in_use_auth() -> &'static [u8] {
        static WHEN_IN_USE_AUTH: Lazy<Vec<u8>> = Lazy::new(|| {
            fs::read("src/app/stem/front/static/when_in_use_auth_ios.png")
                .unwrap()
        });
        &WHEN_IN_USE_AUTH
    }
    #[cfg(feature = "android_config")]
    pub fn get_when_in_use_auth() -> &'static [u8] {
        static WHEN_IN_USE_AUTH: Lazy<Vec<u8>> = Lazy::new(|| {
            fs::read("src/app/stem/front/static/when_in_use_auth_android.png")
                .unwrap()
        });
        &WHEN_IN_USE_AUTH
    }

    #[cfg(feature = "ios_config")]
    pub fn get_always_auth() -> &'static [u8] {
        static ALWAYS_AUTH: Lazy<Vec<u8>> = Lazy::new(|| {
            fs::read("src/app/stem/front/static/always_auth.png").unwrap()
        });
        &ALWAYS_AUTH
    }

    pub fn get_blank_png() -> &'static [u8] {
        static BLANK_PNG: Lazy<Vec<u8>> = Lazy::new(|| {
            fs::read("src/app/stem/front/static/blank.png").unwrap()
        });
        &BLANK_PNG
    }
}
