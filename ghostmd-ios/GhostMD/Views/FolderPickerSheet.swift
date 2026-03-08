import SwiftUI

struct FolderPickerSheet: View {
    @Environment(NoteStore.self) private var store
    @Environment(\.dismiss) private var dismiss
    let onSelected: (URL) -> Void

    @State private var newFolderName = ""
    @State private var showNewFolder = false
    @State private var newFolderParent: URL?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(store.allFolders(), id: \.self) { folder in
                        Button {
                            onSelected(folder)
                            dismiss()
                        } label: {
                            Label(
                                store.relativePath(of: folder),
                                systemImage: folder == store.rootURL ? "house" : "folder"
                            )
                        }
                        .accessibilityIdentifier("folderRow_\(store.relativePath(of: folder))")
                    }
                }

                Section {
                    Button {
                        newFolderParent = store.rootURL
                        showNewFolder = true
                    } label: {
                        Label("New Folder...", systemImage: "folder.badge.plus")
                    }
                    .accessibilityIdentifier("newFolderButton")
                }
            }
            .navigationTitle("Choose Folder")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .accessibilityIdentifier("cancelButton")
                }
            }
            .alert("New Folder", isPresented: $showNewFolder) {
                TextField("Folder name", text: $newFolderName)
                    .accessibilityIdentifier("newFolderTextField")
                Button("Cancel", role: .cancel) { newFolderName = "" }
                    .accessibilityIdentifier("newFolderCancelButton")
                Button("Create") {
                    if let parent = newFolderParent,
                       let folder = store.createFolder(in: parent, name: newFolderName) {
                        newFolderName = ""
                        onSelected(folder)
                        dismiss()
                    }
                }
                .accessibilityIdentifier("newFolderCreateButton")
            }
        }
        .presentationDetents([.medium, .large])
    }
}
