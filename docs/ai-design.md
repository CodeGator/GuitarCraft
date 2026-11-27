# AI integration: ideas and design

Status: ideas, 2026-11-23. Built so far: Preferences > AI (`src/app/ai.rs`: the Ollama and
Claude settings, the Claude key in the system credential store, a "Test connection" button
for each), and, the same day, the **AI Assistant panel** (`src/app/aipanel.rs`, a dockable
pane; `ai::ask` sends a conversation to Ollama's `/api/chat` or Claude's messages call on a
background thread, not streamed). The panel answers in words, and can propose changes to the score as cards (`src/app/aiplan.rs`: copy, paste and delete bars; Apply is one undoable edit): an
the bars selected, or the one at the cursor, sent automatically (ASCII tablature, key and time
signature, no title or artist, at most 16 bars, plus facts the program works out from the notes:
the notes used, the scales that contain them and the chord on each beat, `src/app/aifacts.rs`, and a summary of the whole track: its key, every note used and the scales that fit best, with the share of notes each covers),
Return to send (Shift+Return for a new line; no Send button), Stop, New chat (at the top of the
panel). The "What will be sent" preview was built and then removed at the owner's request.
Ollama can be kept loaded: the model is loaded when the panel shows it, and kept for 30 minutes
(Preferences > AI > Keep model loaded). Tested with unit tests and one live Ollama question
(`cargo test ollama_answers_live -- --ignored`); a Claude key was accepted by "Test connection" but no Claude answer has been seen (answering needs paid API credits). This is
the base the ideas below build on: the next step is idea 1 or 2, with the preview and apply
steps. On 2026-11-27 a third provider was added, **Claude Code** (`src/app/aicode.rs`): the
person's own signed-in `claude` program, so no API key (see "Claude Code" below). This document lists what the AI could be used for, picks a few to build first, and sets
the rules every one of them follows.

## What exists today

- **Providers** (Preferences > AI): Ollama (address, model, context size `num_ctx`), Claude Code
  (model) and Claude (API key, model). Each has an on switch. The key never goes in
  `preferences.json`.
- **Plumbing:** `ai.rs` talks to Ollama and Claude by running `curl`, and to Claude Code by
  running `claude`, on a separate thread, handing the answer back over a channel (`Probe`,
  `Pending`), so the window never freezes. The address and the key go to `curl` on its standard
  input, never on the command line. The Claude API key is the only secret GuitarCraft keeps (in
  the system credential store); `preferences.json` is plain text, so a user name and password in
  the Ollama address are removed (`ai::without_password`) and a server that needs them can't be
  used.
- **Things an AI could read or write, as plain text:** ASCII tab (`textmidi.rs`, export and
  import), chord chart text with inline chords (`[E]Let my heart be a [B]temple`, `chart.rs`),
  MusicXML, and the chord and scale tables in `theory.rs` / `chordshape.rs`.
- **Ways an AI could act:** every edit goes through `App::edit`, so it is undoable; the
  command palette now has three modes (commands, `@` actions, `>` expression text) with a
  fixed list of things it can do.

## Rules for every feature

1. **Nothing of the person's is sent until they ask.** No background calls that carry their
   music or text, no sending on open or on save. Which provider is used is set in Preferences >
   AI, and the panel shows nothing about it (the owner's choice: there is no room, and the
   outcome is what matters). Ollama on the same computer sends nothing off it; Claude and Claude
   Code do, and Preferences > AI says so. (The earlier ideas of a "Show what is sent" line and a "Goes to ..."
   line were dropped by the owner.)
   One exception, for Ollama only: loading the model in the background when the panel shows
   it, so the first answer is faster. That request names the model and the context size and
   carries nothing else, and Preferences > AI > Keep model loaded turns it off.
2. **Send the least.** The selection or the current bar range by default, never the whole song
   unless asked. GuitarCraft never adds file names, paths or the person's details (Preferences >
   Misc). An online provider needs some personal information, with no way round it: the key or
   sign-in says who is asking, and Claude Code also adds the account's email address, the
   operating system, the date and the folder it runs in (the top folder of the system drive, so
   it names nobody). That is accepted when the person switches the provider on. What is sent must
   be said in Preferences > AI, the guide, the README and the privacy policy.
3. **The AI proposes, the person applies.** Results appear as a preview (the proposed bars
   drawn in the score in a different colour, or a list of changes) with Apply and Discard.
   Apply is one `App::edit`, so one Undo takes it back.
4. **Answers are checked before they are used.** Ask for a fixed JSON shape, parse it, and
   check it against the model (fret within the track's range, durations that add up to the
   bar, chord names the app can read). What doesn't pass is dropped and reported, never
   half-applied.
5. **Small local models are the baseline.** A feature has to be useful with a 4 to 8 billion
   parameter Ollama model on a 16K context, and be better with Claude. That means narrow
   tasks, short prompts, and the app doing everything that doesn't need judgement (key
   detection, bar lengths, spelling).
