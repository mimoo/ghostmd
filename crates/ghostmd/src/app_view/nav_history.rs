use std::path::PathBuf;

const MAX_ENTRIES: usize = 100;
/// Skip push if cursor is within this many bytes of the current entry.
const DEDUP_THRESHOLD: usize = 50;

#[derive(Debug, Clone)]
pub(crate) struct NavEntry {
    pub workspace_id: usize,
    pub pane_id: usize,
    pub path: PathBuf,
    pub cursor_offset: usize,
}

/// A linear history stack with a movable index for back/forward navigation.
pub(crate) struct NavHistory {
    entries: Vec<NavEntry>,
    /// Points to the "current" entry. When navigating back, index decreases.
    /// When the stack is empty, index is 0 and entries is empty.
    index: usize,
}

impl NavHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: 0,
        }
    }

    /// Push a new navigation entry. Truncates any forward history beyond the
    /// current index, then appends.
    pub fn push(&mut self, entry: NavEntry) {
        // Dedup: skip if current entry matches
        if !self.entries.is_empty() && self.index < self.entries.len() {
            let current = &self.entries[self.index];
            if current.workspace_id == entry.workspace_id
                && current.pane_id == entry.pane_id
                && current.path == entry.path
                && current.cursor_offset.abs_diff(entry.cursor_offset) < DEDUP_THRESHOLD
            {
                return;
            }
        }

        // Truncate forward history
        if !self.entries.is_empty() {
            self.entries.truncate(self.index + 1);
        }

        self.entries.push(entry);
        self.index = self.entries.len() - 1;

        // Cap size
        if self.entries.len() > MAX_ENTRIES {
            let excess = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..excess);
            self.index = self.index.saturating_sub(excess);
        }
    }

    /// Move back in history. Returns the entry to navigate to, or None if at
    /// the beginning.
    pub fn go_back(&mut self) -> Option<&NavEntry> {
        if self.entries.is_empty() || self.index == 0 {
            return None;
        }
        self.index -= 1;
        Some(&self.entries[self.index])
    }

    /// Move forward in history. Returns the entry to navigate to, or None if
    /// at the end.
    pub fn go_forward(&mut self) -> Option<&NavEntry> {
        if self.entries.is_empty() || self.index >= self.entries.len() - 1 {
            return None;
        }
        self.index += 1;
        Some(&self.entries[self.index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ws: usize, pane: usize, path: &str, offset: usize) -> NavEntry {
        NavEntry {
            workspace_id: ws,
            pane_id: pane,
            path: PathBuf::from(path),
            cursor_offset: offset,
        }
    }

    #[test]
    fn back_and_forward() {
        let mut h = NavHistory::new();
        h.push(entry(0, 0, "a.md", 0));
        h.push(entry(0, 0, "b.md", 0));
        h.push(entry(0, 0, "c.md", 0));

        let e = h.go_back().unwrap();
        assert_eq!(e.path, PathBuf::from("b.md"));

        let e = h.go_back().unwrap();
        assert_eq!(e.path, PathBuf::from("a.md"));

        assert!(h.go_back().is_none());

        let e = h.go_forward().unwrap();
        assert_eq!(e.path, PathBuf::from("b.md"));
    }

    #[test]
    fn push_truncates_forward() {
        let mut h = NavHistory::new();
        h.push(entry(0, 0, "a.md", 0));
        h.push(entry(0, 0, "b.md", 0));
        h.push(entry(0, 0, "c.md", 0));
        h.go_back(); // -> b
        h.go_back(); // -> a

        // Push new entry from "a" position — forward history (b, c) is truncated
        h.push(entry(0, 0, "d.md", 0));

        assert!(h.go_forward().is_none());
        let e = h.go_back().unwrap();
        assert_eq!(e.path, PathBuf::from("a.md"));
    }

    #[test]
    fn dedup_same_location() {
        let mut h = NavHistory::new();
        h.push(entry(0, 0, "a.md", 100));
        h.push(entry(0, 0, "a.md", 120)); // within threshold, should be skipped
        assert!(h.go_back().is_none()); // only one entry
    }

    #[test]
    fn dedup_different_offset() {
        let mut h = NavHistory::new();
        h.push(entry(0, 0, "a.md", 100));
        h.push(entry(0, 0, "a.md", 200)); // beyond threshold, should be kept
        let e = h.go_back().unwrap();
        assert_eq!(e.cursor_offset, 100);
    }

    #[test]
    fn max_entries_cap() {
        let mut h = NavHistory::new();
        for i in 0..150 {
            h.push(entry(0, 0, &format!("{i}.md"), 0));
        }
        assert_eq!(h.entries.len(), MAX_ENTRIES);
    }
}
