# GhostMD — Agent Guide

## Project Overview

GhostMD is a native macOS note-taking app written in Rust using GPUI (GPU-accelerated UI framework). Notes are plain `.md` files in `~/Documents/ghostmd/`.

## Architecture

Cargo workspace with two crates (`crates/ghostmd`, `crates/ghostmd-core`). The workspace root `Cargo.toml` patches `gpui-component` to the vendored `crates/gpui-component/`. There is also a separate Swift iOS companion app at `ghostmd-ios/` (see below) — not part of the Cargo workspace.

- **`crates/ghostmd-core/`** — Pure business logic, zero UI dependencies. All data structures and algorithms live here.
  - `buffer.rs` — Rope-based text buffer (ropey 2.0.0-beta.1) with branching undo tree (undo 0.44)
  - `note.rs` — Note CRUD and auto-save
  - `diary.rs` — Date-based diary path generation (`diary/YYYY/MM-monthname/DD/`)
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
    - `nav_history.rs` — Back/forward navigation stack (cursor + path + workspace/pane id). Max 100 entries, dedupes when cursor moves <50 bytes, truncates forward branch on new push. `navigating_history` flag prevents recursive pushes during navigation. Not persisted across sessions. Used twice: one global stack (`nav_history`, alt-cmd-[/]) and one per-pane stack (`pane_nav_history: HashMap<pane_id, NavHistory>`, cmd-[/], never switches workspace/pane focus).
    - `file_undo.rs` — In-memory undo/redo for file ops (`Delete`, `DeleteBatch`, `Rename`, `Move`, `Create`). Backs up entire directory trees recursively as `(path, bytes)` pairs. Max 50 entries. `reverse_op()` morphs `Create` into `Delete`. Used by trash and file-tree rename/move.
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
  - `theme.rs` — Multi-theme support (`GhostTheme`, `ResolvedTheme` pre-converted HSLA cache) with `rgb_to_hsla` converter. ~23 themes; default is `RosePine`. Each theme defines 13 color slots (bg, fg, selection, cursor, line_number, sidebar_bg, tab_active/inactive, accent, error, border, pane_title_bg/fg).
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
- **Diary paths** use `MM-monthname` with lowercase month names: `diary/2026/03-march/15/HHMMSS-slug.md` (zero-padded DD).
- **String truncation** must use `chars().take(n)` not byte slicing `&s[..n]` — byte slicing panics on multi-byte UTF-8.
- **Dead code policy**: No crate-level `#![allow(dead_code)]`. Each module/item that is tested but not yet wired to GPUI gets its own `#[allow(dead_code)]` or module-level `#![allow(dead_code)]`. When wiring new features, remove the corresponding allows.
- **Modules with `#![allow(dead_code)]`** (entirely unwired / test-only): `ai.rs`, `app.rs`, `editor.rs`, `splits.rs`, `search.rs` (the state machine; the `FileFinder` is wired separately). All other dead code is suppressed per-item.
- **Vendored gpui-component**: The workspace root `Cargo.toml` has `[patch.crates-io]` pointing to `crates/gpui-component/`. Local patches to gpui-component (e.g. input behavior fixes) are applied there.
- **gpui-component features**: The `tree-sitter-languages` feature MUST be enabled on the `gpui-component` dependency for syntax highlighting to work. Without it, `code_editor("markdown")` silently falls back to a plain JSON grammar.
- **Testing with custom workspace**: Set `GHOSTMD_ROOT=/tmp/ghostmd-test cargo run` to use a test directory instead of `~/Documents/ghostmd/`. Useful for testing without touching personal notes.
- **Same-file multi-pane sync**: Each pane has its own independent `EditorView`/`InputState`. Edits in one pane only appear in another pane showing the same file after auto-save triggers the file watcher reload — there is no live in-memory sync yet.
- **`EditorView` reload gotcha**: `skip_next_change` suppresses the `Change` event fired during external file reloads — otherwise reload would mark the buffer dirty and trigger auto-save loops.
- **`ContentSearch` is regex, not literal**: `grep-regex` interprets the query as a regex pattern. Callers must escape special chars (`search.rs` in `crates/ghostmd/` has a regex-escape helper for literal matching).
- **`ContentSearch` is uncached** — each call walks the entire tree. `FuzzySearch` caches file paths and must be refreshed via `refresh_cache()`.
- **`FuzzySearch::refresh_cache()` caches files *and* folders** (root excluded) so cmd-P can surface folder names too. Haystack is the **relative path** (slash-joined), not just the leaf — so `"13"` matches `13/april/notes.md`. `nucleo`'s `match_paths()` config still weights leaf-segment hits highest. `refresh_dir_cache()` is the folder-only variant (used for the "move to folder" picker).
- **`unique_path()` caps at 100** — if 99 collision-suffixed candidates exist it returns the original path unmodified. Don't rely on it for unbounded collision avoidance.
- **`tree.rs` pins `diary/` first** — after every scan the diary dir is moved to index 0. Collapsed state is preserved across rescans. `.ghostmd/`, `.gitignore`, `.DS_Store` are filtered out.
- **`diary::slugify()` is ASCII-only** — non-ASCII alphanum is stripped, which can reduce a CJK or accent-heavy title to `untitled`. Diary timestamps use `Local::now()`, not UTC. Full diary path: `diary/YYYY/MM-monthname/DD/HHMMSS-slug.md` (zero-padded DD, lowercase month).
- **Three distinct history stacks**: `nav_history` (global back/forward across panes and workspaces, alt-cmd-`[` / alt-cmd-`]`), `pane_nav_history` (per-pane back/forward, cmd-`[` / cmd-`]`, keyed by pane id — only changes the file shown in the focused pane), and `Pane.path_history` (per-pane fallback used when the open file is deleted/moved). Don't conflate them. Per-pane histories survive workspace close/restore (pane ids are stable) but are dropped when a pane is closed via cmd-w.
- **`OverlayKind` variants**: `Palette`, `FileFinder`, `AgenticSearch`, `NoteSwitcher` (plus location picker and context menu, which are not part of the enum). Always exactly one or zero overlays open.

