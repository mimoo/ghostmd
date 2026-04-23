# Changelog

All notable changes to GhostMD are documented in this file.

## [Unreleased]

- New "→" button on the diary row: jumps to today's `notes.md` if it exists, otherwise opens the most recently modified file in today's folder, otherwise creates a new daily note (carrying over pending items)
- Double-click a workspace tab to rename it
- Tab rename input now auto-selects the name so typing immediately replaces it
- Fix: find-in-file (Cmd+F) Next/Prev now correctly scrolls when cycling wraps around (was pinned to viewport edge after wrapping)
- Find & Replace: Cmd+Opt+F opens the search panel with the replace row expanded; also added "Find in File" and "Replace in File" to the command palette
- Fix: after replacing a match, the match index is no longer stuck in "keep current" mode for subsequent searches

## [0.11.2] — 2026-04-21

- Fix: Cmd+N (and other shortcuts) now work immediately after creating/renaming a folder or file in the sidebar — no click required
- Fix: focus is properly restored after dismissing context menus (Escape or click-away) and canceling inline renames

## [0.11.1] — 2026-04-19

- AI commands (Rename File, Suggest Folder) now available in the file tree and pane title bar right-click menus
- AI: Rename All Tabs added to the tab right-click menu
- Update dependencies

## [0.11.0] — 2026-04-18

- Right-click on a workspace tab opens a context menu (Rename, AI Rename, Move to New Window, Close Other Tabs, Close Tab)
- Right-click on a pane's title bar opens the file context menu (Rename, New Note, Open in Finder, Copy Path/Name, Move to Trash)
- Context menus (tab and tree) now support keyboard navigation (arrow keys + Enter) and show shortcut hints
- "Copy Path" and "Copy Name" added to the file tree context menu
- "Duplicate" option in the tree context menu for files (copies with "-copy" suffix)
- Cmd+Enter in the file finder opens the selected file in a new split pane
- Overlays (file finder, command palette, agentic search, note switcher) now reliably grab keyboard focus when opened
- Clicking anywhere in the file tree sidebar now moves keyboard focus to the tree; fixes typing going to the editor pane when editing inline rename / new-folder names
- Extract shared overlay shell (reduces ~120 lines of duplicated boilerplate)
- Check for updates every 4 hours instead of only at launch
- File finder (Cmd+P) results are now clickable

## [0.10.1] — 2026-04-08

- Quick note (Cmd+Opt+N) only carries over pending `- [ ]` items for the first note of the day

## [0.10.0] — 2026-04-07

- Tear off a tab into a new window by dragging it outside the tab bar or outside the window
- Auto-select full filename when starting inline rename (double-click or palette) so typing immediately replaces it
- Drag-and-drop workspace tabs to reorder them (persisted across sessions)
- Cmd+N now creates a note in the selected directory (or root if nothing selected), removing the location picker
- Rename "New Daily Note" to "Quick Note" in command palette; add `+` button on diary folder in sidebar
- Daily note now preserves full header hierarchy (`#` > `##` > `###`) when carrying over pending items
- Fix navigation history (Cmd+[/]) recreating deleted files — now skips entries whose files no longer exist on disk

## [0.9.1] — 2026-04-04

- Fix daily note: pending items now correctly pulled from the previous day's notes (was finding today's empty dir instead)
- Fix build: `len_bytes()` → `len()` on ropey `Rope` type
- Push Cargo.lock to repo for reproducible CI builds

## [0.9.0] — 2026-04-02

- **New Daily Note** (Cmd+Opt+N): creates a diary note pre-filled with pending `- [ ]` items from the last diary note
- Add missing palette entries for all keyboard shortcuts (file finder, content search, note switcher, next/prev workspace, focus pane directions)
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
