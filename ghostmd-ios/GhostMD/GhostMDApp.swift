import SwiftUI

@main
struct GhostMDApp: App {
    @State private var store = NoteStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(store)
        }
    }
}
