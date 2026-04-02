# Changelog

All notable changes to GhostMD are documented in this file.

## [Unreleased]

- Add Go Back / Go Forward to command palette with shortcut hints
- Fix build: use public accessor methods for `EditorView` cursor instead of accessing private `input_state` field

## [0.8.0] — 2026-04-01

- **Navigation history**: Cmd+[ / Cmd+] to go back/forward through file, pane, and workspace locations
- Show animated "Updating..." indicator when downloading an update (replaces silent background download)

## [0.7.3] — 2026-03-22

- Remove `*` dirty indicator from workspace tabs (pane title bar already shows `●`)
- Add collapse-all button (`⊟`) in the sidebar header
- Click pane title bar to reveal and highlight the file in the sidebar tree
- Diary paths now use `MM-month` format (e.g. `diary/2026/03-march/21/`)

## [0.7.2] — 2026-03-21

- **Note switcher** (Cmd+Shift+A): fuzzy search all open notes across tabs, ranked by title match then content match, with tab name displayed on each result
- **Custom workspace root**: set `GHOSTMD_ROOT` env var to use an alternate notes directory
- Fix release CI: merge auto-tag and release into single workflow (GITHUB_TOKEN tags don't trigger other workflows)

## [0.7.1] — 2026-03-16

- **Markdown syntax highlighting**: toggleable tree-sitter-based highlighting for markdown files (persisted in session)
- **Live cross-pane sync**: edits in one pane instantly update other panes showing the same file
- **Friendly dates** in the file tree: today, yesterday, weekday name, or medium date
- Full path display in panes, fuzzy search improvements, diary notes named 'notes'
- Fix release CI: drop linux aarch64 cross-compile target (missing cross-compile libs)

## [0.6.0] — 2026-03-08

- **Linux support**: cross-platform keybindings, RNG, paths, CI, and install script
- 5 new light themes: Ayu Light, Gruvbox Light, Everforest Light, Nord Light, Tokyo Night Day
- File tree undo/redo (Cmd+Z / Cmd+Shift+Z when sidebar focused)
- Reuse editor entity when switching files to fix soft-wrap flicker
- Restore previous file in pane after deleting the current one
- iOS app with iCloud sync (ghostmd-ios)

## [0.5.1] — 2026-03-08

- Miscellaneous bug fixes and stability improvements

## [0.5.0] — 2026-03-08

- Move to folder (Cmd+Shift+M): file finder in folder-only mode
- Drag-and-drop file moves in the file tree
- Collision-safe path generation (appends -2, -3, etc.)
- Session persistence improvements

## [0.4.5] — 2026-03-07

- Prioritize today's diary notes at top of file finder (Cmd+P)
- Fix: apply saved theme on startup so Input text renders correctly
- Fix: restart after update by spawning detached relaunch process

## [0.4.4] — 2026-03-07

- Increase unfocused pane opacity from 0.5 to 0.85 for readability
- Fix: click-to-focus through editor Input

## [0.4.3] — 2026-03-07

- 4 new light themes: Solarized Light, Catppuccin Latte, Rosé Pine Dawn, GitHub Light
- Only refresh file tree on structural fs events, not content modifications
- Spinner + fade-out old→new path transition for AI rename/suggest
- Fix light theme palette selection contrast

## [0.4.2] — 2026-03-07

- Share as Gist command (private gist via `gh` CLI)
- Skip inline rename for Cmd+N: create file and open directly
- Dark/light labels on theme palette entries, sorted alphabetically
- Fix save resetting cursor, fix Enter key not inserting newlines

## [0.4.1] — 2026-03-07

- Semver comparison for update check
- Fix M-< / M-> keybindings
- Fix location picker focus

## [0.4.0] — 2026-03-07

- Multi-workspace tabs with session persistence
- Split panes (Cmd+D vertical, Cmd+Shift+D horizontal) with 2D navigation
- Command palette (Cmd+Shift+P) with fuzzy filtering
- Agentic search (Cmd+Shift+F) using Claude CLI
- File finder (Cmd+P) with fuzzy filename + content search
- File tree with context menu, drag-and-drop, inline rename
- 19 themes (dark and light)
- Emacs-style keybindings
- Auto-save with file watcher for external changes
- macOS .app packaging with CI/CD
