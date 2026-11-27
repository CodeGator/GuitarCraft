//! Connecting GuitarCraft to an AI provider: Ollama (a model running on this computer or one
//! the person runs), Claude Code (the person's own signed-in `claude` program, see `aicode`)
//! and Claude (Anthropic's service, with an API key). Preferences > AI holds the settings and
//! has a "Test connection" button for each; the AI Assistant panel asks the questions.
//! Everything runs on a separate thread so the window never freezes. Like the update check,
//! the web requests go through `curl`.
//!
//! The Claude API key is not kept in `preferences.json`: it goes to the Windows Credential
//! Manager (the macOS Keychain on a Mac), see [`key`]. It is handed to `curl` on its
//! standard input, never on the command line, where other programs could read it.

use super::aicode;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

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
    /// Ready to use, for a provider that has no list of models: what to tell the person.
    Ready(String),
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

    pub fn test_claude_code(&mut self, ctx: &egui::Context) {
        self.start(ctx, aicode::test);
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

/// The address without the user name and password in it (`user:password@`), or None when it
/// holds none. Neither is ever kept or sent: the address is saved as plain text. No server
/// address needs an `@` for anything else, so all before the last one goes, whatever characters
/// the password has.
pub fn without_password(url: &str) -> Option<String> {
    let at = url.rfind('@')?;
    let host = url[..at].find("://").map_or(0, |i| i + 3);
    Some(format!("{}{}", &url[..host], &url[at + 1..]))
}

/// The server address with its scheme and without a closing slash; Ollama's own when empty.
fn ollama_base(server: &str) -> String {
    let server = without_password(server).unwrap_or_else(|| server.to_string());
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
    ClaudeCode,
    Claude,
}

impl Provider {
    /// Who the answers are from.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Ollama => "Ollama",
            Provider::ClaudeCode | Provider::Claude => "Claude",
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
    /// Ask Ollama to keep the model loaded between questions.
    pub ollama_keep: bool,
    /// The model to ask. Empty, for Claude Code only: its own default.
    pub model: String,
    /// Claude's API key (empty for the other providers).
    pub key: String,
    pub system: String,
    /// The conversation so far, ending with the question: the person's turns and the answers,
    /// one after the other, starting with the person's (Ollama and Claude require that).
    pub turns: Vec<Turn>,
}

/// What the assistant is told about itself.
pub const SYSTEM: &str = "You are the assistant inside GuitarCraft, a desktop editor for guitar tablature and \
music notation in the style of Guitar Pro. Answer briefly and concretely, for a guitarist. When bars of music \
are included they are ASCII tablature: the lines are the strings from the highest to the lowest, numbers are \
frets, x is a dead note, h a hammer-on, p a pull-off, / and \\ slides, b a bend, ~ vibrato and L a tied note. \
Below the tablature there may be facts the program worked out from the notes (the notes used, the scales that \
contain them, the chords they spell). Use them instead of working these out yourself, but they are possibilities: \
when several fit, say so, and the key signature may not match the music. A block headed \"The whole track\" \
describes every bar of the track, not only the bars asked about: a few bars can use very few notes, so use it \
to decide the key and say which scale fits the whole piece as well as the bars. \
A question may also come with parts of the GuitarCraft user guide. For a question about using GuitarCraft, answer \
from that text: use its menu names and shortcuts exactly, and if it doesn't cover the question, say so rather \
than guessing at menus or features GuitarCraft might have. \
You answer questions only: you can't change the score or anything else in the program. If you are asked to make a \
change, say that you can't, and explain how to do it in GuitarCraft, from the user guide text if it covers it. \
If you aren't sure, say so.";

/// The result of a question: the answer, or why there isn't one.
pub type Answer = Result<String, String>;

/// A question on its way. Polled every frame, like [`Probe`]. Dropping it gives up on the
/// answer, and ends the program if Claude Code was asked.
pub struct Pending {
    rx: Receiver<Answer>,
    stop: aicode::Stop,
}

impl Pending {
    pub fn poll(&self) -> Option<Answer> {
        self.rx.try_recv().ok()
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.stop.stop();
    }
}

/// How long a question may take (a model on a modest computer can be slow).
const ASK_SECONDS: u32 = 300;

/// Sends the question on a separate thread. The window repaints when the answer is in.
pub fn ask(ctx: &egui::Context, a: Ask) -> Pending {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    let stop = aicode::Stop::default();
    let theirs = stop.clone();
    std::thread::spawn(move || {
        let _ = tx.send(ask_until(&a, &theirs));
        ctx.request_repaint();
    });
    Pending { rx, stop }
}

/// [`ask`], waiting for the answer on this thread.
#[cfg(test)]
pub fn ask_now(a: &Ask) -> Answer {
    ask_until(a, &aicode::Stop::default())
}

fn ask_until(a: &Ask, stop: &aicode::Stop) -> Answer {
    match a.provider {
        Provider::Ollama => {
            let url = format!("{}/api/chat", ollama_base(&a.ollama_url));
            let body = ollama_body(&a.model, a.ollama_context, &a.system, &a.turns, a.ollama_keep);
            let config = format!("header = \"content-type: application/json\"\ndata = \"{}\"\n", conf_quote(&body));
            read_ollama_chat(&curl_for(&url, &config, ASK_SECONDS)?)
        }
        Provider::ClaudeCode => aicode::ask(&a.system, &a.model, &a.turns, Duration::from_secs(ASK_SECONDS.into()), stop),
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
/// With `keep` the model stays in memory for [`OLLAMA_KEEP_ALIVE`] after the answer (Ollama's own
/// choice is five minutes).
pub fn ollama_body(model: &str, context: u32, system: &str, turns: &[Turn], keep: bool) -> String {
    let mut all = vec![serde_json::json!({ "role": "system", "content": system })];
    all.extend(messages(turns));
    let mut body = serde_json::json!({ "model": model, "stream": false, "options": { "num_ctx": context, "temperature": OLLAMA_TEMPERATURE }, "messages": all });
    if keep {
        body["keep_alive"] = OLLAMA_KEEP_ALIVE.into();
    }
    body.to_string()
}

/// How long Ollama keeps a model in memory after it was last used, when the person has asked for
/// it to be kept loaded.
pub const OLLAMA_KEEP_ALIVE: &str = "30m";

/// How adventurous Ollama's answers are (its own default is 0.8). Lower keeps the assistant to
/// the facts it was given.
pub const OLLAMA_TEMPERATURE: f32 = 0.3;

/// How long after loading (or the last question) the model is loaded again, a little less than
/// [`OLLAMA_KEEP_ALIVE`] so it never has time to unload.
pub const OLLAMA_REWARM: std::time::Duration = std::time::Duration::from_secs(25 * 60);

/// The request that loads a model into memory without asking it anything. The context size must
/// be the one the questions will use: Ollama loads the model again if it differs.
pub fn ollama_warm_body(model: &str, context: u32) -> String {
    serde_json::json!({ "model": model, "stream": false, "keep_alive": OLLAMA_KEEP_ALIVE, "options": { "num_ctx": context } }).to_string()
}

/// Loads the Ollama model in the background so the first question doesn't wait for it. Nothing
/// of the person's is sent: the request names the model and says nothing else. Failures are
/// ignored; a question that comes later will report them.
pub fn warm_ollama(ctx: &egui::Context, url: &str, model: &str, context: u32) {
    let url = format!("{}/api/generate", ollama_base(url));
    let body = ollama_warm_body(model, context);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let config = format!("header = \"content-type: application/json\"\ndata = \"{}\"\n", conf_quote(&body));
        let _ = curl_for(&url, &config, WARM_SECONDS);
        ctx.request_repaint();
    });
}

/// The longest a model is given to load.
const WARM_SECONDS: u32 = 180;

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

/// [`curl`] that gives up after `secs` seconds. The address goes on standard input with the
/// rest of the settings, so it isn't on the command line either.
fn curl_for(url: &str, config: &str, secs: u32) -> Result<String, String> {
    let config = format!("url = \"{}\"\n{config}", conf_quote(url));
    let mut cmd = Command::new("curl");
    cmd.args(["--silent", "--max-time", &secs.to_string(), "--user-agent", "GuitarCraft", "--config", "-"])
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
        let o: Value = serde_json::from_str(&ollama_body("qwen3:4b", 8192, "sys", &turns, false)).unwrap();
        assert_eq!(o["model"], "qwen3:4b");
        assert!(o.get("keep_alive").is_none(), "Ollama's own timeout applies unless asked");
        assert!((o["options"]["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6, "answers keep to what was given");
        let kept: Value = serde_json::from_str(&ollama_body("qwen3:4b", 8192, "sys", &turns, true)).unwrap();
        assert_eq!(kept["keep_alive"], OLLAMA_KEEP_ALIVE);
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
    fn the_warm_up_names_only_the_model_and_the_same_context_as_the_questions() {
        let w: Value = serde_json::from_str(&ollama_warm_body("qwen3:8b", 16_384)).unwrap();
        assert_eq!(w["model"], "qwen3:8b");
        assert_eq!(w["options"]["num_ctx"], 16_384);
        assert_eq!(w["keep_alive"], OLLAMA_KEEP_ALIVE);
        // No prompt or messages: Ollama loads the model and says nothing.
        assert!(w.get("prompt").is_none() && w.get("messages").is_none());
        // Same context as a question, or Ollama would load the model a second time.
        let turns = vec![Turn { from_user: true, text: "x".into() }];
        let q: Value = serde_json::from_str(&ollama_body("qwen3:8b", 16_384, "s", &turns, true)).unwrap();
        assert_eq!(w["options"]["num_ctx"], q["options"]["num_ctx"]);
        assert!(OLLAMA_REWARM < std::time::Duration::from_secs(30 * 60));
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
            ollama_keep: false,
            model,
            key: String::new(),
            system: "Answer in one short sentence.".into(),
            turns: vec![Turn { from_user: true, text: "Which note is the open high E string on a guitar? Quote \"E\" in your answer.".into() }],
        };
        let answer = ask_now(&a).expect("Ollama should answer");
        println!("ANSWER: {answer}");
        assert!(!answer.is_empty());
    }

    /// Asks Claude one thing, through the real request path, with the key saved in Preferences > AI
    /// (read from the system credential store; it is never printed). Spends a few tokens.
    /// Run by hand: `cargo test claude_answers_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn claude_answers_live() {
        let key = key::get().expect("save a Claude API key in Preferences > AI first");
        let model = std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| CLAUDE_MODEL.into());
        let a = Ask {
            provider: Provider::Claude,
            ollama_url: String::new(),
            ollama_context: 4096,
            ollama_keep: false,
            model,
            key,
            system: "Answer in one short sentence.".into(),
            turns: vec![Turn { from_user: true, text: "Which note is the open high E string on a guitar? Quote \"E\" in your answer.".into() }],
        };
        let answer = ask_now(&a).expect("Claude should answer");
        println!("ANSWER: {answer}");
        assert!(!answer.is_empty());
    }

    #[test]
    fn a_password_in_the_ollama_address_is_dropped() {
        assert_eq!(without_password("http://localhost:11434"), None);
        assert_eq!(without_password("http://me:secret@server:11434"), Some("http://server:11434".into()));
        assert_eq!(without_password("me:secret@server"), Some("server".into()));
        // Characters a password may hold, even unescaped.
        assert_eq!(without_password("https://me:p@ss/w://d@server/ai"), Some("https://server/ai".into()));
        assert_eq!(ollama_tags_url("http://me:secret@server:11434/"), "http://server:11434/api/tags");
    }

    /// Through the real `curl`, reading a file so no network is needed.
    #[test]
    fn curl_takes_the_address_from_its_settings() {
        let file = std::env::temp_dir().join(format!("guitarcraft curl {}.txt", std::process::id()));
        std::fs::write(&file, "fetched").unwrap();
        let path = file.to_string_lossy().replace('\\', "/").replace(' ', "%20");
        let url = format!("file:///{}", path.trim_start_matches('/'));
        assert_eq!(curl(&url, ""), Ok("fetched".into()));
        let _ = std::fs::remove_file(&file);
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
