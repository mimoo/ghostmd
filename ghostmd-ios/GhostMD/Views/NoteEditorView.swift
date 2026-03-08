import SwiftUI

struct NoteEditorView: View {
    @Environment(NoteStore.self) private var store
    @Environment(\.dismiss) private var dismiss
    let noteURL: URL

    @FocusState private var isFocused: Bool
    @State private var showMoveSheet = false
    @State private var showRenameAlert = false
    @State private var renameText = ""
    @State private var showDeleteConfirm = false

    private var title: String {
        guard let url = store.editingURL else { return "Note" }
        return FileNode(url: url, isDirectory: false, modificationDate: .now).displayName
    }

    var body: some View {
        @Bindable var store = store
        TextEditor(text: $store.editingContent)
            .font(.system(.body, design: .monospaced))
            .focused($isFocused)
            .scrollDismissesKeyboard(.interactively)
            .accessibilityIdentifier("noteEditor")
            .onChange(of: store.editingContent) {
                store.contentChanged()
            }
            .onAppear {
                store.openNote(noteURL)
                isFocused = true
            }
            .onDisappear {
                store.saveImmediately()
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        Button {
                            renameText = store.editingURL?.deletingPathExtension().lastPathComponent ?? ""
                            showRenameAlert = true
                        } label: {
                            Label("Rename", systemImage: "pencil")
                        }
                        .accessibilityIdentifier("renameButton")

                        Button {
                            showMoveSheet = true
                        } label: {
                            Label("Move to...", systemImage: "folder")
                        }
                        .accessibilityIdentifier("moveButton")

                        Divider()

                        Button(role: .destructive) {
                            showDeleteConfirm = true
                        } label: {
                            Label("Delete", systemImage: "trash")
                        }
                        .accessibilityIdentifier("deleteButton")
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                    .accessibilityIdentifier("menuButton")
                }

                ToolbarItem(placement: .keyboard) {
                    HStack {
                        Spacer()
                        Button("Done") { isFocused = false }
                            .accessibilityIdentifier("doneButton")
                    }
                }
            }
            .sheet(isPresented: $showMoveSheet) {
                FolderPickerSheet { folder in
                    if let newURL = store.moveNote(noteURL, toFolder: folder) {
                        // Stay on editor — URL is updated in store
                        _ = newURL
                    }
                }
            }
            .alert("Rename", isPresented: $showRenameAlert) {
                TextField("Name", text: $renameText)
                    .accessibilityIdentifier("renameTextField")
                Button("Cancel", role: .cancel) { }
                    .accessibilityIdentifier("renameCancelButton")
                Button("Rename") {
                    if let url = store.editingURL {
                        _ = store.renameNote(url, to: renameText)
                    }
                }
                .accessibilityIdentifier("renameConfirmButton")
            }
            .confirmationDialog("Delete this note?", isPresented: $showDeleteConfirm, titleVisibility: .visible) {
                Button("Delete", role: .destructive) {
                    if store.deleteNote(noteURL) {
                        dismiss()
                    }
                }
                .accessibilityIdentifier("confirmDeleteButton")
                Button("Cancel", role: .cancel) { }
                    .accessibilityIdentifier("cancelDeleteButton")
            }
    }
}
