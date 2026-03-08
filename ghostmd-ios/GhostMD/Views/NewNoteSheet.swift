import SwiftUI

struct NewNoteSheet: View {
    @Environment(NoteStore.self) private var store
    @Environment(\.dismiss) private var dismiss
    let currentFolder: URL
    let onCreated: (URL) -> Void

    @State private var showFolderPicker = false
    @State private var pendingFromPicker: URL?

    private var currentFolderLabel: String {
        currentFolder == store.rootURL
            ? "Notes (root)"
            : currentFolder.lastPathComponent
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Button {
                        if let url = store.createDiaryNote() {
                            onCreated(url)
                        }
                    } label: {
                        Label {
                            VStack(alignment: .leading) {
                                Text("Today's Diary")
                                Text(Diary.todayDir(root: store.rootURL).lastPathComponent)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        } icon: {
                            Image(systemName: "calendar")
                        }
                    }
                    .accessibilityIdentifier("newNoteDiaryButton")

                    Button {
                        if let url = store.createNote(in: currentFolder) {
                            onCreated(url)
                        }
                    } label: {
                        Label {
                            VStack(alignment: .leading) {
                                Text("In Current Folder")
                                Text(currentFolderLabel)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        } icon: {
                            Image(systemName: "folder")
                        }
                    }
                    .accessibilityIdentifier("newNoteCurrentFolderButton")

                    Button {
                        showFolderPicker = true
                    } label: {
                        Label("Choose Folder...", systemImage: "folder.badge.questionmark")
                    }
                    .accessibilityIdentifier("newNoteChooseFolderButton")
                } header: {
                    Text("Where to create the note?")
                }
            }
            .navigationTitle("New Note")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .accessibilityIdentifier("cancelButton")
                }
            }
            .sheet(isPresented: $showFolderPicker, onDismiss: {
                if let folder = pendingFromPicker {
                    pendingFromPicker = nil
                    if let url = store.createNote(in: folder) {
                        onCreated(url)
                    }
                }
            }) {
                FolderPickerSheet { folder in
                    pendingFromPicker = folder
                    showFolderPicker = false
                }
            }
        }
        .presentationDetents([.medium])
    }
}
