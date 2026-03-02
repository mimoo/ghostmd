# GhostMD — Agent Guide

## Project Overview

GhostMD is a native macOS note-taking app written in Rust using GPUI (GPU-accelerated UI framework). Notes are plain `.md` files in `~/Documents/ghostmd/`.

## Architecture

Cargo workspace with two crates:

- **`crates/ghostmd-core/`** — Pure business logic, zero UI dependencies. All data structures and algorithms live here.
  - `buffer.rs` — Rope-based text buffer (ropey 2.0.0-beta.1) with branching undo tree (undo 0.44)
  - `note.rs` — Note CRUD and auto-save
  - `diary.rs` — Date-based diary path generation (`diary/YYYY/MM/DD/`)
  - `tree.rs` — File tree model with recursive scan
  - `search.rs` — Fuzzy file search (nucleo-matcher) + full-text content search (grep-searcher)

- **`crates/ghostmd/`** — Native GPUI application shell. State machines are tested independently of GPUI rendering.
  - `editor.rs` — Editor panel state machine wrapping UndoBuffer
  - `app.rs` — Workspace state (overlays, splits, tabs, sidebar)
  - `file_tree.rs` — File tree sidebar with keyboard navigation
  - `search.rs` — File finder and content search overlays
  - `tabs.rs` — Tab manager with restore history
  - `splits.rs` — Split pane layout
  - `palette.rs` — Command palette
  - `ai.rs` — Claude Code integration for title/org suggestions
  - `theme.rs` — Warm dark color scheme
  - `keybindings.rs` — All keyboard shortcut definitions

## Building & Testing

```sh
cargo build                     # full build
cargo test                      # all tests (~202)
cargo test -p ghostmd-core      # core logic only
cargo test -p ghostmd           # UI state machine tests
cargo bench -p ghostmd-core     # criterion benchmarks (buffer, tree, search)
```

Requires Rust 1.75+ and Xcode with Metal Toolchain on macOS.

## Key Technical Details

- **ropey 2.0.0-beta.1** uses byte indices, not char indices. All buffer operations work in bytes.
- **undo 0.44** uses `Action` trait (not `Edit`). Methods: `apply`, `undo`, `merge` returning `Merged`.
- **GPUI dependencies** require pinning `core-foundation = "=0.10.0"` and `core-text = "=21.0.0"` to avoid conflicts.
- The `main.rs` currently prints "GhostMD" — GPUI rendering is not yet wired.

## Agent Preferences

- Don't create unnecessary markdown files
- Do not edit markdown files without asking for permission
- ALWAYS COMMIT YOUR CHANGES, and do it with a succinct one-liner and no attribution (don't add "Co-Authored-By" type of shit)
- Always combine `git add` and `git commit` in a single shell command (e.g. `git add file.ts && git commit -m "msg"`) — never split them into separate tool calls, as other agents working on the repo can race you and either commit your staged files or have their files included in your commit
- NEVER amend commits that have already been pushed
- NEVER amend commits at all - other agents might be working concurrently and amending can cause conflicts
- NEVER use `git restore` or `git checkout` to discard changes - other agents may be working concurrently and their changes could be lost
- NEVER use `git commit -a` - always stage specific files to avoid committing adjacent changes from other agents
