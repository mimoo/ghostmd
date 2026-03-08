import Foundation

struct FileNode: Identifiable, Hashable {
    let url: URL
    let isDirectory: Bool
    let modificationDate: Date

    var id: URL { url }

    var name: String {
        url.deletingPathExtension().lastPathComponent
    }

    /// Display-friendly name: strips HHMMSS- timestamp prefix and replaces hyphens with spaces.
    var displayName: String {
        let raw = name
        if raw.count > 7,
           raw.prefix(6).allSatisfy(\.isNumber),
           raw[raw.index(raw.startIndex, offsetBy: 6)] == "-" {
            return String(raw.dropFirst(7)).replacingOccurrences(of: "-", with: " ")
        }
        return raw.replacingOccurrences(of: "-", with: " ")
    }
}
