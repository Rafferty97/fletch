use fletch::{FletchOpts, OutputSink};
use wasm_bindgen::prelude::*;

// Better panic messages in the browser console. Optional but worth it.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once(); // add console_error_panic_hook dep if you want this
    wasm_log::init(wasm_log::Config::default());
}

// check(source) -> JsValue (array of diagnostics)
#[wasm_bindgen]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    let diags = fletch::check(source);
    serde_wasm_bindgen::to_value(&diags).map_err(|e| JsValue::from_str(&e.to_string()))
}

// An OutputSink that forwards to a JS callback.
struct JsSink<'a>(&'a js_sys::Function);

impl<'a> OutputSink for JsSink<'a> {
    fn emit(&mut self, text: &str) {
        let _ = self.0.call1(&JsValue::NULL, &JsValue::from_str(text));
    }

    fn emit_err(&mut self, text: &str) {
        let _ = self.0.call1(&JsValue::NULL, &JsValue::from_str(text));
    }
}

// run(source, printCb) -> JsValue (result/diagnostics)
#[wasm_bindgen]
pub fn run(source: &str, print_cb: &js_sys::Function) -> Result<JsValue, JsValue> {
    let mut sink = JsSink(print_cb);
    let opts = FletchOpts { sexpr: false, disassemble: true };
    let result = fletch::run("<anon>", source, opts, &mut sink);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
