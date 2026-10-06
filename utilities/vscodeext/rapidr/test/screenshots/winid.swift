import CoreGraphics
import Foundation

// Prints the CGWindowID of the first on-screen window whose title contains argv[1].
let needle = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : ""
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
for w in list {
    let name = w[kCGWindowName as String] as? String ?? ""
    let layer = w[kCGWindowLayer as String] as? Int ?? 0
    if layer == 0 && name.contains(needle) {
        print(w[kCGWindowNumber as String] as? Int ?? 0)
        exit(0)
    }
}
exit(1)
