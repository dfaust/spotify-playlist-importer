use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/download_file.js")]
extern "C" {
    pub fn download_file(filename: &str, content: &str);
}
