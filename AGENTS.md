# GhostMD — Agent Guide

## Project Overview

GhostMD is a native macOS note-taking app written in Rust using GPUI (GPU-accelerated UI framework). Notes are plain `.md` files in `~/Documents/ghostmd/`.

## Architecture

Cargo workspace with two crates:

- **`crates/ghostmd-core/`** — Pure business logic, zero UI dependencies. All data structures and algorithms live here.
  - `buffer.rs` — Rope-based text buffer (ropey 2.0.0-beta.1) with branching undo tree (undo 0.44)
  - `note.rs` — Note CRUD and auto-save
  - `diary.rs` — Date-based diary path generation (`diary/YYYY/month-name/DD/`)
  - `tree.rs` — File tree model with recursive scan
  - `search.rs` — Fuzzy file search (nucleo-matcher) + full-text content search (grep-searcher)

- **`crates/ghostmd/`** — Native GPUI application shell. State machines are tested independently of GPUI rendering.
  - `app_view.rs` — **Main GPUI view**. Contains `GhostAppView`, `Workspace`, `SplitNode`, `Pane`. This is the primary file for UI wiring — multi-workspace tabs, split panes, pane titles, focus indicators, command palette overlay, AI title generation
  - `editor_view.rs` — GPUI view wrapping `InputState` for editing a single note. Tracks path, dirty flag, auto-save timing
  - `file_tree_view.rs` — GPUI view for the sidebar file tree (renders `FileTreePanel`)
  - `app.rs` — Legacy workspace state (overlays, splits, tabs, sidebar). Partially superseded by `app_view.rs` but still used for `GhostApp` (root dir, sidebar toggle, open_files list)
  - `editor.rs` — Legacy editor panel state machine wrapping UndoBuffer. Superseded by `editor_view.rs` but tests kept
  - `file_tree.rs` — File tree sidebar state machine with keyboard navigation
  - `search.rs` — File finder and content search overlay state machines (not yet wired to GPUI)
  - `tabs.rs` — Tab manager with closed-tab restore history
  - `splits.rs` — Legacy flat split pane layout. Superseded by tree-based `SplitNode` in `app_view.rs`
  - `palette.rs` — Command palette state machine (filtering, selection). Wired into `app_view.rs`
  - `ai.rs` — AI manager for suggestion storage/retrieval. Not yet wired to GPUI
  - `theme.rs` — Warm dark color scheme (`GhostTheme`) with `rgb_to_hsla` converter and gpui-component theme application
  - `keybindings.rs` — GPUI action definitions and keyboard shortcut registration
  - `assets.rs` — Asset loading (fonts)
  - `main.rs` — Application entry point, window creation

## Building & Testing

```sh
cargo build                     # full build
cargo test                      # all tests (202: 117 ghostmd + 77 core + 8 integration)
cargo test -p ghostmd-core      # core logic only (77 tests)
cargo test -p ghostmd           # UI state machine tests (117 tests)
cargo bench -p ghostmd-core     # criterion benchmarks (buffer, tree, search)
cargo clippy --tests            # must pass with zero warnings
```

Requires Rust 1.75+ and Xcode with Metal Toolchain on macOS.

## Key Technical Details

- **ropey 2.0.0-beta.1** uses byte indices, not char indices. All buffer operations work in bytes.
- **undo 0.44** uses `Action` trait (not `Edit`). Methods: `apply`, `undo`, `merge` returning `Merged`.
- **GPUI dependencies** require pinning `core-foundation = "=0.10.0"` and `core-text = "=21.0.0"` to avoid conflicts.
- **Diary paths** use lowercase month names: `diary/2026/march/03/HHMMSS-slug.md`.
- **Dead code policy**: No crate-level `#![allow(dead_code)]`. Each module/item that is tested but not yet wired to GPUI gets its own `#[allow(dead_code)]` or module-level `#![allow(dead_code)]`. When wiring new features, remove the corresponding allows.
- **Modules with `#![allow(dead_code)]`** (entirely unwired): `ai.rs`, `editor.rs`, `splits.rs`, `search.rs`. All other dead code is suppressed per-item.

## App Structure (app_view.rs)

The GPUI app uses a multi-workspace model:

- **`GhostAppView`** — Root view. Holds shared `editors: HashMap<PathBuf, Entity<EditorView>>` cache, a `Vec<Workspace>`, and `active_workspace` index.
- **`Workspace`** — Contains `split_root: SplitNode`, `panes: HashMap<usize, Pane>`, `focused_pane`, `title`, and `title_generated` flag.
- **`SplitNode`** — Binary tree of splits. `Leaf(pane_id)` or `Split { direction, left, right }`. Methods: `leaves()`, `split_leaf()`, `remove_leaf()`.
- **`Pane`** — Just holds `active_path: Option<PathBuf>`.

Key patterns:
- Editors are shared globally across workspaces (same file = same editor entity)
- `cleanup_unused_editors()` GCs editors not referenced by any pane in any workspace
- Workspace titles are generated async via `claude -p` CLI, with fallback to first pane's file title
- The borrow checker requires extracting data from `self.workspaces[idx]` before calling `self.active_ws_mut()` — watch for this when adding methods that read editors while mutating workspace state

## Keybindings

| Shortcut | Action |
|----------|--------|
| cmd-n | New diary note in focused pane |
| cmd-shift-n | New workspace with diary note |
| cmd-w | Close pane; last pane closes workspace |
| cmd-shift-t | Restore last closed workspace |
| ctrl-tab | Next workspace |
| ctrl-shift-tab | Previous workspace |
| cmd-d | Split right (vertical) |
| cmd-shift-d | Split down (horizontal) |
| alt-cmd-arrows | Focus pane in direction |
| cmd-s | Save |
| cmd-b | Toggle sidebar |
| cmd-p | File finder (TODO) |
| cmd-shift-f | Content search (TODO) |
| cmd-shift-p | Command palette |
| cmd-q | Quit |

## Not Yet Wired (TODO)

- File finder overlay (`cmd-p`) — state machine exists in `search.rs::FileFinder`, needs GPUI view
- Content search overlay (`cmd-shift-f`) — state machine exists in `search.rs::ContentSearchPanel`, needs GPUI view
- AI suggestion manager (`ai.rs::AiManager`) — storage/retrieval works, not connected to UI
- Command palette execution — palette renders and filters but Enter/Escape don't dispatch actions yet

## Agent Preferences

- Don't create unnecessary markdown files
- Do not edit markdown files without asking for permission
- ALWAYS COMMIT YOUR CHANGES, and do it with a succinct one-liner and no attribution (don't add "Co-Authored-By" type of shit)
- Always combine `git add` and `git commit` in a single shell command (e.g. `git add file.ts && git commit -m "msg"`) — never split them into separate tool calls, as other agents working on the repo can race you and either commit your staged files or have their files included in your commit
- NEVER amend commits that have already been pushed
- NEVER amend commits at all - other agents might be working concurrently and amending can cause conflicts
- NEVER use `git restore` or `git checkout` to discard changes - other agents may be working concurrently and their changes could be lost
- NEVER use `git commit -a` - always stage specific files to avoid committing adjacent changes from other agents
