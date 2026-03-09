import Foundation

extension Date {
    var friendlyDate: String {
        let cal = Calendar.current
        if cal.isDateInToday(self) {
            return "today"
        } else if cal.isDateInYesterday(self) {
            return "yesterday"
        } else {
            let days = cal.dateComponents([.day], from: self, to: .now).day ?? 0
            if days < 7 {
                let fmt = DateFormatter()
                fmt.dateFormat = "EEEE" // e.g. "Wednesday"
                return fmt.string(from: self)
            } else {
                let fmt = DateFormatter()
                fmt.dateStyle = .medium
                fmt.timeStyle = .none
                return fmt.string(from: self)
            }
        }
    }
}
