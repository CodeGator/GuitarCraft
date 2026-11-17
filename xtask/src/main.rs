//! `cargo xtask <version|icons|ico>`: the repository's helper commands.

mod icons;
mod version;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The repository root (the parent of `xtask/`).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or(&[]);
    let r = match args.first().map(String::as_str) {
        Some("version") => version::run(rest),
        Some("icons") => icons::run(rest),
        Some("ico") => icons::run_ico(rest),
        _ => Err(
            "usage: cargo xtask <version [set X.Y.Z[-pre]] | icons | ico <out.ico> <in.png>…>"
                .into(),
        ),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
