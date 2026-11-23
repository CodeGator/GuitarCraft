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
    format!("{}/api/tags", ollama_base(server))
}

/// The server address with its scheme and without a closing slash; Ollama's own when empty.
fn ollama_base(server: &str) -> String {
    let s = server.trim().trim_end_matches('/');
    let s = if s.is_empty() { OLLAMA_URL } else { s };
    if s.contains("://") { s.to_string() } else { format!("http://{s}") }
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

// ----- asking a question -----

/// Where the answer comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Ollama,
    Claude,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Ollama => "Ollama",
            Provider::Claude => "Claude",
        }
    }
}

/// One side of the conversation.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    pub from_user: bool,
    pub text: String,
}

/// Everything one question needs. Built by the panel when the person presses Send.
pub struct Ask {
    pub provider: Provider,
    /// Ollama: the server address (empty: Ollama's own) and the context size.
    pub ollama_url: String,
    pub ollama_context: u32,
    /// The model to ask, for either provider.
    pub model: String,
    /// Claude's API key.
    pub key: String,
    pub system: String,
    /// The conversation so far, ending with the question: the person's turns and the answers,
    /// one after the other, starting with the person's (both providers require that).
    pub turns: Vec<Turn>,
}

/// What the assistant is told about itself.
pub const SYSTEM: &str = "You are the assistant inside GuitarCraft, a desktop editor for guitar tablature and \
music notation in the style of Guitar Pro. Answer briefly and concretely, for a guitarist. When bars of music \
are included they are ASCII tablature: the lines are the strings from the highest to the lowest, numbers are \
frets, x is a dead note, h a hammer-on, p a pull-off, / and \\ slides, b a bend, ~ vibrato and L a tied note. \
You can't change the score yourself; say what the person could do. If you aren't sure, say so.";

/// The result of a question: the answer, or why there isn't one.
pub type Answer = Result<String, String>;

/// A question on its way. Polled every frame, like [`Probe`].
pub struct Pending {
    rx: Receiver<Answer>,
}

impl Pending {
    pub fn poll(&self) -> Option<Answer> {
        self.rx.try_recv().ok()
    }
}

/// How long a question may take (a model on a modest computer can be slow).
const ASK_SECONDS: u32 = 300;

/// Sends the question on a separate thread. The window repaints when the answer is in.
pub fn ask(ctx: &egui::Context, a: Ask) -> Pending {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(ask_now(&a));
        ctx.request_repaint();
    });
    Pending { rx }
}

fn ask_now(a: &Ask) -> Answer {
    match a.provider {
        Provider::Ollama => {
            let url = format!("{}/api/chat", ollama_base(&a.ollama_url));
            let body = ollama_body(&a.model, a.ollama_context, &a.system, &a.turns);
            let config = format!("header = \"content-type: application/json\"\ndata = \"{}\"\n", conf_quote(&body));
            read_ollama_chat(&curl_for(&url, &config, ASK_SECONDS)?)
        }
        Provider::Claude => {
            let key = a.key.trim();
            if key.is_empty() {
                return Err("There is no Claude API key. Add one in Preferences > AI.".into());
            }
            if key.chars().any(|c| c.is_control() || c == '"' || c == '\\') {
                return Err("The API key has characters a key never has. Paste it again in Preferences > AI.".into());
            }
            let body = claude_body(&a.model, &a.system, &a.turns);
            let config = format!(
                "header = \"x-api-key: {key}\"\nheader = \"anthropic-version: 2023-06-01\"\nheader = \"content-type: application/json\"\ndata = \"{}\"\n",
                conf_quote(&body)
            );
            read_claude_message(&curl_for(CLAUDE_MESSAGES_URL, &config, ASK_SECONDS)?)
        }
    }
}

const CLAUDE_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
/// The most Claude may answer with, in tokens.
const CLAUDE_MAX_TOKENS: u32 = 2048;

fn messages(turns: &[Turn]) -> Vec<Value> {
    turns
        .iter()
        .map(|t| serde_json::json!({ "role": if t.from_user { "user" } else { "assistant" }, "content": t.text }))
        .collect()
}

/// The request for Ollama's `/api/chat`: one answer, not streamed.
pub fn ollama_body(model: &str, context: u32, system: &str, turns: &[Turn]) -> String {
    let mut all = vec![serde_json::json!({ "role": "system", "content": system })];
    all.extend(messages(turns));
    serde_json::json!({ "model": model, "stream": false, "options": { "num_ctx": context }, "messages": all }).to_string()
}

/// The request for Claude's messages call.
pub fn claude_body(model: &str, system: &str, turns: &[Turn]) -> String {
    serde_json::json!({ "model": model, "max_tokens": CLAUDE_MAX_TOKENS, "system": system, "messages": messages(turns) }).to_string()
}

