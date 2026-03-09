import SwiftUI

struct FolderView: View {
    @Environment(NoteStore.self) private var store
    let folderURL: URL
    @Binding var path: [Route]

    @State private var contents: [FileNode] = []
    @State private var searchText = ""
    @State private var showNewNote = false
    @State private var pendingNoteURL: URL?
    @State private var showSearch = false

    // Context menu actions
    @State private var showRenameAlert = false
    @State private var renameTarget: URL?
    @State private var renameText = ""
    @State private var showMoveSheet = false
    @State private var moveTarget: URL?

    private var isRoot: Bool { folderURL == store.rootURL }

    private var filteredContents: [FileNode] {
        if searchText.isEmpty { return contents }
        return contents.filter { $0.displayName.localizedCaseInsensitiveContains(searchText) }
    }

    var body: some View {
        Group {
            if contents.isEmpty {
                ContentUnavailableView {
                    Label("No Notes", systemImage: "note.text")
                } description: {
                    Text("Tap  \(Image(systemName: "square.and.pencil"))  to create your first note")
                }
                .accessibilityIdentifier("emptyState")
            } else {
                List {
                    ForEach(filteredContents) { node in
                        if node.isDirectory {
                            NavigationLink(value: Route.folder(node.url)) {
                                Label(node.displayName, systemImage: "folder")
                            }
                            .accessibilityIdentifier("folderRow_\(node.displayName)")
                            .contextMenu { folderContextMenu(node) }
                        } else {
                            NavigationLink(value: Route.note(node.url)) {
                                noteRow(node)
                            }
                            .accessibilityIdentifier("noteRow_\(node.displayName)")
                            .contextMenu { noteContextMenu(node) }
                        }
                    }
                    .onDelete(perform: deleteItems)
                }
                .listStyle(.plain)
                .searchable(text: $searchText, prompt: "Filter")
                .accessibilityIdentifier("notesList")
            }
        }
        .navigationTitle(isRoot ? "GhostMD" : folderURL.lastPathComponent)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                HStack(spacing: 16) {
                    Button { showSearch = true } label: {
                        Image(systemName: "magnifyingglass")
                    }
                    .accessibilityIdentifier("searchButton")
                    Button { showNewNote = true } label: {
                        Image(systemName: "square.and.pencil")
                    }
                    .accessibilityIdentifier("composeButton")
                }
            }
        }
        .sheet(isPresented: $showSearch) {
            SearchSheet { url in
                showSearch = false
                path.append(.note(url))
            }
        }
        .sheet(isPresented: $showNewNote, onDismiss: {
            if let url = pendingNoteURL {
                pendingNoteURL = nil
                path.append(.note(url))
            }
        }) {
            NewNoteSheet(currentFolder: folderURL) { url in
                pendingNoteURL = url
                showNewNote = false
            }
        }
        .sheet(isPresented: $showMoveSheet) {
            if let target = moveTarget {
                FolderPickerSheet { folder in
                    if let _ = store.moveNote(target, toFolder: folder) {
                        refresh()
                    }
                }
            }
        }
        .alert("Rename", isPresented: $showRenameAlert) {
            TextField("Name", text: $renameText)
                .accessibilityIdentifier("renameTextField")
            Button("Cancel", role: .cancel) { }
                .accessibilityIdentifier("renameCancelButton")
            Button("Rename") {
                if let target = renameTarget {
                    _ = store.renameNote(target, to: renameText)
                    refresh()
                }
            }
            .accessibilityIdentifier("renameConfirmButton")
        }
        .onAppear { refresh() }
        .refreshable { refresh() }
    }

    @ViewBuilder
    private func noteRow(_ node: FileNode) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(node.displayName)
                .font(.body.weight(.medium))
            HStack(spacing: 4) {
                Text(store.relativePath(of: node.url.deletingLastPathComponent()))
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                Text("/")
                    .font(.caption)
                    .foregroundStyle(.quaternary)
                Text(node.url.lastPathComponent)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .lineLimit(1)
            Text(node.modificationDate.friendlyDate)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.vertical, 2)
    }

    private func refresh() {
        contents = store.contentsOf(folder: folderURL)
    }

    private func deleteItems(at offsets: IndexSet) {
        for index in offsets {
            let node = filteredContents[index]
            _ = store.deleteNote(node.url)
        }
        refresh()
    }

    @ViewBuilder
    private func noteContextMenu(_ node: FileNode) -> some View {
        Button {
            renameTarget = node.url
            renameText = node.name
            showRenameAlert = true
        } label: {
            Label("Rename", systemImage: "pencil")
        }
        .accessibilityIdentifier("renameButton")

        Button {
            moveTarget = node.url
            showMoveSheet = true
        } label: {
            Label("Move to...", systemImage: "folder")
        }
        .accessibilityIdentifier("moveButton")

        Divider()

        Button(role: .destructive) {
            _ = store.deleteNote(node.url)
            refresh()
        } label: {
            Label("Delete", systemImage: "trash")
        }
        .accessibilityIdentifier("deleteButton")
    }

    @ViewBuilder
    private func folderContextMenu(_ node: FileNode) -> some View {
        Button(role: .destructive) {
            _ = store.deleteNote(node.url)
            refresh()
        } label: {
            Label("Delete Folder", systemImage: "trash")
        }
        .accessibilityIdentifier("deleteFolderButton")
    }
}
