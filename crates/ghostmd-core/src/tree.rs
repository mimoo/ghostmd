use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A node in the file tree, representing either a directory or a file.
#[derive(Debug, Clone)]
pub enum TreeNode {
    Directory {
        path: PathBuf,
        name: String,
        children: Vec<TreeNode>,
        expanded: bool,
    },
    File {
        path: PathBuf,
        name: String,
    },
}

impl TreeNode {
    /// Returns the path of this node.
    pub fn path(&self) -> &Path {
        match self {
            TreeNode::Directory { path, .. } => path,
            TreeNode::File { path, .. } => path,
        }
    }

    /// Returns the display name of this node.
    pub fn name(&self) -> &str {
        match self {
            TreeNode::Directory { name, .. } => name,
            TreeNode::File { name, .. } => name,
        }
    }

    /// Returns true if this node is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, TreeNode::Directory { .. })
    }

    /// Returns true if this node is an expanded directory.
    pub fn is_expanded(&self) -> bool {
        match self {
            TreeNode::Directory { expanded, .. } => *expanded,
            TreeNode::File { .. } => false,
        }
    }
}

/// A file tree rooted at a given directory, supporting scan, expand/collapse, and search.
pub struct FileTree {
    pub root: PathBuf,
    pub nodes: Vec<TreeNode>,
}

impl FileTree {
    /// Creates a new, unscanned file tree rooted at the given path.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            nodes: vec![],
        }
    }

    /// Recursively scans the root directory and populates the tree.
    /// Excludes `.ghostmd` directories.
    pub fn scan(&mut self) -> Result<()> {
        // Preserve collapsed state across rescans
        let mut collapsed = HashSet::new();
        collect_collapsed(&self.nodes, &mut collapsed);
        self.nodes = scan_dir(&self.root)?;
        apply_collapsed(&mut self.nodes, &collapsed);
        // Pin "diary" folder to the front at root level
        if let Some(idx) = self.nodes.iter().position(|n| n.name() == "diary") {
            if idx > 0 {
                let diary = self.nodes.remove(idx);
                self.nodes.insert(0, diary);
            }
        }
        Ok(())
    }

    /// Toggles the expanded state of a directory node at the given path.
    /// Returns `true` if the node was found and toggled.
    pub fn toggle_dir(&mut self, path: &Path) -> bool {
        toggle_in_nodes(&mut self.nodes, path)
    }

    /// Finds a node by its path, returning a reference if found.
    pub fn find_node(&self, path: &Path) -> Option<&TreeNode> {
        find_in_nodes(&self.nodes, path)
    }

    /// Flattens the tree into a list of `(depth, node)` pairs for display,
    /// only including children of expanded directories.
    pub fn flatten(&self) -> Vec<(usize, &TreeNode)> {
        let mut result = Vec::new();
        flatten_nodes(&self.nodes, 0, &mut result);
        result
    }

    /// Returns the total number of file nodes in the tree.
    pub fn file_count(&self) -> usize {
        count_files(&self.nodes)
    }

    /// Expand ancestor directories on the path to `target` (without collapsing others).
    pub fn reveal_path(&mut self, target: &Path) {
        expand_ancestors(&mut self.nodes, target);
    }

    /// Returns the set of collapsed directory paths.
    pub fn collapsed_paths(&self) -> HashSet<PathBuf> {
        let mut out = HashSet::new();
        collect_collapsed(&self.nodes, &mut out);
        out
    }

    /// Collapse specific directories by path.
    pub fn set_collapsed(&mut self, collapsed: &HashSet<PathBuf>) {
        apply_collapsed(&mut self.nodes, collapsed);
    }

    /// Collapse all directories.
    pub fn collapse_all(&mut self) {
        set_all_expanded(&mut self.nodes, false);
    }

    /// Expand all directories.
    pub fn expand_all(&mut self) {
        set_all_expanded(&mut self.nodes, true);
    }
}

fn scan_dir(dir: &Path) -> Result<Vec<TreeNode>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if entry.file_type()?.is_dir() {
            // Exclude .ghostmd and hidden directories (starting with .)
            if name == ".ghostmd" || name.starts_with('.') {
                continue;
            }
            let children = scan_dir(&path)?;
            dirs.push(TreeNode::Directory {
                path,
                name,
                children,
                expanded: true,
            });
        } else {
            // Skip hidden files (e.g. .DS_Store)
            if name.starts_with('.') {
                continue;
            }
            files.push(TreeNode::File { path, name });
        }
    }

    // Sort directories alphabetically, then files alphabetically
    dirs.sort_by(|a, b| node_name(a).cmp(node_name(b)));
    files.sort_by(|a, b| node_name(a).cmp(node_name(b)));

    dirs.extend(files);
    Ok(dirs)
}

