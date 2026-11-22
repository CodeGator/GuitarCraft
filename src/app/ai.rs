//! Preferences > AI: the settings for connecting GuitarCraft to an AI provider, Ollama (a
//! model running on this computer or one the person runs) and Claude (Anthropic's service).
//! Nothing is sent anywhere yet except by the "Test connection" buttons, which ask the
//! provider which models it offers. Like the update check, the web request goes through
//! `curl` on a separate thread so the window never freezes.
//!
//! The Claude API key is not kept in `preferences.json`: it goes to the Windows Credential
//! Manager (the macOS Keychain on a Mac), see [`key`]. It is handed to `curl` on its
//! standard input, never on the command line, where other programs could read it.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};

/// Where Ollama listens unless it was set up otherwise.
pub const OLLAMA_URL: &str = "http://localhost:11434";
/// How much text an Ollama model reads at once, in tokens, unless another size is chosen.
/// Ollama's own default is far too small for most of what GuitarCraft could send it.
pub const OLLAMA_CONTEXT: u32 = 16_384;
/// The smallest and largest sizes the Preferences window allows.
pub const OLLAMA_CONTEXT_RANGE: std::ops::RangeInclusive<u32> = 2048..=131_072;
/// The model Claude is asked for unless another is chosen.
pub const CLAUDE_MODEL: &str = "claude-sonnet-5-5";
const CLAUDE_MODELS_URL: &str = "https://api.anthropic.com/v1/models?limit=100";

/// The Claude API key, in the system's credential store.
pub mod key {
    #[cfg(any(windows, target_os = "macos"))]
    mod store {
        fn entry() -> Result<keyring::Entry, String> {
            keyring::Entry::new("GuitarCraft", "claude-api-key").map_err(|e| e.to_string())
        }
        pub fn get() -> Option<String> {
            entry().ok()?.get_password().ok().filter(|k| !k.is_empty())
        }
        pub fn set(key: &str) -> Result<(), String> {
            entry()?.set_password(key).map_err(|e| e.to_string())
        }
        pub fn remove() -> Result<(), String> {
            match entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
    /// Nowhere safe to keep a key here yet, so none is kept.
    #[cfg(not(any(windows, target_os = "macos")))]
    mod store {
        pub fn get() -> Option<String> {
            None
        }
        pub fn set(_: &str) -> Result<(), String> {
            Err("This system has no credential store GuitarCraft can use yet.".into())
        }
        pub fn remove() -> Result<(), String> {
            Ok(())
        }
    }

    pub fn get() -> Option<String> {
        store::get()
    }
    pub fn exists() -> bool {
        get().is_some()
    }
    pub fn set(key: &str) -> Result<(), String> {
        store::set(key.trim())
    }
    pub fn remove() -> Result<(), String> {
        store::remove()
    }
}

/// What one "Test connection" found.
#[derive(Clone, Debug, PartialEq)]
pub enum Test {
    /// Connected; the models the provider offers.
    Ok(Vec<String>),
    Failed(String),
}

/// One provider's test in the Preferences window.
#[derive(Default)]
pub struct Probe {
    rx: Option<Receiver<Test>>,
    pub result: Option<Test>,
}

impl Probe {
    pub fn busy(&self) -> bool {
        self.rx.is_some()
    }

    /// Picks up the answer when it has come.
    pub fn poll(&mut self) {
        if let Some(Ok(t)) = self.rx.as_ref().map(|rx| rx.try_recv()) {
            self.result = Some(t);
            self.rx = None;
        }
    }

    fn start(&mut self, ctx: &egui::Context, job: impl FnOnce() -> Test + Send + 'static) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.result = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(job());
            ctx.request_repaint();
        });
    }

    pub fn test_ollama(&mut self, ctx: &egui::Context, url: &str) {
        let url = ollama_tags_url(url);
        self.start(ctx, move || match curl(&url, "") {
            Ok(body) => read_ollama(&body),
            Err(e) => Test::Failed(e),
        });
    }