6. **Cancel and time out.** Every request can be cancelled; every request has a limit (the
   test uses 15 seconds; a real request needs longer, with a visible Stop button).
7. **Failures are plain.** "Ollama isn't running at http://localhost:11434", "Claude says the
   key is invalid", "the answer wasn't music GuitarCraft could read" with the provider's own
   message when it gave one.
8. **A feature that works without AI keeps working without it.** The AI is an extra route
   into a tool, not a replacement for its controls.

## Ideas

Ranked by how well they fit the rules above. Effort is small (days), medium or large.

### Build first

1. **Lyrics syllables.** The Lyrics tool wants `Hel-lo ev-ery-bo-dy`. Paste plain lyrics and
   split them into syllables. Tiny prompt, easy to check (the syllables must rejoin to the
   same words), works with any local model. *Effort: small.*
2. **Chord suggestions for a chord chart.** Select lines of a `.gcx` section and ask for chords
   in a key ("simple, folk", "jazzier"). The reply is the same text with `[chords]` inserted,
   which the chart parser already reads, and which the app can check against `theory.rs`.
   Preview as the chart with changes marked. *Effort: small to medium.*
3. **Ask about this passage.** A side panel: select bars, ask "what key is this?", "what scale
   fits bars 5 to 8?", "why does this sound tense?". The selection goes as ASCII tab plus the
   key and time signature the app already knows, plus a short list of the chords and scales
   `theory.rs` finds for it, so the model explains rather than computes. The answer is text
   only, so nothing to validate and nothing to apply. *Effort: medium.*
4. **Natural language in the command palette.** A fourth mode (`?`): "double the tempo of the
   chorus", "add a repeat around bars 5 to 8", "mute the bass". The model is given the list
   of palette commands, actions and expression entries and answers with a short list of them
   to run (a whitelist, never free-form edits), shown for confirmation. This reuses the work
   done for the Action list and Expression text. *Effort: medium.*

### Worth doing after that

5. **Practice material.** "A 4-bar blues lick in A, bends, around the 5th position" or "a
   bass line for these chords". The model returns bars as JSON (string, fret, duration);
   the preview is a new track or the bars after the cursor. The hardest checking of any idea
   here, since the notes have to be playable and fit the bars, so it needs a validator that
   also repairs (clamp frets, drop notes that don't fit). *Effort: large.*
6. **Fix messy tab.** Pasted ASCII tab from the web is often ragged (uneven columns, missing
   bar lines). Today the importer is strict. The model tidies the text first, then the existing
   importer reads it, so the AI never writes the score directly. *Effort: medium.*
7. **Alternative fingerings.** "Make this playable without leaving positions 5 to 7." Most of
   this is a search problem the app can do itself; the model, if used at all, only picks among
   the candidates the app produces. Probably better done without AI. *Effort: medium.*
8. **Name the sections.** Suggest Intro, Verse, Chorus markers from the repeat structure and
   the chords. Small, cheap, easy to undo. *Effort: small.*
9. **Help** (built): ask a question about GuitarCraft and answer from the user guide (the text already
   in `tools.rs`, `GUIDE`). Send the whole guide as context to Claude, the best matching
   sections to a local model. *Effort: small.*
10. **Explain a notation warning.** The bar-duration and notation checks say what is wrong;
    the model says why and what to try. Low value on its own. *Effort: small.*

### Probably not

- **Writing a whole song or arrangement.** Long answers, hard to check, and the result isn't
  the person's work. If wanted later, build on idea 5, bar by bar.
- **Anything that listens to audio** (transcribing a recording). The providers here take text.
- **Training or tuning on the person's files.** Out of scope, and against rule 2.

## How a request flows

```
   tool (lyrics, chart, panel, palette)
        |  builds a Request: provider, task, text to send, expected answer shape
        v
   ai::ask(request) -> thread: curl to Ollama or Claude -> channel
        |
        v
   ai::parse(task, reply) -> Proposal | Failure     (checked against the model)
        |
        v
   preview in the tool  -->  Apply = one App::edit  |  Discard
```

- **One entry point.** `ai::ask` takes a `Request` and returns a handle that is polled each
  frame, as `Probe` does. It generalises `Probe`; the two connection tests become the
  simplest request.
- **Ollama:** `POST /api/chat` with `"stream": false`, the chosen model, `options.num_ctx` from
  Preferences, and `"format"` set to a JSON schema when the task wants JSON.
- **Claude:** `POST /v1/messages` with the key header, the chosen model, a system prompt, and
  for JSON tasks a tool definition the model must call, so the reply is already structured.
- **Streaming:** not at first. Long answers (idea 3) are the first thing that would want it;
  add it for those when a one-minute wait is annoying.
- **Which provider:** the first one switched on (Ollama, then Claude Code, then Claude with a
  key). There is no choice in the panel; it is set in Preferences > AI.
- **Prompts live in one file** (`src/app/ai_prompts.rs`), one function per task, so they can
  be read and changed in one place and tested without a network.
- **Cost:** for Claude, estimate the size before sending (characters over about 3.5) and ask
  first above a limit set in Preferences > AI (a new "Ask before sending more than N tokens").

## Claude Code

Each question runs `claude` once (`aicode.rs`):

```
claude -p --output-format json --system-prompt <ours> --tools "" --strict-mcp-config
       --safe-mode --no-session-persistence [--model <chosen>]
```

- The question goes in on standard input, with the earlier turns written out above it, since no
  session is saved. One JSON reply comes back; the answer is its `result`, and `is_error` marks a
  failure (the `subtype` can still say "success").
- No tools, MCP servers, plugins, hooks or `CLAUDE.md`. Never `--bare`: it ignores the sign-in
  and demands an API key.
- GuitarCraft never reads the sign-in. "Test connection" runs `claude auth status --json`, which
  asks Claude nothing, and shows the plan or that an API key is in use, never the email address.
  A person's `ANTHROPIC_API_KEY` is left in place, so it is what gets billed when set.
- The program is looked for on the search path and where its installers put it. On Windows an npm
  install's `claude.cmd` is followed to its `claude.exe` (line breaks can't pass through a
  script), and the Claude desktop app's `claude.exe` is passed over.
