use gpui::{actions, KeyBinding as GpuiKeyBinding};
use gpui_component::input::{
    Backspace, Delete, DeleteToEndOfLine, MoveLeft, MoveRight, MovePageDown, MovePageUp,
    MoveToEnd, MoveToNextWord, MoveToPreviousWord, MoveToStart, Paste,
};

// GPUI unit-struct actions
actions!(
    ghostmd,
    [
        NewNote,
        NewTab,
        NewWindow,
        Save,
        Quit,
        CloseTab,
        RestoreTab,
        NextTab,
        PrevTab,
        OpenFileFinder,
        OpenContentSearch,
        OpenCommandPalette,
        ToggleSidebar,
        SplitRight,
        SplitDown,
        FocusPaneLeft,
        FocusPaneRight,
        FocusPaneUp,
        FocusPaneDown,
        Escape,
        PaletteUp,
        PaletteDown,
        PaletteConfirm,
        SwitchTab1,
        SwitchTab2,
        SwitchTab3,
        SwitchTab4,
        SwitchTab5,
        SwitchTab6,
        SwitchTab7,
        SwitchTab8,
        SwitchTab9,
        MoveToTrash,
    ]
);

/// Register all GhostMD keyboard shortcuts with GPUI.
pub fn register_keybindings(cx: &mut gpui::App) {
    // "secondary" maps to Cmd on macOS and Ctrl on Linux/Windows
    cx.bind_keys([
        GpuiKeyBinding::new("secondary-n", NewNote, None),
        GpuiKeyBinding::new("secondary-t", NewTab, None),
        GpuiKeyBinding::new("secondary-shift-n", NewWindow, None),
        GpuiKeyBinding::new("secondary-s", Save, None),
        GpuiKeyBinding::new("secondary-q", Quit, None),
        GpuiKeyBinding::new("secondary-w", CloseTab, None),
        GpuiKeyBinding::new("secondary-shift-t", RestoreTab, None),
        GpuiKeyBinding::new("ctrl-tab", NextTab, None),
        GpuiKeyBinding::new("ctrl-shift-tab", PrevTab, None),
        GpuiKeyBinding::new("secondary-p", OpenFileFinder, None),
        GpuiKeyBinding::new("secondary-shift-f", OpenContentSearch, None),
        GpuiKeyBinding::new("secondary-shift-p", OpenCommandPalette, None),
        GpuiKeyBinding::new("secondary-b", ToggleSidebar, None),
        GpuiKeyBinding::new("escape", Escape, None),
        // Splits
        GpuiKeyBinding::new("secondary-d", SplitRight, None),
        GpuiKeyBinding::new("secondary-shift-d", SplitDown, None),
        GpuiKeyBinding::new("alt-secondary-left", FocusPaneLeft, None),
        GpuiKeyBinding::new("alt-secondary-right", FocusPaneRight, None),
        GpuiKeyBinding::new("alt-secondary-up", FocusPaneUp, None),
        GpuiKeyBinding::new("alt-secondary-down", FocusPaneDown, None),
        // Palette navigation
        GpuiKeyBinding::new("up", PaletteUp, None),
        GpuiKeyBinding::new("down", PaletteDown, None),
        GpuiKeyBinding::new("enter", PaletteConfirm, None),
        // Quick tab switching
        GpuiKeyBinding::new("secondary-1", SwitchTab1, None),
        GpuiKeyBinding::new("secondary-2", SwitchTab2, None),
        GpuiKeyBinding::new("secondary-3", SwitchTab3, None),
        GpuiKeyBinding::new("secondary-4", SwitchTab4, None),
        GpuiKeyBinding::new("secondary-5", SwitchTab5, None),
        GpuiKeyBinding::new("secondary-6", SwitchTab6, None),
        GpuiKeyBinding::new("secondary-7", SwitchTab7, None),
        GpuiKeyBinding::new("secondary-8", SwitchTab8, None),
        GpuiKeyBinding::new("secondary-9", SwitchTab9, None),
        // Move to Trash
        GpuiKeyBinding::new("secondary-backspace", MoveToTrash, None),
        // Emacs-style bindings (active when Input is focused)
        GpuiKeyBinding::new("ctrl-f", MoveRight, Some("Input")),
        GpuiKeyBinding::new("ctrl-b", MoveLeft, Some("Input")),
        GpuiKeyBinding::new("ctrl-p", PaletteUp, Some("Input")),
        GpuiKeyBinding::new("ctrl-n", PaletteDown, Some("Input")),
        GpuiKeyBinding::new("ctrl-k", DeleteToEndOfLine, Some("Input")),
        GpuiKeyBinding::new("shift-backspace", Backspace, Some("Input")),
        GpuiKeyBinding::new("ctrl-h", Backspace, Some("Input")),
        GpuiKeyBinding::new("ctrl-d", Delete, Some("Input")),
        GpuiKeyBinding::new("ctrl-y", Paste, Some("Input")),
        // Word movement (alt-f/b produce special chars on macOS, bind explicitly)
        GpuiKeyBinding::new("alt-f", MoveToNextWord, Some("Input")),
        GpuiKeyBinding::new("alt-b", MoveToPreviousWord, Some("Input")),
        // Page up/down (Emacs C-v / M-v)
        GpuiKeyBinding::new("ctrl-v", MovePageDown, Some("Input")),
        GpuiKeyBinding::new("alt-v", MovePageUp, Some("Input")),
        // Beginning/end of buffer (Emacs M-< / M->)
        GpuiKeyBinding::new("alt-<", MoveToStart, Some("Input")),
        GpuiKeyBinding::new("alt->", MoveToEnd, Some("Input")),
    ]);
}

