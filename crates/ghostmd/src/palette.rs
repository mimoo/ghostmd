use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// A single command that can appear in the command palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    /// Display label shown in the palette.
    pub label: String,
    /// Optional keyboard shortcut hint (e.g. "Cmd+S").
    pub shortcut_hint: Option<String>,
    /// Identifier used to dispatch this command (e.g. "new_note", "save").
    pub action_id: String,
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

    /// Returns the commands that match the current query via fuzzy matching, sorted by score.
    pub fn filtered_commands(&self) -> Vec<&PaletteCommand> {
        if self.query.is_empty() {
            return self.commands.iter().collect();
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::new(
            &self.query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        let mut scored: Vec<(u32, usize)> = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                let mut buf = Vec::new();
                let haystack = Utf32Str::new(&cmd.label, &mut buf);
                pattern
                    .score(haystack, &mut matcher)
                    .map(|score| (score, i))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.iter().map(|&(_, i)| &self.commands[i]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_commands() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand {
                label: "New Note".into(),
                shortcut_hint: Some("Cmd+N".into()),
                action_id: "new_note".into(),
            },
            PaletteCommand {
                label: "Save".into(),
                shortcut_hint: Some("Cmd+S".into()),
                action_id: "save".into(),
            },
            PaletteCommand {
                label: "Split Right".into(),
                shortcut_hint: None,
                action_id: "split_right".into(),
            },
            PaletteCommand {
                label: "Theme: Tokyo Night".into(),
                shortcut_hint: None,
                action_id: "theme_tokyo_night".into(),
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
        assert_eq!(palette.filtered_commands().len(), 4);
    }

    #[test]
    fn fuzzy_match_works() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "nwnot".into();
        let results = palette.filtered_commands();
        assert!(!results.is_empty());
        assert_eq!(results[0].label, "New Note");
    }

    #[test]
    fn fuzzy_match_abbreviation() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "sr".into();
        let results = palette.filtered_commands();
        assert!(!results.is_empty());
        assert_eq!(results[0].label, "Split Right");
    }

    #[test]
    fn fuzzy_match_theme() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "tkn".into();
        let results = palette.filtered_commands();
        assert!(!results.is_empty());
        assert_eq!(results[0].label, "Theme: Tokyo Night");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut palette = CommandPalette::new(sample_commands());
        palette.query = "SAVE".into();
        let results = palette.filtered_commands();
        assert_eq!(results.len(), 1);
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
        assert_eq!(commands[0].shortcut_hint, Some("Cmd+N".to_string()));
        assert_eq!(commands[1].shortcut_hint, Some("Cmd+S".to_string()));
        assert!(commands[2].shortcut_hint.is_none());
    }
}
