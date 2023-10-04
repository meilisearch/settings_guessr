use serde_json::{Deserializer, Value};
use settings_guessr::{Document, FieldAccumulator};
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub fn guess(buffer: &[u8]) -> JsValue {
    let mut accumulator = FieldAccumulator::new();

    let deserializer = Deserializer::from_slice(buffer);
    let mut deserializer = deserializer.into_iter();
    let value: Value = deserializer.next().expect("found empty stream").unwrap();

    if let Some(values) = value.as_array() {
        for value in values {
            let document: &Document = value.as_object().expect("invalid document");
            accumulator.push(document);
        }
    } else if let Some(document) = value.as_object() {
        accumulator.push(document);

        for document in deserializer {
            accumulator.push(document.unwrap().as_object().unwrap());
        }
    }

    let settings = accumulator.finish();

    serde_wasm_bindgen::to_value(&settings).unwrap()
}