- Stop and a five minute limit end the process.
- Variables a surrounding Claude Code session sets are removed, for when GuitarCraft is started
  from one.
- **Terms:** this runs the unmodified program the person installed and signed in to themselves.
  GuitarCraft must not offer a Claude login, handle tokens or resell usage. Anthropic's terms
  were not checked first hand when this was built; read the current ones before releasing it, and
  again before any paid version.

Checked live on 2026-11-27, Windows, Claude Code 2.1.332, a Pro plan
(`cargo test claude_code_answers_live -- --ignored --nocapture`): two linked questions answered.
Not checked: a score change card from Claude, the panel by hand, macOS, Linux, and a native (not
npm) install.

## Suggest Chords

Built 2026-11-27 (`src/app/chartsuggest.rs`), the first AI feature that is a button for one task
and not a request typed into the panel. In a chord chart: the Section panel's button or Section >
Suggest Chords.

- Sent: the section's lyric lines without chords, the chart's key and an optional style. Nothing
  else of the chart.
- Claude only (Claude Code first, then the API key); Ollama is never asked, even when it is on.
- The reply's words are never used. `merge` checks that every lyric line kept its letters, that
  every chord is a chord name and that there is at least one, then puts the chords on the
  person's own lines. Bar lines, blank lines and lines of chords alone are left as they are.
- A window shows Now beside Suggested. Use These is one undo step; Cancel, Escape and closing
  stop a question on its way.

**Why only chords, and only Claude** (a trial on 2026-11-27, one run of each):

| Chords for four short lyrics | Result |
|---|---|
| Claude Code, `sonnet` | words kept and chords placed in all four, about 2 s each; the owner judged them usable ("I suppose") |
| Ollama `qwen3.5:4b`, thinking on | 2 of 4 answered, 44 to 98 s |
| Ollama `qwen3.5:4b`, thinking off | words garbled in 2 of 4, about 3 s |

**Tidying pasted tab was tried and not built.** Four damaged copies of `Morning Walk` tab (75
beats), counted by GuitarCraft's importer: a few lines of plain clean-up code read 75 beats in
three of them (the code was written knowing the damage), Claude 75, 54, 69 and 75, the local
model at most 67. Claude's errors imported without complaint. Ragged columns were the one case
where only Claude helped (69 against 40). Better to teach the importer those clean-up rules.

Ollama note: GuitarCraft leaves a model's thinking on, and `qwen3.5:4b` then sometimes answered
nothing in the trial, so the panel may show "The model sent an empty answer." with it.

## Testing

- A **fake provider** (a function from request to canned reply) behind the same `ask`, used by
  every test, so the parsing, checking and applying are tested with no network.
- Each task keeps a few **recorded real replies**, good and bad (extra prose around the JSON,
  a fret out of range, a bar that doesn't add up), as test fixtures.
- Hand testing against a real Ollama and a real Claude key, noted here as it is done, as
  `midi-design.md` does.

## Build order

1. **`ai::ask`** and the fake provider, with the two existing connection tests moved onto it.
2. **Lyrics syllables** (idea 1): the smallest end-to-end feature, to settle the preview,
   apply, cancel and error pieces.
3. **Chord suggestions** (idea 2): first use of a preview inside the score area.
4. **Ask about this passage** (idea 3): the side panel, built (with the facts from `aifacts.rs`).
5. **Palette mode** (idea 4).
6. README, in-app help, Preferences note ("asked of them" wording), gap doc.

## Open questions

- **Should Claude be the default when both are on,** or should every tool ask each time?
- **Is a spending limit wanted in the app,** or is the size warning enough?
- **Do the answers need to be saved** with the song (a note on the bar) or are they throwaway?
- **Are the macOS and Linux key stores in scope** for the first release? Today only Windows
  and macOS keep the key; elsewhere Claude can only be used through Claude Code.
