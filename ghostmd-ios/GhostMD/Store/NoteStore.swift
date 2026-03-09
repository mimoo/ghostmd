import Foundation
import Observation

@MainActor
@Observable
final class NoteStore {
    private(set) var rootURL: URL

    // Editor state
    var editingURL: URL?
    var editingContent: String = ""
    private(set) var isDirty = false

    @ObservationIgnored private var isLoadingContent = false
    @ObservationIgnored private var saveTask: Task<Void, Never>?

    init() {
        if ProcessInfo.processInfo.arguments.contains("--ui-testing") {
            // Isolated temp directory for UI tests
            let tmp = FileManager.default.temporaryDirectory.appending(path: "ghostmd-uitests")
            try? FileManager.default.removeItem(at: tmp)
            rootURL = tmp
        } else if let containerURL = FileManager.default.url(forUbiquityContainerIdentifier: nil) {
            rootURL = containerURL.appending(path: "Documents")
        } else {
            let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            rootURL = docs.appending(path: "ghostmd")
        }
        try? FileManager.default.createDirectory(at: rootURL, withIntermediateDirectories: true)
    }

    // MARK: - File Tree

    func contentsOf(folder: URL) -> [FileNode] {
        guard let items = try? FileManager.default.contentsOfDirectory(
            at: folder,
            includingPropertiesForKeys: [.isDirectoryKey, .contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        return items.compactMap { url in
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey, .contentModificationDateKey])
            let isDir = values?.isDirectory ?? false
            let modDate = values?.contentModificationDate ?? .distantPast

            // Skip the .ghostmd session folder
            if isDir && url.lastPathComponent == ".ghostmd" { return nil }
            // Only show .md files (skip other file types)
            if !isDir && url.pathExtension != "md" { return nil }

            return FileNode(url: url, isDirectory: isDir, modificationDate: modDate)
        }
        .sorted { a, b in
            // Folders first, then alphabetical
            if a.isDirectory != b.isDirectory { return a.isDirectory && !b.isDirectory }
            return a.name.localizedStandardCompare(b.name) == .orderedAscending
        }
    }

    // MARK: - Open / Save

    func openNote(_ url: URL) {
        saveImmediately()
        isLoadingContent = true
        editingURL = url
        editingContent = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        isDirty = false
        isLoadingContent = false
    }

    func contentChanged() {
        guard !isLoadingContent else { return }
        isDirty = true
        scheduleSave()
    }

    /// Save synchronously — call on background/inactive transitions to guarantee data is persisted.
    func saveImmediately() {
        saveTask?.cancel()
        saveTask = nil
        guard isDirty, let url = editingURL else { return }
        do {
            let dir = url.deletingLastPathComponent()
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            try editingContent.write(to: url, atomically: true, encoding: .utf8)
            isDirty = false
        } catch {
            print("Save failed: \(error)")
        }
    }

