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
  - `path_utils.rs` — Collision-safe path generation (`unique_path()` appends `-2`, `-3`, ...)

- **`crates/ghostmd/`** — Native GPUI application shell. State machines are tested independently of GPUI rendering.
  - `app_view/` — **Main GPUI view** (refactored into submodules):
    - `mod.rs` — `GhostAppView` struct, constructor, `Render` impl with action handlers
    - `workspace.rs` — Workspace CRUD, pane focus, split management, tab switching
    - `file_ops.rs` — File open/create/move/trash, location picker, `update_editor_paths()` helper, update mechanism
    - `rendering.rs` — All render methods: tab bar, split nodes, overlays (file finder, agentic search, location picker, command palette), context menu
    - `overlays.rs` — Open/close methods for all overlays
    - `palette_dispatch.rs` — Command palette command list and dispatch, rename mode
    - `ai_commands.rs` — AI rename tab/file, suggest folder, agentic search, search matches
    - `session.rs` — Session persistence (save/restore workspaces to JSON)
    - `split_node.rs` — `SplitNode` binary tree with directional navigation
    - `fs_watcher.rs` — File system watcher for external changes (notify crate)
  - `editor_view.rs` — GPUI view wrapping `InputState` for editing a single note. Tracks path, dirty flag, auto-save timing
  - `file_tree_view.rs` — GPUI view for the sidebar file tree (renders `FileTreePanel`)
  - `app.rs` — Legacy root state machine. `#![allow(dead_code)]` — fully superseded by `GhostAppView`'s direct `root` and `sidebar_visible` fields
  - `editor.rs` — Legacy editor state machine. `#[cfg(test)]` only
  - `file_tree.rs` — File tree sidebar state machine with keyboard navigation
  - `search.rs` — File finder state machine (wired to GPUI via `app_view/rendering.rs`)
  - `tabs.rs` — Legacy tab manager. `#[cfg(test)]` only
  - `splits.rs` — Legacy flat split pane layout. `#[cfg(test)]` only
  - `palette.rs` — Command palette state machine (filtering, selection)
  - `ai.rs` — AI manager for suggestion storage/retrieval. `#[cfg(test)]` only
  - `theme.rs` — Multi-theme support (`GhostTheme`, `ResolvedTheme` pre-converted HSLA cache) with `rgb_to_hsla` converter
  - `keybindings.rs` — GPUI action definitions and keyboard shortcut registration
  - `assets.rs` — Asset loading (fonts)
  - `main.rs` — Application entry point, window creation

## Building & Testing

```sh
cargo build                     # full build
cargo test                      # all tests
cargo test -p ghostmd-core      # core logic only
cargo test -p ghostmd           # UI state machine tests
cargo bench -p ghostmd-core     # criterion benchmarks (buffer, tree, search)
cargo clippy --tests            # must pass with zero warnings
```

Requires Rust 1.75+ and Xcode with Metal Toolchain on macOS.

## Versioning & Releases

- Version is defined in `crates/ghostmd/Cargo.toml` — this is the single source of truth.
- CI auto-creates a git tag (`vX.Y.Z`) when the version changes on main (`.github/workflows/auto-tag.yml`).
- The release workflow (`.github/workflows/release.yml`) triggers on `v*` tags and builds macOS binaries.
- **To release**: bump the version in `Cargo.toml` and push to main. CI handles the rest.
- **Known CI caveat**: Tags created by `GITHUB_TOKEN` (e.g. via the auto-tag workflow) do NOT trigger other workflows. If the release workflow doesn't fire after auto-tag, delete the remote tag and re-push it locally: `git push origin :refs/tags/vX.Y.Z && git tag vX.Y.Z && git push origin vX.Y.Z`.

## Key Technical Details

- **ropey 2.0.0-beta.1** uses byte indices, not char indices. All buffer operations work in bytes.
- **undo 0.44** uses `Action` trait (not `Edit`). Methods: `apply`, `undo`, `merge` returning `Merged`.
- **GPUI dependencies** require pinning `core-foundation = "=0.10.0"` and `core-text = "=21.0.0"` to avoid conflicts.
- **Diary paths** use lowercase month names: `diary/2026/march/03/HHMMSS-slug.md`.
- **String truncation** must use `chars().take(n)` not byte slicing `&s[..n]` — byte slicing panics on multi-byte UTF-8.
- **Dead code policy**: No crate-level `#![allow(dead_code)]`. Each module/item that is tested but not yet wired to GPUI gets its own `#[allow(dead_code)]` or module-level `#![allow(dead_code)]`. When wiring new features, remove the corresponding allows.
- **Modules with `#![allow(dead_code)]`** (entirely unwired / test-only): `ai.rs`, `app.rs`, `editor.rs`, `splits.rs`, `search.rs` (the state machine; the `FileFinder` is wired separately). All other dead code is suppressed per-item.
- **Vendored gpui-component**: The workspace root `Cargo.toml` has `[patch.crates-io]` pointing to `crates/gpui-component/`. Local patches to gpui-component (e.g. input behavior fixes) are applied there.
- **gpui-component features**: The `tree-sitter-languages` feature MUST be enabled on the `gpui-component` dependency for syntax highlighting to work. Without it, `code_editor("markdown")` silently falls back to a plain JSON grammar.
- **Testing with custom workspace**: Set `GHOSTMD_ROOT=/tmp/ghostmd-test cargo run` to use a test directory instead of `~/Documents/ghostmd/`. Useful for testing without touching personal notes.
- **Same-file multi-pane sync**: Each pane has its own independent `EditorView`/`InputState`. Edits in one pane only appear in another pane showing the same file after auto-save triggers the file watcher reload — there is no live in-memory sync yet.