/// Collect paths of all collapsed directories.
fn collect_collapsed(nodes: &[TreeNode], out: &mut HashSet<PathBuf>) {
    for node in nodes {
        if let TreeNode::Directory { path, expanded, children, .. } = node {
            if !expanded {
                out.insert(path.clone());
            }
            collect_collapsed(children, out);
        }
    }
}

/// Re-collapse directories that were collapsed before a rescan.
fn apply_collapsed(nodes: &mut [TreeNode], collapsed: &HashSet<PathBuf>) {
    for node in nodes.iter_mut() {
        if let TreeNode::Directory { path, expanded, children, .. } = node {
            if collapsed.contains(path) {
                *expanded = false;
            }
            apply_collapsed(children, collapsed);
        }
    }
}

fn node_name(node: &TreeNode) -> &str {
    match node {
        TreeNode::Directory { name, .. } => name,
        TreeNode::File { name, .. } => name,
    }
}

fn toggle_in_nodes(nodes: &mut [TreeNode], path: &Path) -> bool {
    for node in nodes.iter_mut() {
        match node {
            TreeNode::Directory {
                path: ref node_path,
                ref mut expanded,
                ref mut children,
                ..
            } => {
                if node_path == path {
                    *expanded = !*expanded;
                    return true;
                }
                if toggle_in_nodes(children, path) {
                    return true;
                }
            }
            TreeNode::File { .. } => {}
        }
    }
    false
}

fn find_in_nodes<'a>(nodes: &'a [TreeNode], path: &Path) -> Option<&'a TreeNode> {
    for node in nodes {
        match node {
            TreeNode::Directory {
                path: ref node_path,
                ref children,
                ..
            } => {
                if node_path == path {
                    return Some(node);
                }
                if let Some(found) = find_in_nodes(children, path) {
                    return Some(found);
                }
            }
            TreeNode::File {
                path: ref node_path,
                ..
            } => {
                if node_path == path {
                    return Some(node);
                }
            }
        }
    }
    None
}

fn flatten_nodes<'a>(nodes: &'a [TreeNode], depth: usize, result: &mut Vec<(usize, &'a TreeNode)>) {
    for node in nodes {
        result.push((depth, node));
        if let TreeNode::Directory {
            expanded: true,
            ref children,
            ..
        } = node
        {
            flatten_nodes(children, depth + 1, result);
        }
    }
}

fn expand_ancestors(nodes: &mut [TreeNode], target: &Path) {
    for node in nodes.iter_mut() {
        if let TreeNode::Directory { ref path, ref mut children, ref mut expanded, .. } = node {
            if target.starts_with(path.as_path()) && target != path.as_path() {
                *expanded = true;
                expand_ancestors(children, target);
            }
        }
    }
}

fn set_all_expanded(nodes: &mut [TreeNode], value: bool) {
    for node in nodes.iter_mut() {
        if let TreeNode::Directory { expanded, children, .. } = node {
            *expanded = value;
            set_all_expanded(children, value);
        }
    }
}

