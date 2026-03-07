use std::collections::HashMap;

use super::*;

pub(crate) fn random_note_name() -> String {
    const ADJECTIVES: &[&str] = &[
        "amber", "azure", "bleak", "bold", "brisk", "bright", "broad", "calm",
        "clear", "cool", "crisp", "dark", "deep", "dense", "dim", "dry",
        "eager", "even", "faint", "fair", "fast", "fine", "firm", "flat",
        "fleet", "fond", "free", "fresh", "full", "gentle", "glad", "gold",
        "grand", "gray", "green", "grim", "hale", "harsh", "hazy", "high",
        "idle", "keen", "kind", "last", "lean", "light", "live", "lone",
        "long", "lost", "loud", "lucid", "main", "meek", "mellow", "mild",
        "mint", "moody", "mute", "neat", "new", "next", "noble", "odd",
        "old", "open", "pale", "plain", "plum", "pure", "quick", "quiet",
        "rapid", "rare", "raw", "real", "rich", "ripe", "rosy", "rough",
        "round", "rust", "sage", "sharp", "sheer", "short", "shy", "silent",
        "slim", "slow", "small", "smooth", "snug", "soft", "solid", "spare",
        "stark", "steep", "still", "stout", "stray", "strong", "sure", "sweet",
        "swift", "tall", "tame", "taut", "thick", "thin", "tidy", "tiny",
        "true", "twin", "vast", "vivid", "warm", "weak", "whole", "wide",
        "wild", "wise", "worn", "young", "zeal",
    ];
    const NOUNS: &[&str] = &[
        "arch", "aspen", "birch", "blade", "blaze", "bloom", "bolt", "bone",
        "brook", "cairn", "cedar", "chalk", "chime", "clay", "cliff", "cloud",
        "coast", "coral", "cove", "crane", "creek", "crest", "crown", "dawn",
        "delta", "dew", "dove", "drift", "dune", "dusk", "echo", "edge",
        "elm", "ember", "fern", "field", "finch", "fjord", "flame", "flare",
        "flint", "frost", "gale", "gate", "gem", "glade", "glen", "glow",
        "gorge", "grain", "grove", "gust", "hare", "haven", "hawk", "hazel",
        "heath", "heron", "hill", "hive", "hollow", "horn", "inlet", "iris",
        "isle", "ivy", "jade", "knoll", "lake", "lark", "leaf", "ledge",
        "lily", "loch", "marsh", "meadow", "mist", "moon", "moss", "north",
        "oak", "opal", "orbit", "otter", "pass", "path", "peak", "pearl",
        "pier", "pine", "plume", "point", "pond", "quartz", "rain", "range",
        "reef", "ridge", "river", "robin", "root", "rover", "sage", "sand",
        "shade", "shell", "shore", "slate", "slope", "snow", "spark", "spire",
        "spray", "spring", "spur", "star", "stem", "stone", "storm", "stream",
        "summit", "thorn", "tide", "timber", "tower", "trail", "vale", "vine",
        "wave", "weald", "wheat", "wind", "wing", "wren", "yarrow",
    ];
    // SAFETY: arc4random is always available on macOS, returns a uniform u32
    let r1 = unsafe { libc::arc4random() } as usize;
    let r2 = unsafe { libc::arc4random() } as usize;
    let adj = ADJECTIVES[r1 % ADJECTIVES.len()];
    let noun = NOUNS[r2 % NOUNS.len()];
    format!("{}-{}", adj, noun)
}

