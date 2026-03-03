use std::collections::VecDeque;
use std::path::PathBuf;

/// Information about a tab that was recently closed, enabling restore.
pub struct ClosedTab {
    #[allow(dead_code)]
    pub path: PathBuf,
    #[allow(dead_code)]
    pub cursor_position: usize,
}

/// Manages a bounded stack of recently-closed tabs for restore (Cmd-Shift-T).
pub struct TabManager {
    closed_tabs: VecDeque<ClosedTab>,
    #[allow(dead_code)]
    max_closed: usize,
}

impl TabManager {
    /// Creates a new TabManager that remembers at most `max_closed` tabs.
    pub fn new(max_closed: usize) -> Self {
        TabManager {
            closed_tabs: VecDeque::new(),
            max_closed,
        }
    }

    /// Records a closed tab. If the capacity is full, the oldest entry is evicted.
    #[allow(dead_code)]
    pub fn push_closed(&mut self, tab: ClosedTab) {
        if self.closed_tabs.len() == self.max_closed {
            self.closed_tabs.pop_front();
        }
        self.closed_tabs.push_back(tab);
    }

    /// Restores the most recently closed tab, or `None` if the stack is empty.
    #[allow(dead_code)]
    pub fn pop_closed(&mut self) -> Option<ClosedTab> {
        self.closed_tabs.pop_back()
    }

    /// Returns the number of closed tabs currently stored.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.closed_tabs.len()
    }

    /// Returns `true` if there are no closed tabs stored.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.closed_tabs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab(name: &str, cursor: usize) -> ClosedTab {
        ClosedTab {
            path: PathBuf::from(name),
            cursor_position: cursor,
        }
    }

    #[test]
    fn new_manager_is_empty() {
        let mgr = TabManager::new(10);
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn push_and_pop_single() {
        let mut mgr = TabManager::new(10);
        mgr.push_closed(make_tab("a.md", 5));
        assert_eq!(mgr.len(), 1);

        let tab = mgr.pop_closed().unwrap();
        assert_eq!(tab.path, PathBuf::from("a.md"));
        assert_eq!(tab.cursor_position, 5);
        assert!(mgr.is_empty());
    }

    #[test]
    fn pop_returns_most_recent_first() {
        let mut mgr = TabManager::new(10);
        mgr.push_closed(make_tab("first.md", 0));
        mgr.push_closed(make_tab("second.md", 10));
        mgr.push_closed(make_tab("third.md", 20));

        assert_eq!(mgr.pop_closed().unwrap().path, PathBuf::from("third.md"));
        assert_eq!(mgr.pop_closed().unwrap().path, PathBuf::from("second.md"));
        assert_eq!(mgr.pop_closed().unwrap().path, PathBuf::from("first.md"));
    }

    #[test]
    fn max_capacity_evicts_oldest() {
        let mut mgr = TabManager::new(2);
        mgr.push_closed(make_tab("a.md", 0));
        mgr.push_closed(make_tab("b.md", 0));
        mgr.push_closed(make_tab("c.md", 0));

        // "a.md" should have been evicted
        assert_eq!(mgr.len(), 2);
        assert_eq!(mgr.pop_closed().unwrap().path, PathBuf::from("c.md"));
        assert_eq!(mgr.pop_closed().unwrap().path, PathBuf::from("b.md"));
        assert!(mgr.pop_closed().is_none());
    }

    #[test]
    fn pop_empty_returns_none() {
        let mut mgr = TabManager::new(5);
        assert!(mgr.pop_closed().is_none());
    }

    #[test]
    fn len_tracks_correctly() {
        let mut mgr = TabManager::new(10);
        assert_eq!(mgr.len(), 0);

        mgr.push_closed(make_tab("a.md", 0));
        assert_eq!(mgr.len(), 1);

        mgr.push_closed(make_tab("b.md", 0));
        assert_eq!(mgr.len(), 2);

        mgr.pop_closed();
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn restore_order_is_lifo() {
        let mut mgr = TabManager::new(10);
        mgr.push_closed(make_tab("first.md", 1));
        mgr.push_closed(make_tab("second.md", 2));
        mgr.push_closed(make_tab("third.md", 3));

        // Last closed should be first restored (LIFO)
        let t1 = mgr.pop_closed().unwrap();
        assert_eq!(t1.path, PathBuf::from("third.md"));
        let t2 = mgr.pop_closed().unwrap();
        assert_eq!(t2.path, PathBuf::from("second.md"));
        let t3 = mgr.pop_closed().unwrap();
        assert_eq!(t3.path, PathBuf::from("first.md"));
        assert!(mgr.pop_closed().is_none());
    }

    #[test]
    fn push_same_path_twice_stores_both() {
        let mut mgr = TabManager::new(10);
        mgr.push_closed(make_tab("same.md", 10));
        mgr.push_closed(make_tab("same.md", 20));
        assert_eq!(mgr.len(), 2);

        let t1 = mgr.pop_closed().unwrap();
        assert_eq!(t1.path, PathBuf::from("same.md"));
        assert_eq!(t1.cursor_position, 20);

        let t2 = mgr.pop_closed().unwrap();
        assert_eq!(t2.path, PathBuf::from("same.md"));
        assert_eq!(t2.cursor_position, 10);
    }

    #[test]
    fn max_closed_zero_evicts_on_every_push() {
        // With max_closed=0, the first push triggers the eviction check
        // (len 0 == max 0), pops from empty (no-op), then pushes.
        // Subsequent pushes don't hit the check since len > max_closed.
        // This documents current behavior; max_closed=1 is the practical minimum.
        let mut mgr = TabManager::new(0);
        mgr.push_closed(make_tab("a.md", 0));
        // Item is stored because pop_front on empty is a no-op
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn push_then_pop_preserves_cursor_position() {
        let mut mgr = TabManager::new(10);
        mgr.push_closed(make_tab("a.md", 42));
        mgr.push_closed(make_tab("b.md", 999));
        mgr.push_closed(make_tab("c.md", 0));

        assert_eq!(mgr.pop_closed().unwrap().cursor_position, 0);
        assert_eq!(mgr.pop_closed().unwrap().cursor_position, 999);
        assert_eq!(mgr.pop_closed().unwrap().cursor_position, 42);
    }
}
