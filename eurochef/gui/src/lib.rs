#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

mod app;
pub use app::EurochefApp;

#[cfg(target_arch = "wasm32")]
mod web;

// TODO: Move
mod fileinfo;
mod spreadsheet;
mod textures;
