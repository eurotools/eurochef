#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

mod app;
pub use app::EurochefApp;

#[cfg(target_arch = "wasm32")]
mod web;

mod entities;
mod entity_renderer;
mod fileinfo;
mod gl_helper;
mod spreadsheet;
mod textures;