/// Pick a name for a new note in `dir`: "notes" if it doesn't exist yet,
/// otherwise a random adjective-noun name.
pub(crate) fn pick_note_name(dir: &std::path::Path) -> String {
    if !dir.join("notes.md").exists() {
        "notes".into()
    } else {
        random_note_name()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SplitDirection {
    Vertical,   // side-by-side (cmd-d)
    Horizontal, // top/bottom  (cmd-shift-d)
}

#[derive(Clone)]
pub(crate) enum SplitNode {
    Leaf(usize),
    Split {
        direction: SplitDirection,
        left: Box<SplitNode>,
        right: Box<SplitNode>,
    },
}

impl SplitNode {
    /// Collect all leaf pane IDs in left-to-right / top-to-bottom order.
    pub(crate) fn leaves(&self) -> Vec<usize> {
        match self {
            SplitNode::Leaf(id) => vec![*id],
            SplitNode::Split { left, right, .. } => {
                let mut v = left.leaves();
                v.extend(right.leaves());
                v
            }
        }
    }

    /// Replace the leaf with `pane_id` by a split containing `pane_id` and `new_id`.
    pub(crate) fn split_leaf(&mut self, pane_id: usize, new_id: usize, direction: SplitDirection) {
        match self {
            SplitNode::Leaf(id) if *id == pane_id => {
                *self = SplitNode::Split {
                    direction,
                    left: Box::new(SplitNode::Leaf(pane_id)),
                    right: Box::new(SplitNode::Leaf(new_id)),
                };
            }
            SplitNode::Split { left, right, .. } => {
                left.split_leaf(pane_id, new_id, direction);
                right.split_leaf(pane_id, new_id, direction);
            }
            _ => {}
        }
    }

    /// Whether this subtree contains the given pane_id.
    pub(crate) fn contains(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(id) => *id == pane_id,
            SplitNode::Split { left, right, .. } => {
                left.contains(pane_id) || right.contains(pane_id)
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn leftmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { left, .. } => left.leftmost_leaf(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn rightmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.rightmost_leaf(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn topmost_leaf(&self) -> usize {
        self.leftmost_leaf()
    }

    #[allow(dead_code)]
    pub(crate) fn bottommost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.bottommost_leaf(),
        }
    }

    // -----------------------------------------------------------------------
    // Position-aware navigation helpers
    // -----------------------------------------------------------------------

    /// Check if pane is in the top half of this subtree.
    fn is_in_top_half(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => true,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    left.contains(pane_id)
                } else if left.contains(pane_id) {
                    left.is_in_top_half(pane_id)
                } else {
                    right.is_in_top_half(pane_id)
                }
            }
        }
    }

    /// Check if pane is in the left half of this subtree.
    fn is_in_left_half(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => true,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    left.contains(pane_id)
                } else if left.contains(pane_id) {
                    left.is_in_left_half(pane_id)
                } else {
                    right.is_in_left_half(pane_id)
                }
            }
        }
    }

    /// Enter this subtree from the right side, preserving vertical position.
    fn enter_from_right(&self, prefer_top: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    right.enter_from_right(prefer_top)
                } else if prefer_top {
                    left.enter_from_right(true)
                } else {
                    right.enter_from_right(false)
                }
            }
        }
    }

    /// Enter this subtree from the left side, preserving vertical position.
    fn enter_from_left(&self, prefer_top: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    left.enter_from_left(prefer_top)
                } else if prefer_top {
                    left.enter_from_left(true)
                } else {
                    right.enter_from_left(false)
                }
            }
        }
    }

    /// Enter this subtree from below, preserving horizontal position.
    fn enter_from_below(&self, prefer_left: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    right.enter_from_below(prefer_left)
                } else if prefer_left {
                    left.enter_from_below(true)
                } else {
                    right.enter_from_below(false)
                }
            }
        }
    }

    /// Enter this subtree from above, preserving horizontal position.
    fn enter_from_above(&self, prefer_left: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    left.enter_from_above(prefer_left)
                } else if prefer_left {
                    left.enter_from_above(true)
                } else {
                    right.enter_from_above(false)
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Directional navigation (position-aware)
    // -----------------------------------------------------------------------

    /// Find the pane to the right of `from` in a 2D-aware manner.
    pub(crate) fn find_right(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Vertical {
                    if left.contains(from) {
                        if let Some(id) = left.find_right(from) {
                            return Some(id);
                        }
                        let prefer_top = left.is_in_top_half(from);
                        return Some(right.enter_from_left(prefer_top));
                    }
                    right.find_right(from)
                } else if left.contains(from) {
                    left.find_right(from)
                } else {
                    right.find_right(from)
                }
            }
        }
    }

    /// Find the pane to the left of `from` in a 2D-aware manner.
    pub(crate) fn find_left(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Vertical {
                    if right.contains(from) {
                        if let Some(id) = right.find_left(from) {
                            return Some(id);
                        }
                        let prefer_top = right.is_in_top_half(from);
                        return Some(left.enter_from_right(prefer_top));
                    }
                    left.find_left(from)
                } else if left.contains(from) {
                    left.find_left(from)
                } else {
                    right.find_left(from)
                }
            }
        }
    }

    /// Find the pane below `from` in a 2D-aware manner.
    pub(crate) fn find_down(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Horizontal {
                    if left.contains(from) {
                        if let Some(id) = left.find_down(from) {
                            return Some(id);
                        }
                        let prefer_left = left.is_in_left_half(from);
                        return Some(right.enter_from_above(prefer_left));
                    }
                    right.find_down(from)
                } else if left.contains(from) {
                    left.find_down(from)
                } else {
                    right.find_down(from)
                }
            }
        }
    }

    /// Find the pane above `from` in a 2D-aware manner.
    pub(crate) fn find_up(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Horizontal {
                    if right.contains(from) {
                        if let Some(id) = right.find_up(from) {
                            return Some(id);
                        }
                        let prefer_left = right.is_in_left_half(from);
                        return Some(left.enter_from_below(prefer_left));
                    }
                    left.find_up(from)
                } else if left.contains(from) {
                    left.find_up(from)
                } else {
                    right.find_up(from)
                }
            }
        }
    }

    /// A stable ID for this node, derived from the leftmost leaf.
    pub(crate) fn stable_id(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { left, .. } => left.stable_id(),
        }
    }

    /// Remove a leaf by pane_id. Returns true if removed.
    /// When a leaf is removed, its parent split is collapsed to the sibling.
    pub(crate) fn remove_leaf(&mut self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => false,
            SplitNode::Split { left, right, .. } => {
                if let SplitNode::Leaf(id) = **left {
                    if id == pane_id {
                        *self = *right.clone();
                        return true;
                    }
                }
                if let SplitNode::Leaf(id) = **right {
                    if id == pane_id {
                        *self = *left.clone();
                        return true;
                    }
                }
                left.remove_leaf(pane_id) || right.remove_leaf(pane_id)
            }
        }
    }

    /// Convert to serializable session format.
    pub(crate) fn to_session(&self, panes: &HashMap<usize, Pane>) -> SessionSplitNode {
        match self {
            SplitNode::Leaf(id) => {
                let path = panes.get(id)
                    .and_then(|p| p.active_path.as_ref())
                    .map(|p| p.to_string_lossy().to_string());
                SessionSplitNode::Leaf { path }
            }
            SplitNode::Split { direction, left, right } => {
                SessionSplitNode::Split {
                    direction: match direction {
                        SplitDirection::Vertical => "vertical".to_string(),
                        SplitDirection::Horizontal => "horizontal".to_string(),
                    },
                    left: Box::new(left.to_session(panes)),
                    right: Box::new(right.to_session(panes)),
                }
            }
        }
    }
}
