//! Asking Claude through Claude Code, the `claude` program the person installed and signed in to
//! themselves, so GuitarCraft needs no API key and never sees their sign-in. Each question runs
//! the program once, without its tools, plugins, hooks or saved sessions: the question goes in on
//! standard input and one JSON reply comes back.
//!
//! Claude Code adds details of its own to what it sends: the account's email address, the
//! operating system, the date and the folder it runs in. It is run in the top folder of the
//! system drive so that folder says nothing about the person (see [`neutral_folder`]).

use super::ai::{Answer, Test, Turn};
use serde_json::Value;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// The model asked for unless another is chosen: a short name Claude Code understands, which
/// always means the newest model of that family. Empty means Claude Code's own default.
pub const MODEL: &str = "sonnet";
/// The short model names offered in Preferences > AI.
pub const MODELS: [&str; 3] = ["sonnet", "opus", "haiku"];

const NOT_FOUND: &str = "Claude Code wasn't found. Install it, open a terminal and run `claude auth login`, then try again.";
const SIGN_IN: &str = "Claude Code isn't signed in. Open a terminal, run `claude auth login`, then try again.";

/// Stops a question that is on its way, ending the program so it uses no more of the person's
/// allowance. Clones stop the same question.
#[derive(Clone, Default)]
pub struct Stop(Arc<Mutex<Running>>);

#[derive(Default)]
struct Running {
    stopped: bool,
    child: Option<Child>,
}

impl Stop {
    pub fn stop(&self) {
        let mut r = self.lock();
        r.stopped = true;
        end(&mut r);
    }

    fn lock(&self) -> MutexGuard<'_, Running> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Ends the program, if one is running.
fn end(r: &mut Running) {
    if let Some(mut child) = r.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// The `claude` program on this computer.
pub fn find() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
    // Where its installers put it, in case the search path doesn't say (a Mac app started from
    // the Dock gets a very short one).
    if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        dirs.push(Path::new(&home).join(".local").join("bin"));
    }
    if cfg!(windows) {
        if let Some(data) = std::env::var_os("APPDATA") {
            dirs.push(Path::new(&data).join("npm"));
        }
    } else {
        dirs.extend(["/usr/local/bin", "/opt/homebrew/bin"].map(PathBuf::from));
    }
    find_in(&dirs)
}

/// The first `claude` program in `dirs`. On Windows an npm install puts only a `claude.cmd`
/// script on the search path; the real program it starts is used instead, because text with line
/// breaks can't be passed safely through a script. The Claude desktop app also has a
/// `claude.exe`, which isn't Claude Code and is passed over.
fn find_in(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter().filter(|d| !d.as_os_str().is_empty()).find_map(|dir| {
        if !cfg!(windows) {
            return Some(dir.join("claude")).filter(|p| p.is_file());
        }
        let exe = dir.join("claude.exe");
        if exe.is_file() && !is_desktop_app(&exe) {
            return Some(exe);
        }
        if !dir.join("claude.cmd").is_file() {
            return None;
        }
        Some(dir.join("node_modules").join("@anthropic-ai").join("claude-code").join("bin").join("claude.exe")).filter(|p| p.is_file())
    })
}

/// The Claude desktop app's own program: its installer's folder, or the Microsoft Store's.
fn is_desktop_app(exe: &Path) -> bool {
    exe.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name.eq_ignore_ascii_case("AnthropicClaude") || name.eq_ignore_ascii_case("WindowsApps")
    })
}

/// The folder Claude Code runs in. It sends the folder's path with each question, so this is one
/// that names nobody.
fn neutral_folder() -> PathBuf {
    if cfg!(windows) {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        PathBuf::from(format!("{drive}\\"))
    } else {
        PathBuf::from("/")
    }
}

