#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

mod app;
pub use app::EurochefApp;

#[cfg(not(target_arch = "wasm32"))]
pub mod panic_dialog;

mod entities;
mod entity_renderer;
mod fileinfo;
mod gl_helper;
mod render;
mod spreadsheet;
mod textures;
