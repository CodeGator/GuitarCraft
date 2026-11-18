//! Two jobs, done before the app is compiled:
//!
//! 1. Windows only: embed the app icon and version info (VERSIONINFO) into `guitarcraft.exe`, so
//!    it shows in Explorer, the taskbar, the Start menu and Alt-Tab. winresource takes the file
//!    and product version from `CARGO_PKG_VERSION`. A missing resource compiler is a warning, so
//!    a plain `cargo build` still links, unless `GUITARCRAFT_REQUIRE_WINRES=1` turns it into an
//!    error (packaging/windows/package.ps1 sets it for release builds).
//!
//! 2. Every target: the list of open-source components GuitarCraft is built from, for the About
//!    window's Open Source tab. It is read from Cargo.lock and each component's own package
//!    (name, version, description, website, license and license files), so it keeps itself up
//!    to date whenever the components change. Stored compressed in OUT_DIR/licenses.json.z.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=assets/app-icon/guitarcraft.ico");
    println!("cargo:rerun-if-env-changed=GUITARCRAFT_REQUIRE_WINRES");
    licenses();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/app-icon/guitarcraft.ico")
        .set("ProductName", "GuitarCraft")
        .set("FileDescription", "GuitarCraft")
        .set("CompanyName", "CodeGator")
        .set(
            "LegalCopyright",
            "Copyright (c) the GuitarCraft authors. GPL-3.0-or-later.",
        )
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

// ----- the open-source components -----

/// Where Cargo keeps the components' packages.
fn registry_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|h| PathBuf::from(h).join(".cargo"))
        });
    let Some(home) = home else { return vec![] };
    std::fs::read_dir(home.join("registry").join("src"))
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

/// (name, version) of every component from a registry, as listed in Cargo.lock.
fn locked_packages(lock: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    for block in lock.split("[[package]]").skip(1) {
        let field = |k: &str| {
            block.lines().find_map(|l| {
                l.trim()
                    .strip_prefix(&format!("{k} = "))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
        };
        if let (Some(n), Some(v), Some(src)) = (field("name"), field("version"), field("source"))
            && src.starts_with("registry+")
        {
            out.push((n, v));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A few fields of a component's Cargo.toml [package] section.
#[derive(Default)]
struct Manifest {
    description: String,
    license: String,
    link: String,
    authors: Vec<String>,
}

fn read_manifest(dir: &Path) -> Manifest {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    let mut m = Manifest::default();
    let mut in_package = false;
    let mut lines = text.lines();
    let mut homepage = String::new();
    while let Some(line) = lines.next() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, value)) = t.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        // A value: "..." or '...' or a """ / ''' block over several lines.
        let mut read_string = |v: &str| -> String {
            for q in ["\"\"\"", "'''"] {
                if let Some(rest) = v.strip_prefix(q) {
                    if let Some(end) = rest.find(q) {
                        return rest[..end].to_string();
                    }
                    let mut s = rest.to_string();
                    for l in lines.by_ref() {
                        if let Some(end) = l.find(q) {
                            s.push('\n');
                            s.push_str(&l[..end]);
                            break;
                        }
                        s.push('\n');
                        s.push_str(l);
                    }
                    return s;
                }
            }
            if v.starts_with('"') {
                return unescape(v.trim_matches('"'));
            }
            v.trim_matches('\'').to_string()
        };
        match key {
            "description" => {
                m.description = read_string(value)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            "license" => m.license = read_string(value),
            "repository" => m.link = read_string(value),
            "homepage" => homepage = read_string(value),
            "authors" => {
                let mut all = value.to_string();
                while !all.contains(']') {
                    match lines.next() {
                        Some(l) => all.push_str(l),
                        None => break,
                    }
                }
                m.authors = all
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|a| {
                        a.trim()
                            .trim_matches('"')
                            .split('<')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    })
                    .filter(|a| !a.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    if m.link.is_empty() {
        m.link = homepage;
    }
    m
}

/// The license files a component came with (LICENSE, LICENSE-MIT, COPYING ...).
fn license_files(dir: &Path) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let up = name.to_ascii_uppercase();
                    let wanted = ["LICENSE", "LICENCE", "COPYING", "UNLICENSE", "COPYRIGHT"]
                        .iter()
                        .any(|p| up.starts_with(p));
                    if !wanted || e.metadata().map(|m| m.len() > 200_000).unwrap_or(true) {
                        return None;
                    }
                    let text = String::from_utf8_lossy(&std::fs::read(e.path()).ok()?)
                        .replace("\r\n", "\n");
                    Some((name, text))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

const MIT: &str = "MIT License

Copyright (c) {holders}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
";

fn licenses() {
    use std::io::Write;
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    let dirs = registry_dirs();
    let mut texts: Vec<String> = vec![];
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut add_text = |t: String, texts: &mut Vec<String>| -> usize {
        if let Some(&k) = seen.get(&t) {
            return k;
        }
        texts.push(t.clone());
        seen.insert(t, texts.len() - 1);
        texts.len() - 1
    };
    // The standard Apache 2.0 text, taken from any component that ships it.
    let mut apache: Option<String> = None;
    let mut found = vec![];
    for (name, version) in locked_packages(&lock) {
        let dir = dirs
            .iter()
            .map(|d| d.join(format!("{name}-{version}")))
            .find(|d| d.is_dir());
        let (man, files) = match &dir {
            Some(d) => (read_manifest(d), license_files(d)),
            None => (Manifest::default(), vec![]),
        };
        if apache.is_none()
            && let Some((_, t)) = files.iter().find(|(n, t)| {
                n.to_ascii_uppercase().contains("APACHE") && t.contains("Apache License")
            })
        {
            apache = Some(t.clone());
        }
        found.push((name, version, man, files));
    }
    let mut comps = vec![];
    for (name, version, man, files) in found {
        let text = if !files.is_empty() {
            files
                .iter()
                .map(|(n, t)| {
                    if files.len() > 1 {
                        format!("\u{2500}\u{2500} {n} \u{2500}\u{2500}\n\n{}", t.trim_end())
                    } else {
                        t.trim_end().to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n\n")
        } else {
            // No license file in the package: the standard text of the license it names.
            let holders = if man.authors.is_empty() {
                format!("the {name} developers")
            } else {
                man.authors.join(", ")
            };
            let mut parts = vec![];
            if man.license.contains("MIT") {
                parts.push(MIT.replace("{holders}", &holders));
            }
            if man.license.contains("Apache-2.0")
                && let Some(a) = &apache
            {
                parts.push(a.clone());
            }
            if parts.is_empty() {
                let id = man
                    .license
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '(' || c == ')')
                    .to_string();
                parts.push(format!(
                    "This component is licensed under: {}.\n\nIts package came without a license file. The full text of this license is published at https://spdx.org/licenses/{id}.html",
                    if man.license.is_empty() { "(not stated)" } else { &man.license }
                ));
            }
            parts.join("\n\n\n")
        };
        let k = add_text(text, &mut texts);
        comps.push(serde_json::json!({
            "name": name,
            "version": version,
            "description": man.description,
            "license": man.license,
            "link": man.link,
            "text": k,
        }));
    }
    let json = serde_json::json!({ "components": comps, "texts": texts }).to_string();
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("licenses.json.z");
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = enc.write_all(json.as_bytes());
    let data = enc.finish().unwrap_or_default();
    std::fs::write(&out, data).expect("write licenses");
}
