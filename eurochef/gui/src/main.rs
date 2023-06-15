#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide

use color_eyre::eyre::Result;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    use clap::Parser;
    use color_eyre::Report;

    #[derive(Parser, Debug)]
    struct Args {
        /// Input file
        file: Option<String>,

        /// hashcodes.h
        #[arg(long, short = 't')]
        hashcodes: Option<String>,
    }
    let args = Args::parse();

    // Force enable backtraces
    std::env::set_var("RUST_BACKTRACE", "1");

    eurochef_gui::panic_dialog::setup();

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some([1280., 1024.].into()),
        depth_buffer: 24,
        multisampling: 4,
        shader_version: Some(egui_glow::ShaderVersion::Es300),
        ..Default::default()
    };
    let res = eframe::run_native(
        "Eurochef",
        native_options,
        Box::new(|cc| {
            Box::new(eurochef_gui::EurochefApp::new(
                args.file,
                args.hashcodes,
                cc,
            ))
        }),
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
            Box::new(|cc| Box::new(eurochef_gui::EurochefApp::new(None, None, cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
