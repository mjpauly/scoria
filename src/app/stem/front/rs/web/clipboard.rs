//! Writing to the system clipboard.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use web_sys::{Blob, BlobPropertyBag};

#[wasm_bindgen]
extern "C" {
    type ClipboardItem;

    #[wasm_bindgen(constructor)]
    fn new(data: &Object) -> ClipboardItem;
}

/// Write a String to the clipboard, receiving the result in the callback (true
/// if successful, false if not).
pub fn write_to_clipboard(text: String, callback: impl FnOnce(bool) + 'static) {
    yew::platform::spawn_local(async move {
        let clipboard =
            web_sys::window().unwrap().navigator().clipboard().unwrap();
        let item_data = Object::new();
        let item_value = Blob::new_with_blob_sequence_and_options(
            &Array::of1(&text.as_str().into()),
            BlobPropertyBag::new().type_("text/plain"),
        )
        .unwrap();
        Reflect::set(&item_data, &"text/plain".into(), &item_value).unwrap();
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
