//! `cargo xtask version [set X.Y.Z[-pre]]`: the app's version, the single source of truth.
//!
//! The version lives in `[package] version` in the root `Cargo.toml`. The app shows it through
//! `CARGO_PKG_VERSION`, build.rs puts it in the .exe's version info, and the packaging scripts
//! read it from here. (Adapted from storytold's `xtask version`, which reads
//! `[workspace.package]` instead.)

use std::path::Path;
use std::process::Command;

const SECTION: &str = "[package]";

/// The `version = "…"` value inside `[package]`.
pub fn read(manifest: &str) -> Result<String, String> {
    let (_, line) = find_line(manifest)?;
    let v = line.split_once('=').map(|(_, v)| v.trim().trim_matches('"').to_string()).unwrap_or_default();
    if v.is_empty() {
        return Err(format!("empty {SECTION} version"));
    }
    Ok(v)
}

/// `manifest` with the version replaced; everything else is byte-for-byte unchanged.
pub fn replace(manifest: &str, new: &str) -> Result<String, String> {
    validate(new)?;
    let (idx, _) = find_line(manifest)?;
    let mut out = String::with_capacity(manifest.len() + 8);
    for (i, line) in manifest.split_inclusive('\n').enumerate() {
        if i == idx {
            let eol = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("version = \"{new}\"{eol}"));
        } else {
            out.push_str(line);
        }
    }
    Ok(out)
}

/// Semver without build metadata: `MAJOR.MINOR.PATCH` plus an optional `-pre.release` tag.
/// The installer also needs each number to fit an MSI version (major and minor up to 255,
/// patch up to 65535).
pub fn validate(v: &str) -> Result<(), String> {
    let bad = || Err(format!("`{v}` is not a version like 1.2.3 or 1.2.3-rc.1"));
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let nums: Vec<&str> = core.split('.').collect();
    if nums.len() != 3 || nums.iter().any(|n| !n.bytes().all(|b| b.is_ascii_digit()) || (n.len() > 1 && n.starts_with('0')) || n.parse::<u64>().is_err()) {
        return bad();
    }
    // A numeric prerelease identifier has no leading zero (any length is fine); alphanumeric ones may start with `0`.
    let bad_id = |id: &str| {
        id.is_empty()
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || (id.len() > 1 && id.starts_with('0') && id.bytes().all(|b| b.is_ascii_digit()))
    };
    if let Some(p) = pre
        && (p.is_empty() || p.split('.').any(bad_id))
    {
        return bad();
    }
    let n: Vec<u64> = nums.iter().map(|n| n.parse().unwrap_or(u64::MAX)).collect();
    if n[0] > 255 || n[1] > 255 || n[2] > 65535 {
        return Err(format!("`{v}` is too big for an MSI version (at most 255.255.65535)"));
    }
    Ok(())
}

/// Index and text of the `version` line in `[package]`.
fn find_line(manifest: &str) -> Result<(usize, &str), String> {
    let mut in_section = false;
    for (i, line) in manifest.lines().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t == SECTION;
            continue;
        }
        if in_section
            && let Some(rest) = t.strip_prefix("version")
            && rest.trim_start().starts_with('=')
        {
            return Ok((i, line));
        }
    }
    Err(format!("no `version = \"…\"` in {SECTION} of the root Cargo.toml"))
}

fn cargo_update(root: &Path, offline: bool) -> Result<(), String> {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.args(["update", "--workspace"]).current_dir(root);
    if offline {
        c.arg("--offline");
    }
    let st = c.status().map_err(|e| format!("cargo update: {e}"))?;
    if st.success() { Ok(()) } else { Err(format!("cargo update --workspace{} failed", if offline { " --offline" } else { "" })) }
}

/// Write atomically: a half-written root manifest breaks every build.
fn write_manifest(root: &Path, path: &Path, content: &str) -> Result<(), String> {
    let dir = root.join("target");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let tmp = dir.join("Cargo.toml.xtask-version");
    std::fs::write(&tmp, content).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("replace {}: {e}", path.display()))
}