/// Ollama's answer to a chat, or its complaint.
pub fn read_ollama_chat(json: &str) -> Answer {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Err("That address answered, but not like Ollama.".into());
    };
    if let Some(e) = v.get("error").and_then(|e| e.as_str()) {
        return Err(e.to_string());
    }
    match v.pointer("/message/content").and_then(|c| c.as_str()) {
        Some(t) if !t.trim().is_empty() => Ok(t.trim().to_string()),
        Some(_) => Err("The model sent an empty answer.".into()),
        None => Err("That address answered, but not like Ollama.".into()),
    }
}

/// Claude's answer to a message, or its complaint.
pub fn read_claude_message(json: &str) -> Answer {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Err("Anthropic's answer wasn't something GuitarCraft understands.".into());
    };
    if let Some(msg) = v.pointer("/error/message").and_then(|m| m.as_str()) {
        return Err(msg.to_string());
    }
    let Some(parts) = v.get("content").and_then(|c| c.as_array()) else {
        return Err("Anthropic's answer wasn't something GuitarCraft understands.".into());
    };
    let text: String = parts.iter().filter_map(|p| p.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join("");
    if text.trim().is_empty() { Err("Claude sent an empty answer.".into()) } else { Ok(text.trim().to_string()) }
}

/// A value for a curl config file: the characters curl treats specially escaped (the caller
/// puts it in quotes).
fn conf_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Fetches a page with `curl`. `config` is extra curl settings (such as a secret header),
/// given on standard input so they never show up on the command line.
fn curl(url: &str, config: &str) -> Result<String, String> {
    curl_for(url, config, 15)
}

/// [`curl`] that gives up after `secs` seconds.
fn curl_for(url: &str, config: &str, secs: u32) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["--silent", "--max-time", &secs.to_string(), "--user-agent", "GuitarCraft", "--config", "-"])
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
    fn requests_are_built_for_each_provider() {
        let turns = vec![Turn { from_user: true, text: "What key?".into() }];
        let o: Value = serde_json::from_str(&ollama_body("qwen3:4b", 8192, "sys", &turns)).unwrap();
        assert_eq!(o["model"], "qwen3:4b");
        assert_eq!(o["stream"], false);
        assert_eq!(o["options"]["num_ctx"], 8192);
        assert_eq!(o["messages"][0]["role"], "system");
        assert_eq!(o["messages"][1]["content"], "What key?");
        let c: Value = serde_json::from_str(&claude_body("claude-sonnet-5-5", "sys", &turns)).unwrap();
        assert_eq!(c["system"], "sys");
        assert_eq!(c["messages"][0]["role"], "user");
        assert!(c["max_tokens"].as_u64().unwrap() > 0);
    }

    #[test]
    fn answers_are_read_and_complaints_passed_on() {
        assert_eq!(read_ollama_chat(r#"{"message":{"role":"assistant","content":" E minor. "}}"#), Ok("E minor.".into()));
        assert_eq!(read_ollama_chat(r#"{"error":"model 'x' not found"}"#), Err("model 'x' not found".into()));
        assert!(read_ollama_chat("<html>").is_err());
        assert_eq!(read_claude_message(r#"{"content":[{"type":"text","text":"A"},{"type":"text","text":"B"}]}"#), Ok("AB".into()));
        assert_eq!(read_claude_message(r#"{"type":"error","error":{"message":"invalid x-api-key"}}"#), Err("invalid x-api-key".into()));
        assert!(read_claude_message(r#"{"content":[]}"#).is_err());
    }

    #[test]
    fn config_values_are_escaped_for_curl() {
        assert_eq!(conf_quote("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(conf_quote("one\ntwo"), "one\\ntwo");
        // The body of a request survives: JSON never holds a raw newline.
        let turns = vec![Turn { from_user: true, text: "line1\nline2 \"quoted\"".into() }];
        assert!(!claude_body("m", "s", &turns).contains('\n'));
    }

    /// Asks the Ollama on this computer one thing, through the real request path (curl and all).
    /// Run by hand: `cargo test ollama_answers_live -- --ignored`.
    #[test]
    #[ignore]
    fn ollama_answers_live() {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen-craft:latest".into());
        let a = Ask {
            provider: Provider::Ollama,
            ollama_url: String::new(),
            ollama_context: 4096,
            model,
            key: String::new(),
            system: "Answer in one short sentence.".into(),
            turns: vec![Turn { from_user: true, text: "Which note is the open high E string on a guitar? Quote \"E\" in your answer.".into() }],
        };
        let answer = ask_now(&a).expect("Ollama should answer");
        println!("ANSWER: {answer}");
        assert!(!answer.is_empty());
    }

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