    private func scheduleSave() {
        saveTask?.cancel()
        saveTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            self?.saveImmediately()
        }
    }

    // MARK: - Create

    func createNote(in folder: URL) -> URL? {
        let name = PathUtils.pickNoteName(in: folder)
        let url = PathUtils.uniquePath(folder.appending(path: "\(name).md"))
        do {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try "".write(to: url, atomically: true, encoding: .utf8)
            return url
        } catch {
            print("Create note failed: \(error)")
            return nil
        }
    }

    func createDiaryNote() -> URL? {
        let dir = Diary.todayDir(root: rootURL)
        let name = PathUtils.pickNoteName(in: dir)
        let url = Diary.newDiaryPath(root: rootURL, name: name)
        do {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            try "".write(to: url, atomically: true, encoding: .utf8)
            return url
        } catch {
            print("Create diary note failed: \(error)")
            return nil
        }
    }

    // MARK: - Move / Delete / Rename

    func moveNote(_ url: URL, toFolder: URL) -> URL? {
        let dest = PathUtils.uniquePath(toFolder.appending(path: url.lastPathComponent))
        do {
            try FileManager.default.createDirectory(at: toFolder, withIntermediateDirectories: true)
            try FileManager.default.moveItem(at: url, to: dest)
            if editingURL == url { editingURL = dest }
            return dest
        } catch {
            print("Move failed: \(error)")
            return nil
        }
    }

    func deleteNote(_ url: URL) -> Bool {
        do {
            try FileManager.default.trashItem(at: url, resultingItemURL: nil)
            if editingURL == url {
                editingURL = nil
                editingContent = ""
                isDirty = false
            }
            return true
        } catch {
            // trashItem may not be available; fall back to removeItem
            do {
                try FileManager.default.removeItem(at: url)
                if editingURL == url {
                    editingURL = nil
                    editingContent = ""
                    isDirty = false
                }
                return true
            } catch {
                print("Delete failed: \(error)")
                return false
            }
        }
    }

    func renameNote(_ url: URL, to newName: String) -> URL? {
        let ext = url.pathExtension.isEmpty ? "md" : url.pathExtension
        let slug = PathUtils.slugify(newName)
        guard !slug.isEmpty, slug != "untitled" || !newName.isEmpty else { return nil }
        let newURL = PathUtils.uniquePath(
            url.deletingLastPathComponent().appending(path: "\(slug).\(ext)")
        )
        do {
            try FileManager.default.moveItem(at: url, to: newURL)
            if editingURL == url { editingURL = newURL }
            return newURL
        } catch {
            print("Rename failed: \(error)")
            return nil
        }
    }

    // MARK: - Folder Helpers

    func allFolders() -> [URL] {
        var result: [URL] = [rootURL]
        collectFolders(in: rootURL, into: &result)
        return result
    }

    private func collectFolders(in dir: URL, into result: inout [URL]) {
        guard let items = try? FileManager.default.contentsOfDirectory(
            at: dir,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return }

        for item in items {
            let isDir = (try? item.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory ?? false
            if isDir && item.lastPathComponent != ".ghostmd" {
                result.append(item)
                collectFolders(in: item, into: &result)
            }
        }
    }

    func relativePath(of url: URL) -> String {
        let root = rootURL.path(percentEncoded: false)
        let full = url.path(percentEncoded: false)
        if full == root { return "Notes (root)" }
        if full.hasPrefix(root) {
            var rel = String(full.dropFirst(root.count))
            if rel.hasPrefix("/") { rel = String(rel.dropFirst()) }
            return rel
        }
        return url.lastPathComponent
    }

    // MARK: - Search

    /// Recursively collect all notes under rootURL.
    func allNotes() -> [FileNode] {
        var result: [FileNode] = []
        collectNotes(in: rootURL, into: &result)
        return result
    }

    private func collectNotes(in dir: URL, into result: inout [FileNode]) {
        guard let items = try? FileManager.default.contentsOfDirectory(
            at: dir,
            includingPropertiesForKeys: [.isDirectoryKey, .contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) else { return }

        for url in items {
            let values = try? url.resourceValues(forKeys: [.isDirectoryKey, .contentModificationDateKey])
            let isDir = values?.isDirectory ?? false
            let modDate = values?.contentModificationDate ?? .distantPast

            if isDir {
                if url.lastPathComponent != ".ghostmd" {
                    collectNotes(in: url, into: &result)
                }
            } else if url.pathExtension == "md" {
                result.append(FileNode(url: url, isDirectory: false, modificationDate: modDate))
            }
        }
    }

    /// Fuzzy search: matches against path, title, then content. Returns scored results.
    func search(query: String) -> [FileNode] {
        guard !query.isEmpty else { return [] }
        let q = query.lowercased()
        let notes = allNotes()

        struct Scored {
            let node: FileNode
            let score: Int
        }

        let scored: [Scored] = notes.compactMap { node in
            let path = relativePath(of: node.url).lowercased()
            let title = node.displayName.lowercased()
            let content = (try? String(contentsOf: node.url, encoding: .utf8))?.lowercased() ?? ""

            // Fuzzy: check if all characters of query appear in order
            func fuzzyMatch(_ haystack: String, _ needle: String) -> Bool {
                var it = haystack.makeIterator()
                for ch in needle {
                    while let next = it.next() {
                        if next == ch { break }
                    }
                    // If iterator exhausted before finding ch, check differently
                }
                // Simple substring containment is more useful for short queries
                return haystack.contains(needle)
            }

            // Score: lower is better. path match = 0, title = 100, content = 200
            if fuzzyMatch(title, q) {
                return Scored(node: node, score: 0)
            } else if fuzzyMatch(path, q) {
                return Scored(node: node, score: 100)
            } else if fuzzyMatch(content, q) {
                return Scored(node: node, score: 200)
            }
            return nil
        }

        return scored
            .sorted { $0.score == $1.score ? $0.node.displayName < $1.node.displayName : $0.score < $1.score }
            .map(\.node)
    }

    func createFolder(in parent: URL, name: String) -> URL? {
        let slug = PathUtils.slugify(name)
        let url = PathUtils.uniquePath(parent.appending(path: slug))
        do {
            try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
            return url
        } catch {
            print("Create folder failed: \(error)")
            return nil
        }
    }
}