/// Environment variables that a running Claude Code session sets for the programs it starts. If
/// GuitarCraft was started from one, they would make `claude` act as part of that session, so
/// they are removed. The person's own settings (`ANTHROPIC_*`, `CLAUDE_CONFIG_DIR`, the choice of
/// provider in `CLAUDE_CODE_USE_*`) are kept.
fn session_vars(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    let names: Vec<OsString> = names.collect();
    if !names.iter().any(|n| n == "CLAUDECODE") {
        return Vec::new();
    }
    names
        .into_iter()
        .filter(|n| {
            let n = n.to_string_lossy();
            matches!(n.as_ref(), "CLAUDECODE" | "CLAUDE_PID" | "CLAUDE_EFFORT")
                || n.starts_with("CLAUDE_AGENT_SDK_")
                || (n.starts_with("CLAUDE_CODE_") && !n.starts_with("CLAUDE_CODE_USE_"))
        })
        .collect()
}

fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(neutral_folder());
    for name in session_vars(std::env::vars_os().map(|(n, _)| n)) {
        cmd.env_remove(name);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window flashing up
    }
    cmd
}

/// The options for one question: print the reply as JSON and exit, as a plain assistant with
/// `system` as its only instructions, no tools, none of the person's Claude Code customizations,
/// and nothing saved.
fn question_args(system: &str, model: &str) -> Vec<String> {
    let mut args: Vec<String> = ["-p", "--output-format", "json", "--system-prompt", system, "--tools", "", "--strict-mcp-config", "--safe-mode", "--no-session-persistence"]
        .map(String::from)
        .to_vec();
    if !model.trim().is_empty() {
        args.extend(["--model".into(), model.trim().into()]);
    }
    args
}

/// The conversation as the one piece of text Claude Code takes. Nothing is saved between
/// questions, so the earlier turns go along each time.
fn prompt(turns: &[Turn]) -> String {
    let Some((last, earlier)) = turns.split_last() else {
        return String::new();
    };
    if earlier.is_empty() {
        return last.text.clone();
    }
    let mut out = String::from("The conversation so far, oldest first:\n\n");
    for t in earlier {
        let tag = if t.from_user { "person" } else { "assistant" };
        out.push_str(&format!("<{tag}>\n{}\n</{tag}>\n\n", t.text));
    }
    out.push_str("Now answer this, the person's newest message:\n\n");
    out.push_str(&last.text);
    out
}

/// Asks one question. Takes as long as the answer does, up to `limit`.
pub fn ask(system: &str, model: &str, turns: &[Turn], limit: Duration, stop: &Stop) -> Answer {
    let program = find().ok_or(NOT_FOUND)?;
    let mut cmd = command(&program);
    cmd.args(question_args(system, model));
    let out = run(cmd, prompt(turns), limit, stop)?;
    read_reply(&out.stdout, &out.stderr)
}

/// "Test connection": whether Claude Code is there and signed in. Asks it nothing.
pub fn test() -> Test {
    let Some(program) = find() else {
        return Test::Failed(NOT_FOUND.into());
    };
    let mut cmd = command(&program);
    cmd.args(["auth", "status", "--json"]);
    match run(cmd, String::new(), Duration::from_secs(15), &Stop::default()) {
        Ok(out) => read_status(&out.stdout),
        Err(e) => Test::Failed(e),
    }
}

/// What the program printed before it ended.
struct Output {
    stdout: String,
    stderr: String,
}

