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
    JumpToTab(u8), // 1-9

    // Splits
    SplitRight,
    SplitDown,
    FocusSplitLeft,
    FocusSplitRight,
    FocusSplitUp,
    FocusSplitDown,

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

/// A keyboard modifier set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifiers {
    pub cmd: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

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

/// A key binding mapping a key + modifiers to an action.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: String,
    pub modifiers: Modifiers,
    pub action: Action,
}

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
}
