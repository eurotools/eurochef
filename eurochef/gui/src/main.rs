#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide

use color_eyre::eyre::Result;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    use color_eyre::Report;

    color_eyre::install()?;

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    let res = eframe::run_native(
        "Eurochef",
        native_options,
        Box::new(|cc| Box::new(eurochef_gui::EurochefApp::new(std::env::args().nth(1)))),
    );

    match res {
        Ok(()) => Ok(()),
        Err(e) => Err(Report::msg(e.to_string())),
    }
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(eurochef_gui::EurochefApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
