//! Windows only: embed the app icon and version info (VERSIONINFO) into `guitarcraft.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab. winresource takes the file and
//! product version from `CARGO_PKG_VERSION`.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! plain `cargo build` still links, unless `GUITARCRAFT_REQUIRE_WINRES=1` turns it into an error
//! (packaging/windows/package.ps1 sets it for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/app-icon/guitarcraft.ico");
    println!("cargo:rerun-if-env-changed=GUITARCRAFT_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/app-icon/guitarcraft.ico")
        .set("ProductName", "GuitarCraft")
        .set("FileDescription", "GuitarCraft")
        .set("CompanyName", "CodeGator")
        .set("LegalCopyright", "Copyright (c) the GuitarCraft authors. GPL-3.0-or-later.")
        .set("OriginalFilename", "guitarcraft.exe")
        .set("InternalName", "guitarcraft");
    if let Err(e) = res.compile() {
        if std::env::var_os("GUITARCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=guitarcraft.exe built without icon/version resources: {e}");
    }
}