## App Structure (app_view/)

The GPUI app uses a multi-workspace model:

- **`GhostAppView`** — Root view. Holds `root: PathBuf`, `sidebar_visible`, `Vec<Workspace>`, `active_overlay: Option<OverlayKind>`, `theme: ResolvedTheme`, file watcher.
- **`Workspace`** — Contains `id: usize` (stable, monotonically increasing), `split_root: SplitNode`, `panes: HashMap<usize, Pane>`, `focused_pane`, `title`.
- **`SplitNode`** — Binary tree of splits. `Leaf(pane_id)` or `Split { direction, left, right }`. Methods: `leaves()`, `split_leaf()`, `remove_leaf()`, `find_left/right/up/down()`.
- **`Pane`** — Holds `active_path: Option<PathBuf>` and `editor: Option<Entity<EditorView>>`.

Key patterns:
- **`FileTreeView` is a focusable, keyboard-navigable "pane-like" target** but is NOT a real pane in `SplitNode`. It has its own `FocusHandle` and a `key_context("FileTree")` so context-scoped bindings (arrow keys, enter, cmd-enter, cmd-z/cmd-shift-z, esc) only fire when the tree is focused. Keyboard cursor uses the same `last_clicked` / `selected_paths` / `anchor_path` fields as mouse selection — `set_keyboard_selection()` keeps them in sync. `nav_move_left/right` are direction-aware: on dirs they expand/collapse; on files they jump to parent / first child.
- **`alt-cmd-left` fallthrough**: when the focused pane has no left neighbor and the sidebar is visible, focus the tree instead of no-op. `alt-cmd-b` always focuses the tree (and shows the sidebar if hidden). `esc` / `alt-cmd-right` while the tree is focused return focus to the active pane editor.
- **Hide-sidebar focus rescue**: `ToggleSidebar` when hiding refocuses the editor pane so arrow keys don't go to an invisible tree.
- **Finder match highlights**: `FileTreeView` carries a `match_paths: HashSet<PathBuf>` (and a precomputed `match_ancestors` set) that `set_match_paths()` updates. `GhostAppView::sync_finder_match_highlights()` pushes the current fuzzy-finder result set into the tree after every `set_query()`, and `clear_finder_match_highlights()` resets it on dismiss. Matched rows render with the accent color; ancestor dirs get a blended tint so collapsed parents of a match still light up.
- **Workspace ID vs index**: Always use `workspace.id` (stable) for async callbacks, never positional index which shifts on add/remove. `ai_loading: HashSet<usize>` stores workspace IDs.
- **Empty workspace guard**: Always check `self.workspaces.is_empty()` before calling `self.active_ws()` / `self.active_ws_mut()` — they index directly and will panic on empty vec.
- **Editor path updates**: Use `self.update_editor_paths(old, new, cx)` when renaming/moving files or directories. It handles both exact matches and child paths for directory moves.
- **Borrow checker**: The render method clones the active workspace to avoid borrow conflicts with `cx.listener()`. Extract data from `self.workspaces[idx]` before calling methods that take `&mut self`.
- **Overlays**: `active_overlay: Option<OverlayKind>` enum ensures only one overlay at a time. Use `self.overlay_is(OverlayKind::Palette)` to check. `dismiss_overlays()` closes the current overlay via `match`.
- **ResolvedTheme**: Use `self.theme.fg`, `self.theme.accent`, etc. instead of calling `rgb_to_hsla()` per render. Rebuilt automatically on theme switch.
- **Collision avoidance**: Use `ghostmd_core::path_utils::unique_path()` for safe file/folder creation — appends `-2`, `-3`, etc.