#[cfg(test)]
/// All actions that can be triggered via keyboard shortcuts in GhostMD.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    // File operations
    NewNote,
    Save,
    Quit,

    // Tab management
    NewTab,
    CloseTab,
    RestoreTab,
    NextTab,
    PrevTab,

    // Splits
    SplitRight,
    SplitDown,

    // Panels / search
    OpenFileFinder,
    OpenContentSearch,
    OpenCommandPalette,

    // Editing
    Undo,
    Redo,

    // Emacs-style cursor movement
    EmacsMoveForward,
    EmacsMoveBackward,
    EmacsMovePrevLine,
    EmacsMoveNextLine,
    EmacsMoveBeginningOfLine,
    EmacsMoveEndOfLine,
    EmacsKillLine,
    EmacsDeleteBackward,
}

#[cfg(test)]
/// A keyboard modifier set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifiers {
    pub cmd: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[cfg(test)]
impl Modifiers {
    pub fn none() -> Self {
        Modifiers {
            cmd: false,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn cmd() -> Self {
        Modifiers {
            cmd: true,
            ..Self::none()
        }
    }

    pub fn ctrl() -> Self {
        Modifiers {
            ctrl: true,
            ..Self::none()
        }
    }

    pub fn cmd_shift() -> Self {
        Modifiers {
            cmd: true,
            shift: true,
            ..Self::none()
        }
    }
}

#[cfg(test)]
/// A key binding mapping a key + modifiers to an action.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: String,
    pub modifiers: Modifiers,
    pub action: Action,
}

#[cfg(test)]
/// Returns the default set of key bindings for GhostMD.
pub fn default_bindings() -> Vec<KeyBinding> {
    vec![
        // File operations
        KeyBinding {
            key: "n".into(),
            modifiers: Modifiers::cmd(),
            action: Action::NewNote,
        },
        KeyBinding {
            key: "s".into(),
            modifiers: Modifiers::cmd(),
            action: Action::Save,
        },
        KeyBinding {
            key: "q".into(),
            modifiers: Modifiers::cmd(),
            action: Action::Quit,
        },
        // Tab management
        KeyBinding {
            key: "t".into(),
            modifiers: Modifiers::cmd(),
            action: Action::NewTab,
        },
        KeyBinding {
            key: "w".into(),
            modifiers: Modifiers::cmd(),
            action: Action::CloseTab,
        },
        KeyBinding {
            key: "t".into(),
            modifiers: Modifiers::cmd_shift(),
            action: Action::RestoreTab,
        },
        KeyBinding {
            key: "tab".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::NextTab,
        },
        KeyBinding {
            key: "shift-tab".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::PrevTab,
        },
        // Splits
        KeyBinding {
            key: "\\".into(),
            modifiers: Modifiers::cmd(),
            action: Action::SplitRight,
        },
        KeyBinding {
            key: "-".into(),
            modifiers: Modifiers::cmd(),
            action: Action::SplitDown,
        },
        // Panels / search
        KeyBinding {
            key: "p".into(),
            modifiers: Modifiers::cmd(),
            action: Action::OpenFileFinder,
        },
        KeyBinding {
            key: "f".into(),
            modifiers: Modifiers::cmd_shift(),
            action: Action::OpenContentSearch,
        },
        KeyBinding {
            key: "p".into(),
            modifiers: Modifiers::cmd_shift(),
            action: Action::OpenCommandPalette,
        },
        // Editing
        KeyBinding {
            key: "z".into(),
            modifiers: Modifiers::cmd(),
            action: Action::Undo,
        },
        KeyBinding {
            key: "z".into(),
            modifiers: Modifiers::cmd_shift(),
            action: Action::Redo,
        },
        // Emacs bindings
        KeyBinding {
            key: "f".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMoveForward,
        },
        KeyBinding {
            key: "b".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMoveBackward,
        },
        KeyBinding {
            key: "p".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMovePrevLine,
        },
        KeyBinding {
            key: "n".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMoveNextLine,
        },
        KeyBinding {
            key: "a".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMoveBeginningOfLine,
        },
        KeyBinding {
            key: "e".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsMoveEndOfLine,
        },
        KeyBinding {
            key: "k".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsKillLine,
        },
        KeyBinding {
            key: "h".into(),
            modifiers: Modifiers::ctrl(),
            action: Action::EmacsDeleteBackward,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_not_empty() {
        let bindings = default_bindings();
        assert!(!bindings.is_empty());
    }

    #[test]
    fn all_actions_covered() {
        let bindings = default_bindings();
        let actions: Vec<_> = bindings.iter().map(|b| &b.action).collect();

        assert!(actions.contains(&&Action::NewNote));
        assert!(actions.contains(&&Action::Save));
        assert!(actions.contains(&&Action::Quit));
        assert!(actions.contains(&&Action::NewTab));
        assert!(actions.contains(&&Action::CloseTab));
        assert!(actions.contains(&&Action::RestoreTab));
        assert!(actions.contains(&&Action::Undo));
        assert!(actions.contains(&&Action::Redo));
        assert!(actions.contains(&&Action::OpenFileFinder));
        assert!(actions.contains(&&Action::OpenContentSearch));
        assert!(actions.contains(&&Action::OpenCommandPalette));
        assert!(actions.contains(&&Action::SplitRight));
        assert!(actions.contains(&&Action::SplitDown));
    }

    #[test]
    fn bindings_have_valid_keys_and_modifiers() {
        let bindings = default_bindings();
        for binding in &bindings {
            assert!(!binding.key.is_empty(), "Key should not be empty for {:?}", binding.action);
            // At least one modifier should be set (all bindings use cmd or ctrl)
            let mods = &binding.modifiers;
            assert!(
                mods.cmd || mods.ctrl || mods.shift || mods.alt,
                "At least one modifier expected for {:?}",
                binding.action
            );
        }
    }
}
