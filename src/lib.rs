#![recursion_limit = "1024"]

#[macro_use]
extern crate serde_derive;

mod app;
mod connect;
mod download_file;
mod import;
mod playlist_types;
mod spotify_types;
mod track_item;
mod track_list;

pub use app::App;
pub use connect::Connect;
pub use download_file::download_file;
pub use import::Import;
pub use track_item::TrackItem;
pub use track_list::TrackList;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is the entry point for the web app
#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<app::App>();
    Ok(())
}