## iOS Companion App (`ghostmd-ios/`)

Separate Swift / SwiftUI app, not part of the Cargo workspace. Targets iOS 17+. Generated from `project.yml` via XcodeGen (the `.xcodeproj/` is gitignored and regenerated). It shares notes with the macOS app via the filesystem only — it reads/writes the same `.md` files in `Documents/ghostmd/` (or the iCloud ubiquity container if available). No IPC with the Rust app. UI hides `.ghostmd/` and non-markdown files. Most logic lives in `GhostMD/NoteStore.swift`; models are intentionally thin.

## Ancillary directories

- **`scripts/`** — `bundle-macos.sh` (build the macOS `.app` from the cargo binary), `install.sh` (cross-platform installer that fetches the latest GitHub release).
- **`docs/`** — `index.html`, deployed to GitHub Pages by `.github/workflows/pages.yml`.
- **`assets/`** — App icons (`AppIcon.icns`, `icon.png`) and JetBrains Mono font files (embedded via `rust_embed` in `assets.rs`).
- **`.github/workflows/`** — `ci.yml` (test on macOS + Linux), `release.yml` (build macOS binaries on `v*` tags), `auto-tag.yml` (tags on version bump), `pages.yml` (docs site).

## Keybindings

| Shortcut | Action |
|----------|--------|
| cmd-n | New note (shows location picker if folder selected) |
| cmd-option-n | New daily note (carries over pending `- [ ]` items from last diary note) |
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
| cmd-p | File finder (matches files **and** folders; folder names show with a trailing slash and reveal in the tree) |
| cmd-enter (in file finder) | Open selected file in a new split (folder result falls back to "reveal in tree") |
| alt-cmd-b | Focus the sidebar file tree (auto-shows it if hidden) |
| alt-cmd-left (from leftmost pane) | Focus the sidebar file tree (otherwise: focus pane to the left) |
| up / down (file tree focused) | Move tree cursor |
| left / right (file tree focused) | Collapse / expand dir (or jump to parent / first child) |
| enter (file tree focused) | Open file (or toggle directory expansion) |
| cmd-enter (file tree focused) | Open file in a new split |
| esc / alt-cmd-right (file tree focused) | Return focus to the editor pane |
| cmd-shift-f | Agentic search (Claude-powered) |
| cmd-shift-p | Command palette |
| cmd-shift-a | Open note switcher (search open notes) |
| cmd-f | Find in file |
| cmd-[ / cmd-] | Navigate back / forward within the focused pane |
| alt-cmd-[ / alt-cmd-] | Navigate back / forward globally (across panes and workspaces) |
| cmd-z / cmd-shift-z (file tree focused) | Undo / redo file ops (delete, rename, move, create) |
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
- After every committed change, update `CHANGELOG.md` under the `[Unreleased]` section with a concise description of what changed. When bumping a version, move unreleased entries under the new version heading with the date.
- **Always run `cargo check` (or `cargo test -p ghostmd-core` if GPUI Metal shaders fail locally) after making changes** to catch compilation errors before committing. CI runs on Linux and will reject broken builds.
