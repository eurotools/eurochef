use std::{
    backtrace::{Backtrace, BacktraceStatus},
    io::Write,
    panic::PanicInfo,
};

use crate::strip_ansi_codes;

pub fn setup() {
    std::panic::set_hook(Box::new(|info| {
        // First call color-eyre's fancy CLI backtrace
        let (panic_hook, _) = color_eyre::config::HookBuilder::new().into_hooks();
        eprintln!("{}", panic_hook.panic_report(info));

        // Write a panic file
        match write_panic_to_file(info, Backtrace::capture()) {
            Ok(()) => {}
            Err(e) => eprintln!("Failed to create panic log: {e}"),
        }

        // Dont show dialog on debug builds
        if cfg!(debug_assertions) {
            return;
        }

        // Finally, show a dialog
        let panic_message_stripped = &strip_ansi_codes(&format!("{info}"));
        if let Err(e) = native_dialog::MessageDialog::new()
            .set_type(native_dialog::MessageType::Error)
            .set_title("Panic!")
            .set_text(&format!(
                "{}\n\nThe panic has been written to panic.log",
                panic_message_stripped
            ))
            .show_alert()
        {
            eprintln!("Failed to show error dialog: {e}")
        }

        // Make sure the application exits
        std::process::exit(-1);
    }))
}

fn write_panic_to_file(info: &PanicInfo<'_>, bt: Backtrace) -> std::io::Result<()> {
    let mut f = std::fs::File::create("panic.log")?;

    writeln!(f, "{}", info)?;
    if bt.status() == BacktraceStatus::Captured {
        writeln!(f)?;
        writeln!(f, "Backtrace:")?;
        writeln!(f, "{}", bt)?;
    }

    Ok(())
}
