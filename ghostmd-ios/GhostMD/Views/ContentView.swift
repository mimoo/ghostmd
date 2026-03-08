import SwiftUI

struct ContentView: View {
    @Environment(NoteStore.self) private var store
    @Environment(\.scenePhase) private var scenePhase
    @State private var path: [Route] = []

    var body: some View {
        NavigationStack(path: $path) {
            FolderView(folderURL: store.rootURL, path: $path)
                .navigationDestination(for: Route.self) { route in
                    switch route {
                    case .folder(let url):
                        FolderView(folderURL: url, path: $path)
                    case .note(let url):
                        NoteEditorView(noteURL: url)
                    }
                }
        }
        .onChange(of: scenePhase) { _, phase in
            if phase != .active {
                store.saveImmediately()
            }
        }
    }
}
