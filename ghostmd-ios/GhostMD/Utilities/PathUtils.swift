import Foundation

enum PathUtils {

    // MARK: - Slugify

    static func slugify(_ s: String) -> String {
        let replaced = s.lowercased().unicodeScalars.map { c in
            CharacterSet.alphanumerics.contains(c) ? String(c) : "-"
        }.joined()

        let collapsed = replaced
            .components(separatedBy: "-")
            .filter { !$0.isEmpty }
            .joined(separator: "-")

        return collapsed.isEmpty ? "untitled" : collapsed
    }

    // MARK: - Collision-safe path

    static func uniquePath(_ url: URL) -> URL {
        let fm = FileManager.default
        guard fm.fileExists(atPath: url.path(percentEncoded: false)) else { return url }

        let dir = url.deletingLastPathComponent()
        let name = url.deletingPathExtension().lastPathComponent
        let ext = url.pathExtension

        for i in 2...999 {
            let filename = ext.isEmpty ? "\(name)-\(i)" : "\(name)-\(i).\(ext)"
            let candidate = dir.appending(path: filename)
            if !fm.fileExists(atPath: candidate.path(percentEncoded: false)) {
                return candidate
            }
        }
        return url
    }

    // MARK: - Note naming

    static func pickNoteName(in dir: URL) -> String {
        let notesPath = dir.appending(path: "notes.md")
        if !FileManager.default.fileExists(atPath: notesPath.path(percentEncoded: false)) {
            return "notes"
        }
        return randomNoteName()
    }

    static func randomNoteName() -> String {
        let adj = adjectives.randomElement()!
        let noun = nouns.randomElement()!
        return "\(adj)-\(noun)"
    }

    private static let adjectives = [
        "amber", "azure", "bold", "brisk", "calm", "clear", "cool", "crisp",
        "dark", "deep", "fair", "fast", "fine", "fresh", "full", "gentle",
        "gold", "green", "keen", "kind", "light", "mild", "neat", "noble",
        "pale", "pure", "quick", "quiet", "rare", "rich", "sharp", "silent",
        "slim", "soft", "still", "swift", "tall", "true", "vast", "warm",
        "wide", "wild", "wise", "young",
    ]

    private static let nouns = [
        "arch", "birch", "bloom", "brook", "cedar", "cliff", "cloud", "coral",
        "cove", "creek", "dawn", "dew", "drift", "dune", "echo", "edge",
        "elm", "ember", "fern", "flame", "frost", "gale", "gem", "glen",
        "grove", "haven", "hawk", "heath", "hill", "iris", "isle", "jade",
        "lake", "lark", "leaf", "lily", "marsh", "mist", "moon", "moss",
        "oak", "path", "peak", "pine", "rain", "reef", "ridge", "river",
        "sage", "shade", "shore", "snow", "spark", "star", "stone", "storm",
        "trail", "vale", "vine", "wave", "wind", "wing", "wren",
    ]
}