/// Writes version `new` into the manifest at `path` (current text `text`), then runs `update`.
/// If `update` fails the old manifest is put back. Returns the old version.
fn set(root: &Path, path: &Path, text: &str, new: &str, update: impl FnOnce() -> Result<(), String>) -> Result<String, String> {
    let old = read(text)?;
    let updated = replace(text, new)?;
    write_manifest(root, path, &updated)?;
    if let Err(e) = update() {
        // Put the old manifest back so a failed update does not leave the repo changed.
        return Err(match write_manifest(root, path, text) {
            Ok(()) => format!("{e}; Cargo.toml restored to {old}"),
            Err(r) => format!("{e}; could not restore Cargo.toml: {r}"),
        });
    }
    Ok(old)
}

/// `cargo xtask version [set X.Y.Z[-pre]]`; `args` are the words after `version`.
pub fn run(args: &[String]) -> Result<(), String> {
    let root = crate::root();
    let path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => {
            println!("{}", read(&text)?);
            Ok(())
        }
        ["set", new] => {
            // Refresh the workspace's own entries in Cargo.lock (no dependency upgrades).
            let old = set(&root, &path, &text, new, || cargo_update(&root, true).or_else(|_| cargo_update(&root, false)))?;
            println!("version: {old} -> {new}");
            Ok(())
        }
        _ => Err("usage: cargo xtask version [set X.Y.Z[-pre]]".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[workspace]\nmembers = [\".\", \"xtask\"]\n\n[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nfoo = { version = \"1\" }\n";

    #[test]
    fn reads_the_package_version() {
        assert_eq!(read(MANIFEST).unwrap(), "0.1.0");
        assert!(read("[workspace.package]\nversion = \"1.0.0\"\n").is_err());
    }

    #[test]
    fn replaces_only_the_package_version() {
        let out = replace(MANIFEST, "1.2.3-rc.1").unwrap();
        assert_eq!(read(&out).unwrap(), "1.2.3-rc.1");
        assert_eq!(out, MANIFEST.replace("version = \"0.1.0\"", "version = \"1.2.3-rc.1\""));
        assert!(out.contains("foo = { version = \"1\" }"));
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let crlf = MANIFEST.replace('\n', "\r\n");
        let out = replace(&crlf, "2.0.0").unwrap();
        assert_eq!(out, crlf.replace("\"0.1.0\"", "\"2.0.0\""));
    }

    #[test]
    fn validates_versions() {
        for ok in ["0.1.0", "1.20.300", "1.0.0-rc.1", "1.0.0-alpha", "1.0.0-x-y.2", "255.255.65535"] {
            assert!(validate(ok).is_ok(), "{ok}");
        }
        for bad in ["1.0", "1.0.0.0", "v1.0.0", "01.0.0", "1.0.0-", "1.0.0-a..b", "1.0.0+meta", "1.a.0", "", "1.0.0-rc.01", "256.0.0", "1.0.65536"] {
            assert!(validate(bad).is_err(), "{bad}");
        }
        assert!(replace(MANIFEST, "nope").is_err());
    }

    #[test]
    fn a_failed_update_restores_the_manifest() {
        let root = std::env::temp_dir().join(format!("xtask-version-restore-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("Cargo.toml");
        std::fs::write(&path, MANIFEST).unwrap();
        let err = set(&root, &path, MANIFEST, "2.0.0", || Err("cargo update failed".into())).unwrap_err();
        assert!(err.contains("restored to 0.1.0"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), MANIFEST);
        assert_eq!(set(&root, &path, MANIFEST, "2.0.0", || Ok(())).unwrap(), "0.1.0");
        assert_eq!(read(&std::fs::read_to_string(&path).unwrap()).unwrap(), "2.0.0");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_real_manifest_has_a_valid_version() {
        let text = std::fs::read_to_string(crate::root().join("Cargo.toml")).unwrap();
        validate(&read(&text).unwrap()).unwrap();
    }
}
