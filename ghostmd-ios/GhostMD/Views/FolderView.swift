import SwiftUI

struct FolderView: View {
    @Environment(NoteStore.self) private var store
    let folderURL: URL
    @Binding var path: [Route]

    @State private var contents: [FileNode] = []
    @State private var searchText = ""
    @State private var showNewNote = false
    @State private var pendingNoteURL: URL?

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
            } else {
                List {
                    ForEach(filteredContents) { node in
                        if node.isDirectory {
                            NavigationLink(value: Route.folder(node.url)) {
                                Label(node.displayName, systemImage: "folder")
                            }
                            .contextMenu { folderContextMenu(node) }
                        } else {
                            NavigationLink(value: Route.note(node.url)) {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(node.displayName)
                                    Text(node.modificationDate, style: .relative)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .contextMenu { noteContextMenu(node) }
                        }
                    }
                    .onDelete(perform: deleteItems)
                }
                .searchable(text: $searchText, prompt: "Search")
            }
        }
        .navigationTitle(isRoot ? "GhostMD" : folderURL.lastPathComponent)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { showNewNote = true } label: {
                    Image(systemName: "square.and.pencil")
                }
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
            Button("Cancel", role: .cancel) { }
            Button("Rename") {
                if let target = renameTarget {
                    _ = store.renameNote(target, to: renameText)
                    refresh()
                }
            }
        }
        .onAppear { refresh() }
        .refreshable { refresh() }
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

        Button {
            moveTarget = node.url
            showMoveSheet = true
        } label: {
            Label("Move to...", systemImage: "folder")
        }

        Divider()

        Button(role: .destructive) {
            _ = store.deleteNote(node.url)
            refresh()
        } label: {
            Label("Delete", systemImage: "trash")
        }
    }

    @ViewBuilder
    private func folderContextMenu(_ node: FileNode) -> some View {
        Button(role: .destructive) {
            _ = store.deleteNote(node.url)
            refresh()
        } label: {
            Label("Delete Folder", systemImage: "trash")
        }
    }
}