/// Runs the program to its end, giving it `input`, unless `stop` is used or `limit` passes first.
fn run(mut cmd: Command, input: String, limit: Duration, stop: &Stop) -> Result<Output, String> {
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let (stdout, stderr) = {
        // Held while starting, so a Stop at this moment can't miss the program.
        let mut r = stop.lock();
        if r.stopped {
            return Err("Stopped.".into());
        }
        let mut child = cmd.spawn().map_err(|e| format!("GuitarCraft couldn't start Claude Code: {e}"))?;
        // Written on its own thread: a program that doesn't read must still be stoppable.
        if let Some(mut stdin) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = stdin.write_all(input.as_bytes());
            });
        }
        let pipes = (read_all(child.stdout.take()), read_all(child.stderr.take()));
        r.child = Some(child);
        pipes
    };
    let started = Instant::now();
    loop {
        {
            let mut r = stop.lock();
            let Some(child) = r.child.as_mut() else {
                return Err("Stopped.".into());
            };
            match child.try_wait() {
                Ok(Some(_)) => {
                    r.child = None;
                    break;
                }
                Ok(None) if started.elapsed() < limit => {}
                Ok(None) => {
                    end(&mut r);
                    return Err(format!("Claude Code didn't answer within {} seconds.", limit.as_secs()));
                }
                Err(e) => {
                    end(&mut r);
                    return Err(format!("Claude Code didn't finish: {e}"));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // The pipes close with the program; the wait is limited in case something it started holds on.
    let text = |rx: Receiver<String>| rx.recv_timeout(Duration::from_secs(5)).unwrap_or_default();
    Ok(Output { stdout: text(stdout), stderr: text(stderr) })
}

/// Everything a pipe delivers, read on its own thread so a full pipe never stalls the program.
fn read_all(pipe: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = tx.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    rx
}

/// The JSON in what the program printed, passing over any warnings around it.
fn json_in(text: &str) -> Option<Value> {
    let read = |s: &str| serde_json::from_str::<Value>(s.trim()).ok();
    read(text)
        // The reply on a line of its own.
        .or_else(|| text.lines().rev().find_map(|l| read(l).filter(Value::is_object)))
        // An object spread over several lines, as the sign-in status is.
        .or_else(|| text.get(text.find('{')?..=text.rfind('}')?).and_then(read))
}

/// The answer to a question, or why there isn't one.
fn read_reply(stdout: &str, stderr: &str) -> Answer {
    // A list of events instead of one result is what a `verbose` setting gives.
    let reply = json_in(stdout).and_then(|v| match v {
        Value::Array(all) => all.into_iter().rev().find(|e| e["type"] == "result"),
        v => Some(v),
    });
    let Some(v) = reply else {
        if stderr.contains("unknown option") {
            return Err("This Claude Code is too old for GuitarCraft. Open a terminal, run `claude update`, then try again.".into());
        }
        return Err(match stderr.lines().map(str::trim).find(|l| !l.is_empty()) {
            Some(why) => format!("Claude Code stopped: {why}"),
            None => "Claude Code stopped without answering.".into(),
        });
    };
    let text = v["result"].as_str().unwrap_or("").trim();
    // A failure says so in `is_error`; its `subtype` can still read "success".
    if v["is_error"].as_bool() == Some(true) {
        if text.contains("Not logged in") || text.contains("/login") {
            return Err(SIGN_IN.into());
        }
        let why = if text.is_empty() { v["subtype"].as_str().unwrap_or("no reason given") } else { text };
        return Err(format!("Claude Code stopped: {why}"));
    }
    match v["result"].as_str() {
        Some(_) if !text.is_empty() => Ok(text.to_string()),
        Some(_) => Err("Claude sent an empty answer.".into()),
        None => Err("Claude Code's answer wasn't something GuitarCraft understands.".into()),
    }
}

/// What `claude auth status --json` says, as the person needs to know it: whether they are signed
/// in, and what pays for the questions. Their email address is never shown.
fn read_status(stdout: &str) -> Test {
    let Some(v) = json_in(stdout).filter(Value::is_object) else {
        return Test::Failed("Claude Code didn't say whether it is signed in. Try `claude update` in a terminal.".into());
    };
    match v["loggedIn"].as_bool() {
        Some(true) => {}
        Some(false) => return Test::Failed(SIGN_IN.into()),
        None => return Test::Failed("Claude Code didn't say whether it is signed in. Try `claude update` in a terminal.".into()),
    }
    Test::Ready(if let Some(key) = v["apiKeySource"].as_str() {
        format!("Signed in with an API key ({key}), which is billed.")
    } else if let Some(other) = v["apiProvider"].as_str().filter(|p| *p != "firstParty") {
        format!("Signed in. Questions go through {other}.")
    } else if let Some(plan) = v["subscriptionType"].as_str() {
        format!("Signed in with your Claude {plan} plan.")
    } else {
        "Signed in.".into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(from_user: bool, text: &str) -> Turn {
        Turn { from_user, text: text.into() }
    }

    #[test]
    fn a_question_runs_without_tools_customizations_or_saved_sessions() {
        let a = question_args("Be brief.\nSecond line.", "sonnet");
        assert_eq!(a[..3], ["-p", "--output-format", "json"]);
        let after = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].as_str();
        assert_eq!(after("--system-prompt"), "Be brief.\nSecond line.");
        assert_eq!(after("--tools"), "", "no tools at all");
        assert_eq!(after("--model"), "sonnet");
        for f in ["--strict-mcp-config", "--safe-mode", "--no-session-persistence"] {
            assert!(a.iter().any(|x| x == f), "{f}");
        }
        // --bare would ignore the sign-in and demand an API key.
        assert!(!a.iter().any(|x| x == "--bare"));
        assert!(!question_args("s", "  ").iter().any(|x| x == "--model"), "Claude Code's own default");
    }

    #[test]
    fn earlier_turns_go_with_the_new_question() {
        assert_eq!(prompt(&[turn(true, "What key?")]), "What key?");
        let p = prompt(&[turn(true, "What key?"), turn(false, "E minor."), turn(true, "Which scale?")]);
        let at = |s: &str| p.find(s).unwrap_or_else(|| panic!("{s} missing from {p}"));
        assert!(at("<person>\nWhat key?\n</person>") < at("<assistant>\nE minor.\n</assistant>"));
        assert!(p.ends_with("newest message:\n\nWhich scale?"));
        assert_eq!(prompt(&[]), "");
    }

    #[test]
    fn replies_are_read_and_complaints_explained() {
        let ok = r#"{"type":"result","subtype":"success","is_error":false,"result":" E minor. "}"#;
        assert_eq!(read_reply(ok, ""), Ok("E minor.".into()));
        assert_eq!(read_reply(&format!("Warning: something\n{ok}\n"), ""), Ok("E minor.".into()));
        assert_eq!(read_reply(&format!(r#"[{{"type":"system"}},{ok}]"#), ""), Ok("E minor.".into()));
        // As Claude Code 2.1 says it: an error, though the subtype reads "success".
        let out = r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in · Please run /login"}"#;
        assert_eq!(read_reply(out, ""), Err(SIGN_IN.into()));
        let model = r#"{"type":"result","subtype":"success","is_error":true,"api_error_status":404,"result":"There's an issue with the selected model (x)."}"#;
        assert_eq!(read_reply(model, ""), Err("Claude Code stopped: There's an issue with the selected model (x).".into()));
        let turns = r#"{"type":"result","subtype":"error_max_turns","is_error":true}"#;
        assert_eq!(read_reply(turns, ""), Err("Claude Code stopped: error_max_turns".into()));
        assert!(read_reply("", "error: unknown option '--safe-mode'\n").unwrap_err().contains("claude update"));
        assert_eq!(read_reply("", "\nboom\n"), Err("Claude Code stopped: boom".into()));
        assert!(read_reply("", "").is_err());
        assert!(read_reply(r#"{"type":"result","is_error":false,"result":""}"#, "").is_err());
        assert!(read_reply("{}", "").is_err());
    }

    #[test]
    fn the_sign_in_is_described_without_naming_the_person() {
        let plan = r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"pro","email":"someone@example.com"}"#;
        assert_eq!(read_status(plan), Test::Ready("Signed in with your Claude pro plan.".into()));
        let key = r#"{"loggedIn":true,"authMethod":"api_key","apiProvider":"firstParty","apiKeySource":"ANTHROPIC_API_KEY"}"#;
        assert_eq!(read_status(key), Test::Ready("Signed in with an API key (ANTHROPIC_API_KEY), which is billed.".into()));
        let cloud = r#"{"loggedIn":true,"apiProvider":"bedrock"}"#;
        assert_eq!(read_status(cloud), Test::Ready("Signed in. Questions go through bedrock.".into()));
        assert_eq!(read_status(r#"{"loggedIn":false,"authMethod":"none"}"#), Test::Failed(SIGN_IN.into()));
        assert_eq!(read_status("A warning.\n{\n  \"loggedIn\": true\n}\n"), Test::Ready("Signed in.".into()));
        assert!(matches!(read_status("not json"), Test::Failed(_)));
        assert!(matches!(read_status("{}"), Test::Failed(_)));
    }

    #[test]
    fn only_a_claude_sessions_own_variables_are_removed() {
        let names = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        let all = ["PATH", "ANTHROPIC_API_KEY", "CLAUDE_CONFIG_DIR", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_SESSION_ID", "CLAUDECODE"];
        assert_eq!(session_vars(names(&all).into_iter()), names(&["CLAUDE_CODE_SESSION_ID", "CLAUDECODE"]));
        // Outside a session they are the person's own settings.
        assert!(session_vars(names(&["CLAUDE_CODE_MAX_OUTPUT_TOKENS"]).into_iter()).is_empty());
    }

    #[test]
    fn the_program_is_found_but_not_the_desktop_app() {
        let root = std::env::temp_dir().join(format!("guitarcraft-claude-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let put = |p: &Path| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        };
        let (empty, npm, app) = (root.join("empty"), root.join("npm"), root.join("AnthropicClaude"));
        std::fs::create_dir_all(&empty).unwrap();
        if cfg!(windows) {
            let real = npm.join("node_modules").join("@anthropic-ai").join("claude-code").join("bin").join("claude.exe");
            put(&real);
            put(&npm.join("claude.cmd"));
            put(&app.join("claude.exe"));
            assert_eq!(find_in(&[empty.clone(), app.clone(), npm]), Some(real));
            let native = root.join("bin").join("claude.exe");
            put(&native);
            assert_eq!(find_in(&[root.join("bin")]), Some(native));
        } else {
            put(&npm.join("claude"));
            assert_eq!(find_in(&[empty.clone(), npm.clone()]), Some(npm.join("claude")));
        }
        assert_eq!(find_in(&[empty, app, PathBuf::new()]), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A program that runs for half a minute.
    fn slow() -> Command {
        let mut cmd = if cfg!(windows) { Command::new("ping") } else { Command::new("sleep") };
        cmd.args(if cfg!(windows) { &["-n", "30", "127.0.0.1"][..] } else { &["30"][..] });
        cmd
    }

    #[test]
    fn a_question_can_be_stopped_and_has_a_time_limit() {
        let started = Instant::now();
        let late = run(slow(), String::new(), Duration::from_millis(200), &Stop::default());
        assert!(late.err().unwrap().contains("didn't answer"));

        let stop = Stop::default();
        let other = stop.clone();
        let waiting = std::thread::spawn(move || run(slow(), String::new(), Duration::from_secs(60), &other).err());
        std::thread::sleep(Duration::from_millis(200));
        stop.stop();
        assert_eq!(waiting.join().unwrap(), Some("Stopped.".into()));

        // Stopped before it began: nothing is started.
        assert_eq!(run(slow(), String::new(), Duration::from_secs(60), &stop).err(), Some("Stopped.".into()));
        assert!(started.elapsed() < Duration::from_secs(20), "the program was ended, not waited for");
    }

    /// Asks Claude two things in a row through the installed, signed-in Claude Code, and shows
    /// what it was told about the person and the computer. Uses a little of the plan's allowance.
    /// Run by hand: `cargo test claude_code_answers_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn claude_code_answers_live() {
        println!("TEST: {:?}", test());
        let system = super::super::ai::SYSTEM;
        let model = std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| MODEL.into());
        let mut turns = vec![turn(true, "Which note is the open high E string on a guitar? Quote \"E\" in your answer.")];
        let first = ask(system, &model, &turns, Duration::from_secs(120), &Stop::default()).expect("Claude should answer");
        println!("ANSWER: {first}");
        assert!(first.contains('E'));
        turns.push(turn(false, &first));
        turns.push(turn(true, "And the string below it? Then list the folder you are working in, if you were told one."));
        let second = ask(system, &model, &turns, Duration::from_secs(120), &Stop::default()).expect("Claude should answer");
        println!("ANSWER: {second}");
        assert!(second.contains('B'));
    }
}
