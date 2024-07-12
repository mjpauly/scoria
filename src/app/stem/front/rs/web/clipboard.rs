//! Writing to the system clipboard.
//!
//! Android WebView doesn't support the borwser clipboard api, so we use a
//! workaround.

#[cfg(feature = "android_config")]
pub use android::*;
#[cfg(not(feature = "android_config"))]
pub use browser_api::*;

#[cfg(not(feature = "android_config"))]
mod browser_api {
    use js_sys::{Array, Object, Reflect};
    use wasm_bindgen::prelude::*;
    use web_sys::{Blob, BlobPropertyBag};

    #[cfg(not(feature = "android_config"))]
    #[wasm_bindgen]
    extern "C" {
        type ClipboardItem;

        #[wasm_bindgen(constructor)]
        fn new(data: &Object) -> ClipboardItem;
    }

    /// Write a String to the clipboard, receiving the result in the callback
    /// (true if successful, false if not).
    pub fn write_to_clipboard(
        text: String,
        callback: impl FnOnce(bool) + 'static,
    ) {
        #[cfg(not(feature = "android_config"))]
        yew::platform::spawn_local(async move {
            let clipboard =
                web_sys::window().unwrap().navigator().clipboard().unwrap();
            let item_data = Object::new();
            let item_value = Blob::new_with_blob_sequence_and_options(
                &Array::of1(&text.as_str().into()),
                BlobPropertyBag::new().type_("text/plain"),
            )
            .unwrap();
            Reflect::set(&item_data, &"text/plain".into(), &item_value)
                .unwrap();
            let item = ClipboardItem::new(&item_data);
            let promise = clipboard.write(&Array::of1(&item));
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;
            match result {
                Ok(_) => callback(true),
                Err(e) => {
                    log::debug!("Copy failed {:?}", e);
                    callback(false);
                }
            };
        });
    }
}

#[cfg(feature = "android_config")]
mod android {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Android, js_name = copyToClipboard)]
        fn android_copy_to_clipboard(text: &str);
    }

    pub fn write_to_clipboard(
        text: String,
        callback: impl FnOnce(bool) + 'static,
    ) {
        android_copy_to_clipboard(&text);
        callback(true);
    }
}
