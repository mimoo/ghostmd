use gpui::*;

use super::*;

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

        let ws_idx = self.active_workspace;
        self.ai_loading.insert(ws_idx);
        cx.notify();
        let output_path = std::env::temp_dir().join(format!("ghostmd-ai-tab-{}.json", std::process::id()));
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
                let status = std::process::Command::new("claude")
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            this.update(cx, |this, cx| {
                this.ai_loading.remove(&ws_idx);
                if result {
                    if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                        let _ = std::fs::remove_file(&output_path);
                        #[derive(serde::Deserialize)]
                        struct AiTab { title: String }
                        if let Ok(parsed) = serde_json::from_str::<AiTab>(&json_str) {
                            if ws_idx < this.workspaces.len() {
                                this.workspaces[ws_idx].title = parsed.title;
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
        let all_indices: Vec<usize> = (0..count).collect();
        for &i in &all_indices { self.ai_loading.insert(i); }
        cx.notify();
        let output_path = std::env::temp_dir().join(format!("ghostmd-ai-tabs-{}.json", std::process::id()));
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
                let status = std::process::Command::new("claude")
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            this.update(cx, |this, cx| {
                for i in &all_indices { this.ai_loading.remove(i); }
                if result {
                    if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                        let _ = std::fs::remove_file(&output_path);
                        #[derive(serde::Deserialize)]
                        struct AiTabs { titles: Vec<String> }
                        if let Ok(parsed) = serde_json::from_str::<AiTabs>(&json_str) {
                            for (i, name) in parsed.titles.iter().enumerate() {
                                if i < this.workspaces.len() {
                                    this.workspaces[i].title = name.clone();
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

        let output_path = std::env::temp_dir().join(format!("ghostmd-ai-file-{}.json", std::process::id()));
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
                let status = std::process::Command::new("claude")
                    .arg("-p")
                    .arg("--model").arg("sonnet")
                    .arg("--allowedTools").arg("Write")
                    .arg("--dangerously-skip-permissions")
                    .arg("--").arg(&prompt)
                    .status();
                status.is_ok_and(|s| s.success())
            }).await;
            if result {
                if let Ok(json_str) = std::fs::read_to_string(&output_path) {
                    let _ = std::fs::remove_file(&output_path);
                    #[derive(serde::Deserialize)]
                    struct AiFile { filename: String }
                    if let Ok(parsed) = serde_json::from_str::<AiFile>(&json_str) {
                        let suggested = parsed.filename;
                        if !suggested.is_empty() {
                            this.update(cx, |this, cx| {
                                let parent = path.parent().unwrap_or(&this.app.root).to_path_buf();
                                let mut new_path = parent.join(format!("{}{}", suggested, ext));
                                if new_path.exists() {
                                    for n in 2..100 {
                                        let candidate = parent.join(format!("{}-{}{}", suggested, n, ext));
                                        if !candidate.exists() {
                                            new_path = candidate;
                                            break;
                                        }
                                    }
                                }
                                if new_path != path && !new_path.exists()
                                    && std::fs::rename(&path, &new_path).is_ok()
                                {
                                    let mut editors_to_update = Vec::new();
                                    for ws in &mut this.workspaces {
                                        for pane in ws.panes.values_mut() {
                                            if pane.active_path.as_ref() == Some(&path) {
                                                pane.active_path = Some(new_path.clone());
                                                if let Some(editor) = &pane.editor {
                                                    editors_to_update.push(editor.clone());
                                                }
                                            }
                                        }
                                    }
                                    for ed in editors_to_update {
                                        let np = new_path.clone();
                                        ed.update(cx, |e, _cx| { e.path = np; });
                                    }
                                    this.file_tree.update(cx, |tree, cx| {
                                        tree.refresh(cx);
                                        tree.reveal_file(&new_path, cx);
                                    });
                                }
                                cx.notify();
                            }).ok();
                        }
                    }
                }
            }
            let _ = std::fs::remove_file(&output_path_str);
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

        let root = self.app.root.clone();

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
                std::process::Command::new("claude")
                    .arg("-p")
                    .arg("--model").arg("haiku").arg("--effort").arg("low")
                    .arg(&prompt)
                    .output()
            }).await;
            if let Ok(out) = output {
                let suggested = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !suggested.is_empty() {
                    this.update(cx, |this, cx| {
                        let target_dir = this.app.root.join(&suggested);
                        std::fs::create_dir_all(&target_dir).ok();
                        this.move_file_to_dir(source, &target_dir, cx);
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();
    }

    pub(crate) fn run_agentic_search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let query = self.agentic_input.read(cx).value().to_string().trim().to_string();
        if query.is_empty() {
            return;
        }
        self.agentic_loading = true;
        self.agentic_results.clear();
        cx.notify();

        let root = self.app.root.clone();
        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let output = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "Search through the markdown notes in {} and answer: {}. \
                     Be concise. List relevant file paths and quotes.",
                    root.display(), query
                );
                std::process::Command::new("claude")
                    .arg("-p")
                    .arg(&prompt)
                    .current_dir(&root)
                    .output()
            }).await;

            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let lines: Vec<String> = if text.trim().is_empty() {
                        if stderr.trim().is_empty() {
                            vec!["No results found.".to_string()]
                        } else {
                            vec![format!("Error: {}", stderr.trim())]
                        }
                    } else {
                        text.lines().map(|l| l.to_string()).collect()
                    };
                    this.update(cx, |this, cx| {
                        this.agentic_results = lines;
                        this.agentic_loading = false;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.agentic_results = vec![format!("Failed to run claude: {}", e)];
                        this.agentic_loading = false;
                        cx.notify();
                    }).ok();
                }
            }
        })
        .detach();
    }

    /// Update match count based on current search query and focused editor.
    pub(crate) fn update_search_matches(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string().to_lowercase();
        if query.is_empty() {
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
}
