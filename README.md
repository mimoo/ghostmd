# GhostMD

A native macOS note-taking app that feels like [Ghostty](https://ghostty.org) — GPU-accelerated, keyboard-first, monospace, zero-config.

Notes are plain `.md` files in `~/Documents/ghostmd/`. No database, no lock-in. Back up the folder however you want.

## Philosophy

- **Fast path to writing**: open the app, you're typing. AI suggests a title and where to file it later.
- **Pure text**: no markdown rendering, no images. Raw monospace text like a terminal.
- **Files are the truth**: just a folder of `.md` files with whatever hierarchy you want.
- **Zero configuration**: sane defaults, no settings to fiddle with.
- **Keyboard-first**: Ghostty-style tabs/splits, Emacs text navigation, command palette.

## Features

- GPU-accelerated rendering via [GPUI](https://gpui.rs) (Metal on macOS)
- Branching undo tree (undo past a fork, type something new — old branch preserved)
- Auto-save on every keystroke (debounced 300ms) — quit anytime, nothing is lost
- Diary structure: new notes go to `~/Documents/ghostmd/diary/YYYY/MM/DD/`
- Always-visible file tree sidebar
- Fuzzy file finder (Cmd+P) and full-text search (Cmd+Shift+F)
- Command palette (Cmd+Shift+P)
- Ghostty-style tabs and splits
- AI-powered title suggestions and note organization via Claude Code
- JetBrains Mono, warm dark theme

## Keyboard Shortcuts

| Action | Binding |
|--------|---------|
| New note | Cmd+N |
| New tab | Cmd+T |
| Close tab | Cmd+W |
| Restore closed tab | Cmd+Shift+T |
| Next tab | Ctrl+Tab |
| Previous tab | Ctrl+Shift+Tab |
| Jump to tab 1-9 | Cmd+1..9 |
| Split right | Cmd+D |
| Split down | Cmd+Shift+D |
| Navigate splits | Opt+Cmd+Arrows |
| File finder | Cmd+P |
| Content search | Cmd+Shift+F |
| Command palette | Cmd+Shift+P |

### Emacs Navigation

| Action | Binding |
|--------|---------|
| Beginning of line | C-a |
| End of line | C-e |
| Forward char | C-f |
| Back char | C-b |
| Forward word | Opt-f |
| Back word | Opt-b |
| Delete forward | C-d |
| Kill line | C-k |
| Yank | C-y |

## Building

Requires Rust 1.75+ and Xcode with Metal Toolchain on macOS.

```
cargo build --release
cargo run --release
```

## Testing

```
cargo test                  # all 202 tests
cargo test -p ghostmd-core  # core logic (77 unit + 8 integration)
cargo test -p ghostmd       # UI logic (117 unit)
```

## Benchmarks

```
cargo bench -p ghostmd-core
```

Benchmarks cover buffer operations, file tree scanning (100-10K files), and fuzzy/content search at scale.

## Architecture

```
crates/
  ghostmd-core/   # Pure business logic, no UI dependency
    buffer.rs     # Rope-based text buffer with branching undo tree
    note.rs       # Note CRUD and auto-save
    diary.rs      # Date-based diary path generation
    tree.rs       # File tree model
    search.rs     # Fuzzy file search + full-text content search
  ghostmd/        # Native GPUI application
    editor.rs     # Editor panel state machine
    app.rs        # Workspace state (overlays, splits, tabs)
    file_tree.rs  # File tree sidebar with keyboard navigation
    search.rs     # File finder and content search overlays
    tabs.rs       # Tab manager with restore history
    splits.rs     # Split pane layout
    palette.rs    # Command palette
    ai.rs         # Claude Code integration for title/org suggestions
    theme.rs      # Warm dark color scheme
    keybindings.rs# All keyboard shortcut definitions
```

## License

MIT
