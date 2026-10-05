// What macOS accessibility — VoiceOver — sees of a RapidR program's windows,
// and AXPress run through it (AccessKit's adapter → the UI kernel → the
// program's handlers). The Windows counterpart is tools/windows/uia_probe.ps1.
//
//   swiftc -O tools/macos/ax_dump.swift -o /tmp/ax_dump
//   /tmp/ax_dump <pid> [ButtonTitle]
//
// Prints each window's tree (role, title, description, value); with a
// button's title, presses it and prints the tree again. The process
// running it needs the Accessibility permission (System Settings → Privacy
// & Security). The first query wakes AccessKit's adapter; the tree is read
// after a short wait.
import Cocoa
let args = CommandLine.arguments
let pid = pid_t(Int32(args[1])!)
let press = args.count > 2 ? args[2] : nil
let app = AXUIElementCreateApplication(pid)
func attr(_ e: AXUIElement, _ a: String) -> AnyObject? {
    var v: AnyObject?
    let err = AXUIElementCopyAttributeValue(e, a as CFString, &v)
    return err == .success ? v : nil
}
func str(_ o: AnyObject?) -> String { o.map { "\($0)" } ?? "" }
func dump(_ e: AXUIElement, _ d: Int) {
    if d > 10 { return }
    let line = String(repeating: "  ", count: d) + str(attr(e, kAXRoleAttribute)) + " title='" + str(attr(e, kAXTitleAttribute)) + "' desc='" + str(attr(e, kAXDescriptionAttribute)) + "' value='" + str(attr(e, kAXValueAttribute)) + "'"
    print(line)
    for c in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { dump(c, d + 1) }
}
func find(_ e: AXUIElement, _ t: String) -> AXUIElement? {
    if str(attr(e, kAXRoleAttribute)) == "AXButton" && (str(attr(e, kAXTitleAttribute)) == t || str(attr(e, kAXDescriptionAttribute)) == t) { return e }
    for c in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { if let f = find(c, t) { return f } }
    return nil
}
_ = attr(app, kAXWindowsAttribute as String)
usleep(500000)
let wins = (attr(app, kAXWindowsAttribute as String) as? [AXUIElement]) ?? []
for w in wins { dump(w, 0) }
if let t = press, let w = wins.first {
    if let b = find(w, t) {
        let r = AXUIElementPerformAction(b, kAXPressAction as CFString)
        print("== pressed \(t): \(r.rawValue)")
        usleep(800000)
        for w in (attr(app, kAXWindowsAttribute as String) as? [AXUIElement]) ?? [] { dump(w, 0) }
    } else { print("no button \(t)") }
}
