#[macro_use]
extern crate tracing;

mod app;
pub use app::EurochefApp;

#[cfg(not(target_arch = "wasm32"))]
pub mod panic_dialog;

mod entities;
mod entity_frame;
mod fileinfo;
mod map_frame;
mod maps;
mod render;
mod scripts;
mod spreadsheet;
mod textures;

pub fn strip_ansi_codes(input: &str) -> String {
    let mut output = String::new();
    let mut in_escape = false;

    for c in input.chars() {
        if in_escape {
            if c.is_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1B' {
            in_escape = true;
        } else {
            output.push(c);
        }
    }

    output
}