## App Structure (app_view/)

The GPUI app uses a multi-workspace model:

- **`GhostAppView`** — Root view. Holds `root: PathBuf`, `sidebar_visible`, `Vec<Workspace>`, `active_overlay: Option<OverlayKind>`, `theme: ResolvedTheme`, file watcher.
- **`Workspace`** — Contains `id: usize` (stable, monotonically increasing), `split_root: SplitNode`, `panes: HashMap<usize, Pane>`, `focused_pane`, `title`.
- **`SplitNode`** — Binary tree of splits. `Leaf(pane_id)` or `Split { direction, left, right }`. Methods: `leaves()`, `split_leaf()`, `remove_leaf()`, `find_left/right/up/down()`.
- **`Pane`** — Holds `active_path: Option<PathBuf>` and `editor: Option<Entity<EditorView>>`.

Key patterns:
- **Workspace ID vs index**: Always use `workspace.id` (stable) for async callbacks, never positional index which shifts on add/remove. `ai_loading: HashSet<usize>` stores workspace IDs.
- **Empty workspace guard**: Always check `self.workspaces.is_empty()` before calling `self.active_ws()` / `self.active_ws_mut()` — they index directly and will panic on empty vec.
- **Editor path updates**: Use `self.update_editor_paths(old, new, cx)` when renaming/moving files or directories. It handles both exact matches and child paths for directory moves.
- **Borrow checker**: The render method clones the active workspace to avoid borrow conflicts with `cx.listener()`. Extract data from `self.workspaces[idx]` before calling methods that take `&mut self`.
- **Overlays**: `active_overlay: Option<OverlayKind>` enum ensures only one overlay at a time. Use `self.overlay_is(OverlayKind::Palette)` to check. `dismiss_overlays()` closes the current overlay via `match`.
- **ResolvedTheme**: Use `self.theme.fg`, `self.theme.accent`, etc. instead of calling `rgb_to_hsla()` per render. Rebuilt automatically on theme switch.
- **Collision avoidance**: Use `ghostmd_core::path_utils::unique_path()` for safe file/folder creation — appends `-2`, `-3`, etc.

## Keybindings

| Shortcut | Action |
|----------|--------|
| cmd-n | New note (shows location picker if folder selected) |
| cmd-shift-n | New OS window |
| cmd-t | New workspace tab |
| cmd-w | Close pane; last pane closes workspace |
| cmd-shift-t | Restore last closed workspace |
| ctrl-tab / ctrl-shift-tab | Next / previous workspace |
| cmd-1 through cmd-9 | Switch to workspace N |
| cmd-d | Split right (vertical) |
| cmd-shift-d | Split down (horizontal) |
| alt-cmd-arrows | Focus pane in direction |
| cmd-s | Save |
| cmd-b | Toggle sidebar |
| cmd-p | File finder |
| cmd-shift-f | Agentic search (Claude-powered) |
| cmd-shift-p | Command palette |
| cmd-f | Find in file |
| cmd-backspace | Move to trash |
| cmd-q | Quit |

## Agent Preferences

- Don't create unnecessary markdown files
- Do not edit markdown files without asking for permission
- ALWAYS COMMIT YOUR CHANGES, and do it with a succinct one-liner and no attribution (don't add "Co-Authored-By" type of shit)
- Always combine `git add` and `git commit` in a single shell command (e.g. `git add file.ts && git commit -m "msg"`) — never split them into separate tool calls, as other agents working on the repo can race you and either commit your staged files or have their files included in your commit
- NEVER amend commits that have already been pushed
- NEVER amend commits at all - other agents might be working concurrently and amending can cause conflicts
- NEVER use `git restore` or `git checkout` to discard changes - other agents may be working concurrently and their changes could be lost
- NEVER use `git commit -a` - always stage specific files to avoid committing adjacent changes from other agents