    pub fn test_claude(&mut self, ctx: &egui::Context, key: &str) {
        let key = key.trim().to_string();
        self.start(ctx, move || {
            if key.is_empty() {
                return Test::Failed("Enter your API key first.".into());
            }
            if key.chars().any(|c| c.is_control() || c == '"' || c == '\\') {
                return Test::Failed("The API key has characters a key never has. Paste it again.".into());
            }
            let config = format!("header = \"x-api-key: {key}\"\nheader = \"anthropic-version: 2023-06-01\"\n");
            match curl(CLAUDE_MODELS_URL, &config) {
                Ok(body) => read_claude(&body),
                Err(e) => Test::Failed(e),
            }
        });
    }
}

/// The address that lists Ollama's models, from the server address the person typed.
pub fn ollama_tags_url(server: &str) -> String {
    let s = server.trim().trim_end_matches('/');
    let s = if s.is_empty() { OLLAMA_URL } else { s };
    let s = if s.contains("://") { s.to_string() } else { format!("http://{s}") };
    format!("{s}/api/tags")
}

/// Ollama's answer to "which models do you have".
pub fn read_ollama(json: &str) -> Test {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Test::Failed("That address answered, but not like Ollama.".into());
    };
    match v.get("models").and_then(|m| m.as_array()) {
        Some(list) => Test::Ok(list.iter().filter_map(|m| m.get("name").and_then(|n| n.as_str())).map(String::from).collect()),
        None => Test::Failed("That address answered, but not like Ollama.".into()),
    }
}

/// Anthropic's answer to "which models are there", or its complaint.
pub fn read_claude(json: &str) -> Test {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Test::Failed("Anthropic's answer wasn't something GuitarCraft understands.".into());
    };
    if let Some(msg) = v.pointer("/error/message").and_then(|m| m.as_str()) {
        return Test::Failed(msg.to_string());
    }
    match v.get("data").and_then(|m| m.as_array()) {
        Some(list) => Test::Ok(list.iter().filter_map(|m| m.get("id").and_then(|n| n.as_str())).map(String::from).collect()),
        None => Test::Failed("Anthropic's answer wasn't something GuitarCraft understands.".into()),
    }
}

/// Fetches a page with `curl`. `config` is extra curl settings (such as a secret header),
/// given on standard input so they never show up on the command line.
fn curl(url: &str, config: &str) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["--silent", "--max-time", "15", "--user-agent", "GuitarCraft", "--config", "-"])
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window flashing up
    }
    let mut child = cmd.spawn().map_err(|_| "GuitarCraft couldn't start the program it uses to go online (curl).".to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(config.as_bytes());
    }
    let out = child.wait_with_output().map_err(|_| "The connection test didn't finish.".to_string())?;
    if !out.status.success() {
        return Err("Couldn't connect. Check the address and the internet connection.".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ollama_address_is_completed() {
        assert_eq!(ollama_tags_url(""), "http://localhost:11434/api/tags");
        assert_eq!(ollama_tags_url("  192.168.1.5:11434/ "), "http://192.168.1.5:11434/api/tags");
        assert_eq!(ollama_tags_url("https://ai.example.com"), "https://ai.example.com/api/tags");
    }

    #[test]
    fn ollama_models_are_listed() {
        let t = read_ollama(r#"{"models":[{"name":"llama3:8b"},{"name":"qwen3:4b"}]}"#);
        assert_eq!(t, Test::Ok(vec!["llama3:8b".into(), "qwen3:4b".into()]));
        assert!(matches!(read_ollama("<html>"), Test::Failed(_)));
        assert!(matches!(read_ollama("{}"), Test::Failed(_)));
    }

    #[test]
    fn claude_models_and_complaints_are_read() {
        let t = read_claude(r#"{"data":[{"id":"claude-sonnet-5-5"}],"has_more":false}"#);
        assert_eq!(t, Test::Ok(vec!["claude-sonnet-5-5".into()]));
        let bad = read_claude(r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#);
        assert_eq!(bad, Test::Failed("invalid x-api-key".into()));
    }
}
