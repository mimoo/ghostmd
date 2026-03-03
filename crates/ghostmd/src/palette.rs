/// A single command that can appear in the command palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    /// Display label shown in the palette.
    pub label: String,
    /// Optional keyboard shortcut hint (e.g. "Cmd+S").
    pub shortcut_hint: Option<String>,
    /// The action to dispatch when this command is selected.
    #[allow(dead_code)]
    pub action: crate::keybindings::Action,
}

/// State for the command palette overlay.
pub struct CommandPalette {
    /// Whether the palette is currently visible.
    pub visible: bool,
    /// The current filter query typed by the user.
    pub query: String,
    /// All available commands.
    pub commands: Vec<PaletteCommand>,
    /// Index of the currently highlighted command.
    pub selected_index: usize,
}

impl CommandPalette {
    /// Creates a new command palette with the given commands.
    pub fn new(commands: Vec<PaletteCommand>) -> Self {
        CommandPalette {
            visible: false,
            query: String::new(),
            commands,
            selected_index: 0,
        }
    }

    /// Opens the palette and resets the query.
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Closes the palette.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Returns the commands that match the current query (case-insensitive substring).
    pub fn filtered_commands(&self) -> Vec<&PaletteCommand> {
        if self.query.is_empty() {
            return self.commands.iter().collect();
        }
        let query_lower = self.query.to_lowercase();
        self.commands
            .iter()
            .filter(|cmd| cmd.label.to_lowercase().contains(&query_lower))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::Action;

    fn sample_commands() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand {
                label: "New Note".into(),
                shortcut_hint: Some("Cmd+N".into()),
                action: Action::NewNote,
            },
            PaletteCommand {
                label: "Save".into(),
                shortcut_hint: Some("Cmd+S".into()),
                action: Action::Save,
            },
            PaletteCommand {
                label: "Split Right".into(),
                shortcut_hint: None,
                action: Action::SplitRight,
            },
        ]
    }

    #[test]
    fn open_resets_state() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "old query".into();
        palette.selected_index = 5;
        palette.open();
        assert!(palette.visible);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected_index, 0);
    }

    #[test]
    fn close_hides_palette() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.open();
        palette.close();
        assert!(!palette.visible);
    }

    #[test]
    fn empty_query_returns_all() {
        let palette = CommandPalette::new(sample_commands());
        assert_eq!(palette.filtered_commands().len(), 3);
    }

    #[test]
    fn filter_narrows_results() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "note".into();
        let results = palette.filtered_commands();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "New Note");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "SAVE".into();
        let results = palette.filtered_commands();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_case_insensitive_mixed_case() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "nEw NoTe".into();
        let results = palette.filtered_commands();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "New Note");
    }

    #[test]
    fn empty_filter_returns_all_commands() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = String::new();
        let results = palette.filtered_commands();
        assert_eq!(results.len(), 3);
        // Order should be preserved
        assert_eq!(results[0].label, "New Note");
        assert_eq!(results[1].label, "Save");
        assert_eq!(results[2].label, "Split Right");
    }

    #[test]
    fn no_commands_match_returns_empty() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "zzz_nonexistent_command".into();
        let results = palette.filtered_commands();
        assert!(results.is_empty());
    }

    #[test]
    fn commands_include_keybinding_display() {
        let commands = sample_commands();
        // "New Note" has a shortcut hint
        let new_note = &commands[0];
        assert_eq!(new_note.shortcut_hint, Some("Cmd+N".to_string()));

        // "Save" has a shortcut hint
        let save = &commands[1];
        assert_eq!(save.shortcut_hint, Some("Cmd+S".to_string()));

        // "Split Right" has no shortcut hint
        let split = &commands[2];
        assert!(split.shortcut_hint.is_none());
    }
}
