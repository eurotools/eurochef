use std::process::Command;

fn main() {
    let git_hash = if let Ok(output) = Command::new("git").args(&["rev-parse", "HEAD"]).output() {
        String::from_utf8(output.stdout).unwrap()
    } else {
        "unknown".to_string()
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    let date_time = chrono::Utc::now();
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        date_time.format("%Y-%m-%d %H:%M:%S")
    );

    let output = Command::new("rustc").args(&["--version"]).output().unwrap();
    let rustc_version = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);
}
