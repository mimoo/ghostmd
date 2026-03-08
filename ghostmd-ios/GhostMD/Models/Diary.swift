import Foundation

enum Diary {

    /// Diary directory for a specific date: `<root>/diary/YYYY/month/DD/`
    static func diaryDir(root: URL, date: Date) -> URL {
        let cal = Calendar.current
        let year = cal.component(.year, from: date)
        let monthIndex = cal.component(.month, from: date) - 1
        let month = DateFormatter().monthSymbols[monthIndex].lowercased()
        let day = String(format: "%02d", cal.component(.day, from: date))

        return root
            .appending(path: "diary")
            .appending(path: String(year))
            .appending(path: month)
            .appending(path: day)
    }

    /// Diary directory for today.
    static func todayDir(root: URL) -> URL {
        diaryDir(root: root, date: Date())
    }

    /// New diary note path with timestamp-slug filename.
    static func newDiaryPath(root: URL, name: String) -> URL {
        let dir = todayDir(root: root)
        let formatter = DateFormatter()
        formatter.dateFormat = "HHmmss"
        let timestamp = formatter.string(from: Date())
        let slug = PathUtils.slugify(name)
        return dir.appending(path: "\(timestamp)-\(slug).md")
    }
}
