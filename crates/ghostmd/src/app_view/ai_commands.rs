use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use gpui::*;

use super::*;

static AI_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn ai_temp_path(prefix: &str) -> std::path::PathBuf {
    let id = AI_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("ghostmd-{}-{}-{}.json", prefix, std::process::id(), id))
}

/// Resolve the full path to the `claude` CLI binary.
/// macOS .app bundles don't inherit the user's shell PATH, so we check common locations.
fn claude_binary() -> &'static str {
    static RESOLVED: OnceLock<String> = OnceLock::new();
    RESOLVED.get_or_init(|| {
        let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
        let candidates = [
            home.as_ref().map(|h| h.join(".local/bin/claude")),
            home.as_ref().map(|h| h.join(".claude/local/claude")),
            Some(std::path::PathBuf::from("/usr/local/bin/claude")),
            Some(std::path::PathBuf::from("/opt/homebrew/bin/claude")),
        ];
        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        "claude".to_string()
    })
}

impl GhostAppView {
    /// AI: rename the active workspace tab based on note content.
    pub(crate) fn ai_rename_tab(&mut self, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() { return; }
        let ws = self.active_ws();
        let mut snippets = Vec::new();
        for pane in ws.panes.values() {
            if let Some(editor) = &pane.editor {
                let text: String = editor.read(cx).text(cx).chars().take(300).collect();
                if !text.trim().is_empty() {
                    snippets.push(text);
                }
            }
        }
        if snippets.is_empty() { return; }

        let ws_id = ws.id;
        self.ai_loading.insert(ws_id);
        self.start_ai_animation(cx);
        let output_path = ai_temp_path("tab");
        let output_path_str = output_path.display().to_string();

        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let op = output_path.clone();
            let result = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "Based on this note content from a workspace:\n{}\n\n\
                     Suggest a short (2-4 words) tab name.\n\
                     Write a JSON file to {} with this exact format:\n\
                     {{\"title\": \"Your Suggested Name\"}}\n\
                     Write ONLY valid JSON, nothing else.",
                    snippets.join("\n---\n"),
                    op.display()
                );
                let status = std::process::Command::new(claude_binary())
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            this.update(cx, |this, cx| {
                this.ai_loading.remove(&ws_id);
                if result {
                    if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                        let _ = std::fs::remove_file(&output_path);
                        #[derive(serde::Deserialize)]
                        struct AiTab { title: String }
                        if let Ok(parsed) = serde_json::from_str::<AiTab>(&json_str) {
                            if let Some(ws) = this.workspaces.iter_mut().find(|w| w.id == ws_id) {
                                ws.title = parsed.title;
                            }
                        }
                    }
                }
                cx.notify();
            }).ok();
            let _ = std::fs::remove_file(&output_path_str);
        }).detach();
    }

    /// AI: rename all workspace tabs in a single Claude call.
    pub(crate) fn ai_rename_all_tabs(&mut self, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() { return; }

        let descriptions: Vec<String> = self.workspaces.iter().enumerate()
            .map(|(i, ws)| {
                let snippets: Vec<String> = ws.panes.values()
                    .filter_map(|p| p.editor.as_ref())
                    .map(|e| e.read(cx).text(cx).chars().take(200).collect::<String>())
                    .filter(|t| !t.trim().is_empty())
                    .collect();
                if snippets.is_empty() {
                    format!("Tab {}: (empty)", i + 1)
                } else {
                    format!("Tab {}: {}", i + 1, snippets.join(" | "))
                }
            })
            .collect();

        let count = self.workspaces.len();
        let ws_ids: Vec<usize> = self.workspaces.iter().map(|w| w.id).collect();
        for &id in &ws_ids { self.ai_loading.insert(id); }
        self.start_ai_animation(cx);
        let output_path = ai_temp_path("tabs");
        let output_path_str = output_path.display().to_string();

        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let op = output_path.clone();
            let result = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "I have {} tabs in a note-taking app. Here is the content of each tab:\n{}\n\n\
                     For each tab, suggest a short (2-4 words) descriptive name.\n\
                     Write a JSON file to {} with this exact format:\n\
                     {{\"titles\": [\"Name 1\", \"Name 2\", ...]}}\n\
                     Write ONLY valid JSON with exactly {} entries, nothing else.",
                    count,
                    descriptions.join("\n"),
                    op.display(),
                    count
                );
                let status = std::process::Command::new(claude_binary())
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            this.update(cx, |this, cx| {
                for &id in &ws_ids { this.ai_loading.remove(&id); }
                if result {
                    if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                        let _ = std::fs::remove_file(&output_path);
                        #[derive(serde::Deserialize)]
                        struct AiTabs { titles: Vec<String> }
                        if let Ok(parsed) = serde_json::from_str::<AiTabs>(&json_str) {
                            for (i, name) in parsed.titles.iter().enumerate() {
                                if i < ws_ids.len() {
                                    if let Some(ws) = this.workspaces.iter_mut().find(|w| w.id == ws_ids[i]) {
                                        ws.title = name.clone();
                                    }
                                }
                            }
                        }
                    }
                }
                cx.notify();
            }).ok();
            let _ = std::fs::remove_file(&output_path_str);
        }).detach();
    }

    /// AI: rename the current file based on its content.
    pub(crate) fn ai_rename_file(&mut self, cx: &mut Context<Self>) {
        let path = match self.focused_active_path() {
            Some(p) => p,
            None => return,
        };
        let editor = {
            let ws = self.active_ws();
            ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
        };
        let content: String = match &editor {
            Some(e) => e.read(cx).text(cx).chars().take(500).collect(),
            None => return,
        };
        if content.trim().is_empty() { return; }

        let ext = path.extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_else(|| ".md".to_string());

        let ws_id = self.active_ws().id;
        self.ai_loading.insert(ws_id);
        self.start_ai_animation(cx);

        let output_path = ai_temp_path("file");
        let output_path_str = output_path.display().to_string();

        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let op = output_path.clone();
            let result = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "Based on this note content, suggest a short descriptive filename \
                     (kebab-case, max 4 words, no extension):\n\n{}\n\n\
                     Write a JSON file to {} with this exact format:\n\
                     {{\"filename\": \"suggested-name\"}}\n\
                     Write ONLY valid JSON, nothing else.",
                    content,
                    op.display()
                );
                let status = std::process::Command::new(claude_binary())
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            this.update(cx, |this, cx| {
                this.ai_loading.remove(&ws_id);
                if result {
                    if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                        let _ = std::fs::remove_file(&output_path);
                        #[derive(serde::Deserialize)]
                        struct AiFile { filename: String }
                        if let Ok(parsed) = serde_json::from_str::<AiFile>(&json_str) {
                            let suggested = parsed.filename;
                            if !suggested.is_empty() {
                                let old_path = path.clone();
                                let parent = path.parent().unwrap_or(&this.root).to_path_buf();
                                let new_path = ghostmd_core::path_utils::unique_path(
                                    &parent.join(format!("{}{}", suggested, ext)),
                                );
                                if new_path != path && !new_path.exists()
                                    && std::fs::rename(&path, &new_path).is_ok()
                                {
                                    this.update_editor_paths(&old_path, &new_path, cx);
                                    this.file_tree.update(cx, |tree, cx| {
                                        tree.refresh(cx);
                                        tree.reveal_file(&new_path, cx);
                                    });
                                    // Show old → new transition in title bar
                                    this.move_transition = Some((old_path, new_path, std::time::Instant::now()));
                                    this.start_ai_animation(cx);
                                    cx.spawn(async |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
                                        cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
                                        this.update(cx, |this, cx| {
                                            this.move_transition = None;
                                            cx.notify();
                                        }).ok();
                                    }).detach();
                                }
                            }
                        }
                    }
                }
                let _ = std::fs::remove_file(&output_path_str);
                cx.notify();
            }).ok();
        }).detach();
    }

    /// AI: suggest a folder for the current file and move it there.
    pub(crate) fn ai_suggest_folder(&mut self, cx: &mut Context<Self>) {
        let source = match self.focused_active_path() {
            Some(p) => p,
            None => return,
        };
        let editor = {
            let ws = self.active_ws();
            ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
        };
        let content: String = match &editor {
            Some(e) => e.read(cx).text(cx).chars().take(500).collect(),
            None => return,
        };
        if content.trim().is_empty() { return; }

        let root = self.root.clone();
        let ws_id = self.active_ws().id;
        self.ai_loading.insert(ws_id);
        self.start_ai_animation(cx);

        // Collect existing folders (relative to root)
        let mut folders = Vec::new();
        fn walk_dirs(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            if let Ok(rel) = path.strip_prefix(root) {
                                out.push(rel.to_string_lossy().to_string());
                            }
                            walk_dirs(&path, root, out);
                        }
                    }
                }
            }
        }
        walk_dirs(&root, &root, &mut folders);

        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let output = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "I have a note with this content:\n{}\n\n\
                     Existing folders: {}\n\n\
                     Suggest the best folder (relative path) to organize this note. \
                     You may suggest a new subfolder if none fit.\n\
                     Reply with ONLY the folder path, nothing else.",
                    content,
                    if folders.is_empty() { "(none)".to_string() } else { folders.join(", ") }
                );
                std::process::Command::new(claude_binary())
                    .arg("-p")
                    .arg("--model").arg("haiku").arg("--effort").arg("low")
                    .arg(&prompt)
                    .output()
            }).await;
            this.update(cx, |this, cx| {
                this.ai_loading.remove(&ws_id);
                if let Ok(out) = output {
                    let suggested = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !suggested.is_empty() {
                        let target_dir = this.root.join(&suggested);
                        // Sanitize: ensure the target stays within the notes root
                        if let Ok(canonical) = target_dir.canonicalize().or_else(|_| {
                            // Dir may not exist yet — check parent
                            std::fs::create_dir_all(&target_dir).ok();
                            target_dir.canonicalize()
                        }) {
                            if canonical.starts_with(&this.root) {
                                let old_path = source.clone();
                                this.move_file_to_dir(source, &canonical, cx);
                                // Set move transition for title bar animation
                                if let Some(new_path) = this.focused_active_path() {
                                    if new_path != old_path {
                                        this.move_transition = Some((old_path, new_path, std::time::Instant::now()));
                                        this.start_ai_animation(cx);
                                        // Schedule fade-out clear after 4 seconds
                                        cx.spawn(async |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
                                            cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
                                            this.update(cx, |this, cx| {
                                                this.move_transition = None;
                                                cx.notify();
                                            }).ok();
                                        }).detach();
                                    }
                                }
                            }
                        }
                    }
                }
                cx.notify();
            }).ok();
        }).detach();
    }

    pub(crate) fn run_agentic_search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let query = self.agentic_input.read(cx).value().to_string().trim().to_string();
        if query.is_empty() {
            return;
        }
        self.agentic_loading = true;
        self.agentic_results.clear();
        self.agentic_selected = 0;
        cx.notify();

        let root = self.root.clone();
        let output_path = ai_temp_path("search");
        let output_path2 = output_path.clone();

        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let result = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "Search through the markdown notes in {root} and answer: {query}\n\n\
                     Output ONLY a JSON array to {output_file} with results ordered by relevance.\n\
                     Each element must have: \"file\" (absolute path), \"line\" (1-based line number), \
                     \"quote\" (the relevant line or short excerpt), \"reason\" (brief explanation).\n\
                     A file can appear multiple times if multiple lines are relevant.\n\
                     If nothing is found, output an empty array [].\n\
                     Do NOT output anything else besides writing the JSON file.",
                    root = root.display(),
                    query = query,
                    output_file = output_path.display(),
                );
                let output = std::process::Command::new(claude_binary())
                    .arg("-p")
                    .arg(&prompt)
                    .arg("--allowedTools")
                    .arg("Read,Glob,Grep,Write,Bash")
                    .current_dir(&root)
                    .output();
                (output, output_path)
            }).await;

            let (output, json_path) = result;
            let matches: Vec<AgenticMatch> = match output {
                Ok(out) if out.status.success() || !out.stdout.is_empty() => {
                    // Try reading the JSON file first
                    if let Ok(json_str) = std::fs::read_to_string(&json_path) {
                        // Clean up temp file
                        std::fs::remove_file(&json_path).ok();
                        serde_json::from_str(&json_str).unwrap_or_default()
                    } else {
                        // Fallback: try parsing JSON from stdout
                        let text = String::from_utf8_lossy(&out.stdout);
                        parse_json_from_text(&text)
                    }
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if !stderr.trim().is_empty() {
                        vec![AgenticMatch {
                            file: String::new(),
                            line: 0,
                            quote: format!("Error: {}", stderr.trim()),
                            reason: String::new(),
                        }]
                    } else {
                        Vec::new()
                    }
                }
                Err(e) => {
                    vec![AgenticMatch {
                        file: String::new(),
                        line: 0,
                        quote: format!("Failed to run claude: {}", e),
                        reason: String::new(),
                    }]
                }
            };

            // Clean up temp file (in case it wasn't cleaned above)
            std::fs::remove_file(&output_path2).ok();

            this.update(cx, |this, cx| {
                this.agentic_results = matches;
                this.agentic_selected = 0;
                this.agentic_loading = false;
                cx.notify();
            }).ok();
        })
        .detach();
    }

    /// Update match count based on current search query and focused editor.
    pub(crate) fn update_search_matches(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string().to_lowercase();
        if query.is_empty() || self.workspaces.is_empty() {
            self.search_match_count = 0;
            cx.notify();
            return;
        }
        let editor = {
            let ws = self.active_ws();
            ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
        };
        if let Some(editor) = editor {
            let text = editor.read(cx).text(cx).to_lowercase();
            self.search_match_count = text.matches(&query).count();
        } else {
            self.search_match_count = 0;
        }
        cx.notify();
    }

    /// Open a file from an agentic search result and scroll to the matched line.
    pub(crate) fn open_agentic_result(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        let m = match self.agentic_results.get(idx) {
            Some(m) if !m.file.is_empty() => m.clone(),
            _ => return,
        };
        self.close_agentic_search(window, cx);

        let path = std::path::PathBuf::from(&m.file);
        if !path.exists() { return; }
        self.open_file(path, window, cx);

        // Scroll to the matched line after the editor is set up
        if m.line > 0 {
            let ws = self.active_ws();
            if let Some(editor) = ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone()) {
                editor.update(cx, |e, cx| {
                    e.scroll_to_line(m.line, window, cx);
                });
            }
        }
    }
}

/// Try to extract a JSON array from text that may contain markdown fences or other wrapping.
fn parse_json_from_text(text: &str) -> Vec<AgenticMatch> {
    let trimmed = text.trim();
    // Direct parse
    if let Ok(v) = serde_json::from_str::<Vec<AgenticMatch>>(trimmed) {
        return v;
    }
    // Try to find JSON array within markdown code fences
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if let Ok(v) = serde_json::from_str::<Vec<AgenticMatch>>(&trimmed[start..=end]) {
                return v;
            }
        }
    }
    Vec::new()
}
