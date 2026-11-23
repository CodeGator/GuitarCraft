# AI integration: ideas and design

Status: ideas, 2026-11-23. Built so far: Preferences > AI (`src/app/ai.rs`: the Ollama and
Claude settings, the Claude key in the system credential store, a "Test connection" button
for each), and, the same day, the **AI Assistant panel** (`src/app/aipanel.rs`, a dockable
pane; `ai::ask` sends a conversation to Ollama's `/api/chat` or Claude's messages call on a
background thread, not streamed). The panel answers in words only: provider choice, an
optional "include the bars at the cursor / selected bars" box (ASCII tablature, key and time
signature, no title or artist, at most 16 bars, plus facts the program works out from the notes:
the notes used, the scales that contain them and the chord on each beat, `src/app/aifacts.rs`),
Return to send (Shift+Return for a new line; no Send button), Stop, New chat (at the top of the
panel). The "What will be sent" preview was built and then removed at the owner's request.
Ollama can be kept loaded: the model is loaded when the panel shows it, and kept for 30 minutes
(Preferences > AI > Keep model loaded). Tested with unit tests and one live Ollama question
(`cargo test ollama_answers_live -- --ignored`); not yet tried with a real Claude key. This is
the base the ideas below build on: the next step is idea 1 or 2, with the preview and apply
steps. This document lists what the AI could be used for, picks a few to build first, and sets
the rules every one of them follows.

## What exists today

- **Providers** (Preferences > AI): Ollama (address, model, context size `num_ctx`) and Claude
  (API key, model). Each has an on switch. The key never goes in `preferences.json`.
- **Plumbing:** `ai.rs` talks to the provider by running `curl` on a separate thread and
  handing the answer back over a channel (`Probe`), so the window never freezes. The key goes to
  `curl` on its standard input, never on the command line. Only the model-list calls exist.
- **Things an AI could read or write, as plain text:** ASCII tab (`textmidi.rs`, export and
  import), chord chart text with inline chords (`[E]Let my heart be a [B]temple`, `chart.rs`),
  MusicXML, and the chord and scale tables in `theory.rs` / `chordshape.rs`.
- **Ways an AI could act:** every edit goes through `App::edit`, so it is undoable; the
  command palette now has three modes (commands, `@` actions, `>` expression text) with a
  fixed list of things it can do.

## Rules for every feature

1. **Nothing of the person's is sent until they ask.** No background calls that carry their
   music or text, no sending on open or on save. The window says which provider gets a question
   ("Goes to ..."). Ollama on the same computer sends nothing off it; Claude does, and the
   window says so. (The earlier idea of a "Show what is sent" line was dropped by the owner.)
   One exception, for Ollama only: loading the model in the background when the panel shows
   it, so the first answer is faster. That request names the model and the context size and
   carries nothing else, and Preferences > AI > Keep model loaded turns it off.
2. **Send the least.** The selection or the current bar range by default, never the whole song
   unless asked. File names, paths and the person's details (Preferences > Misc) are never
   included.
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
9. **Help.** Ask a question about GuitarCraft and answer from the user guide (the text already
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
- **Which provider:** the first one switched on, Claude if both are, unless the tool lets
  the person choose. A small "Ask using: Ollama / Claude" choice in each tool's window.
- **Prompts live in one file** (`src/app/ai_prompts.rs`), one function per task, so they can
  be read and changed in one place and tested without a network.
- **Cost:** for Claude, estimate the size before sending (characters over about 3.5) and ask
  first above a limit set in Preferences > AI (a new "Ask before sending more than N tokens").

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
  and macOS keep the key; elsewhere there is no way to use Claude.