fn count_files(nodes: &[TreeNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        match node {
            TreeNode::File { .. } => count += 1,
            TreeNode::Directory { children, .. } => count += count_files(children),
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure(tmp: &TempDir) {
        let root = tmp.path();
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::create_dir_all(root.join("diary/2024")).unwrap();
        fs::write(root.join("notes/note1.md"), "note1").unwrap();
        fs::write(root.join("notes/note2.md"), "note2").unwrap();
        fs::write(root.join("diary/2024/jan.md"), "jan").unwrap();
        fs::write(root.join("readme.md"), "readme").unwrap();
    }

    #[test]
    fn scan_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();
        assert_eq!(tree.file_count(), 0);
        assert!(tree.nodes.is_empty());
    }

    #[test]
    fn scan_nested_structure() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        // Should have 4 files total
        assert_eq!(tree.file_count(), 4);
    }

    #[test]
    fn toggle_expands_and_collapses() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        let notes_path = tmp.path().join("notes");

        // Initially directories should not be expanded (or expanded depending on scan impl)
        // Toggle once
        assert!(tree.toggle_dir(&notes_path));
        let node = tree.find_node(&notes_path).unwrap();
        if let TreeNode::Directory { expanded, .. } = node {
            let state1 = *expanded;
            // Toggle again - should flip
            assert!(tree.toggle_dir(&notes_path));
            let node2 = tree.find_node(&notes_path).unwrap();
            if let TreeNode::Directory { expanded, .. } = node2 {
                assert_ne!(state1, *expanded);
            } else {
                panic!("expected directory");
            }
        } else {
            panic!("expected directory");
        }
    }

    #[test]
    fn find_existing_node() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        let note_path = tmp.path().join("notes/note1.md");
        assert!(tree.find_node(&note_path).is_some());
    }

    #[test]
    fn find_missing_node() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        let missing_path = tmp.path().join("nonexistent.md");
        assert!(tree.find_node(&missing_path).is_none());
    }

    #[test]
    fn flatten_returns_correct_order_and_depths() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.md"), "a").unwrap();
        fs::write(root.join("sub/b.md"), "b").unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();

        let flat = tree.flatten();
        // Should have entries at depth 0 and potentially depth 1
        assert!(!flat.is_empty());
        // Top-level items should be at depth 0
        assert_eq!(flat[0].0, 0);
    }

    #[test]
    fn ghostmd_directory_excluded() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join(".ghostmd")).unwrap();
        fs::write(root.join(".ghostmd/config.json"), "{}").unwrap();
        fs::write(root.join("visible.md"), "hi").unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();

        // Should only see visible.md, not .ghostmd contents
        assert_eq!(tree.file_count(), 1);
    }

    #[test]
    fn file_count_accuracy() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "").unwrap();
        fs::write(root.join("b.md"), "").unwrap();
        fs::write(root.join("c.txt"), "").unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();
        assert_eq!(tree.file_count(), 3);
    }

    #[test]
    fn toggle_nonexistent_dir_returns_false() {
        let tmp = TempDir::new().unwrap();
        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        assert!(!tree.toggle_dir(Path::new("/nonexistent")));
    }

    #[test]
    fn directory_with_only_subdirectories_no_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("subdir_a")).unwrap();
        fs::create_dir_all(root.join("subdir_b/nested")).unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();
        assert_eq!(tree.file_count(), 0);
    }

    #[test]
    fn hidden_dot_prefixed_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".hidden.md"), "secret").unwrap();
        fs::write(root.join("visible.md"), "public").unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();

        // Whether hidden files are included depends on implementation.
        // At minimum, visible.md should be present.
        assert!(tree.find_node(&root.join("visible.md")).is_some());
    }

    #[test]
    fn scan_single_file_at_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("only.md"), "solo").unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();

        assert_eq!(tree.file_count(), 1);
        assert!(tree.find_node(&root.join("only.md")).is_some());
    }

    #[test]
    fn reveal_path_expands_ancestors_without_collapsing_others() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        // Collapse diary first
        tree.toggle_dir(&tmp.path().join("diary"));
        let diary = tree.find_node(&tmp.path().join("diary")).unwrap();
        assert!(!diary.is_expanded(), "diary should be collapsed after toggle");

        let target = tmp.path().join("diary/2024/jan.md");
        tree.reveal_path(&target);

        // diary should be expanded (ancestor)
        let diary = tree.find_node(&tmp.path().join("diary")).unwrap();
        assert!(diary.is_expanded(), "diary should be expanded");

        // notes should still be expanded (not touched)
        let notes = tree.find_node(&tmp.path().join("notes")).unwrap();
        assert!(notes.is_expanded(), "notes should still be expanded");
    }

    #[test]
    fn collapse_all_and_expand_all() {
        let tmp = TempDir::new().unwrap();
        create_test_structure(&tmp);

        let mut tree = FileTree::new(tmp.path().to_path_buf());
        tree.scan().unwrap();

        tree.collapse_all();
        let flat = tree.flatten();
        // Only top-level nodes visible
        assert!(flat.iter().all(|(depth, _)| *depth == 0));

        tree.expand_all();
        let flat = tree.flatten();
        // Should have deeper nodes now
        assert!(flat.iter().any(|(depth, _)| *depth > 0));
    }

    #[test]
    fn file_count_zero_for_empty_dir_with_subdirs_only() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("empty_a")).unwrap();
        fs::create_dir_all(root.join("empty_b")).unwrap();
        fs::create_dir_all(root.join("empty_c/deep")).unwrap();

        let mut tree = FileTree::new(root.to_path_buf());
        tree.scan().unwrap();
        assert_eq!(tree.file_count(), 0);
    }
}
