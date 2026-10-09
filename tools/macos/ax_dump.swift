// What macOS accessibility — VoiceOver — sees of a RapidR program's windows,
// and AXPress run through it (AccessKit's adapter → the UI kernel → the
// program's handlers). The Windows counterpart is tools/windows/uia_probe.ps1.
//
//   swiftc -O tools/macos/ax_dump.swift -o /tmp/ax_dump
//   /tmp/ax_dump <pid> [ButtonTitle]
//   /tmp/ax_dump <pid> --text
//
// Prints each window's tree (role, title, description, value — a long
// value cut); with a button's title, presses it and prints the tree again.
// With --text, the first text area's text as VoiceOver reads it: its
// character count, selection, the caret's line, each line's range and
// string (AXRangeForLine, AXStringForRange), the selection's bounds. The
// process running it needs the Accessibility permission (System Settings →
// Privacy & Security). The first query wakes AccessKit's adapter; the tree
// is read after a short wait.
import Cocoa
let args = CommandLine.arguments
let pid = pid_t(Int32(args[1])!)
let press = args.count > 2 && args[2] != "--text" ? args[2] : nil
let textMode = args.count > 2 && args[2] == "--text"
let app = AXUIElementCreateApplication(pid)
func attr(_ e: AXUIElement, _ a: String) -> AnyObject? {
    var v: AnyObject?
    let err = AXUIElementCopyAttributeValue(e, a as CFString, &v)
    return err == .success ? v : nil
}
func param(_ e: AXUIElement, _ a: String, _ p: AnyObject) -> AnyObject? {
    var v: AnyObject?
    let err = AXUIElementCopyParameterizedAttributeValue(e, a as CFString, p, &v)
    return err == .success ? v : nil
}
func str(_ o: AnyObject?) -> String { o.map { "\($0)" } ?? "" }
func cut(_ s: String) -> String {
    let one = s.replacingOccurrences(of: "\n", with: "\\n").replacingOccurrences(of: "\r", with: "\\r").replacingOccurrences(of: "\t", with: "\\t")
    return one.count > 80 ? String(one.prefix(77)) + "..." : one
}
func range(_ o: AnyObject?) -> CFRange? {
    guard let o = o, CFGetTypeID(o) == AXValueGetTypeID() else { return nil }
    var r = CFRange()
    return AXValueGetValue(o as! AXValue, .cfRange, &r) ? r : nil
}
func rect(_ o: AnyObject?) -> CGRect? {
    guard let o = o, CFGetTypeID(o) == AXValueGetTypeID() else { return nil }
    var r = CGRect.zero
    return AXValueGetValue(o as! AXValue, .cgRect, &r) ? r : nil
}
func axRange(_ loc: Int, _ len: Int) -> AnyObject {
    var r = CFRange(location: loc, length: len)
    return AXValueCreate(.cfRange, &r)!
}
func dump(_ e: AXUIElement, _ d: Int) {
    if d > 10 { return }
    let line = String(repeating: "  ", count: d) + str(attr(e, kAXRoleAttribute)) + " title='" + str(attr(e, kAXTitleAttribute)) + "' desc='" + str(attr(e, kAXDescriptionAttribute)) + "' value='" + cut(str(attr(e, kAXValueAttribute))) + "'"
    print(line)
    for c in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { dump(c, d + 1) }
}
func find(_ e: AXUIElement, _ t: String) -> AXUIElement? {
    if str(attr(e, kAXRoleAttribute)) == "AXButton" && (str(attr(e, kAXTitleAttribute)) == t || str(attr(e, kAXDescriptionAttribute)) == t) { return e }
    for c in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { if let f = find(c, t) { return f } }
    return nil
}
func findRole(_ e: AXUIElement, _ role: String) -> AXUIElement? {
    if str(attr(e, kAXRoleAttribute)) == role { return e }
    for c in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { if let f = findRole(c, role) { return f } }
    return nil
}
func text(_ t: AXUIElement) {
    let value = str(attr(t, kAXValueAttribute))
    print("AXTextArea title='\(str(attr(t, kAXTitleAttribute)))' desc='\(str(attr(t, kAXDescriptionAttribute)))' help='\(str(attr(t, kAXHelpAttribute)))' focused=\(str(attr(t, kAXFocusedAttribute)))")
    print("  AXValue: \((value as NSString).length) UTF-16 units, '\(cut(value))'")
    print("  AXNumberOfCharacters: \(str(attr(t, kAXNumberOfCharactersAttribute)))")
    let sel = range(attr(t, kAXSelectedTextRangeAttribute))
    print("  AXSelectedTextRange: \(sel.map { "{\($0.location), \($0.length)}" } ?? "-")  AXSelectedText: '\(cut(str(attr(t, kAXSelectedTextAttribute))))'")
    print("  AXInsertionPointLineNumber: \(str(attr(t, kAXInsertionPointLineNumberAttribute)))")
    if let v = range(attr(t, kAXVisibleCharacterRangeAttribute)) { print("  AXVisibleCharacterRange: {\(v.location), \(v.length)}") }
    if let s = sel {
        print("  AXLineForIndex(\(s.location)): \(str(param(t, kAXLineForIndexParameterizedAttribute, s.location as NSNumber)))")
        if let b = rect(param(t, kAXBoundsForRangeParameterizedAttribute, axRange(s.location, max(s.length, 1)))) { print("  AXBoundsForRange(selection): \(b)") }
        if let s2 = range(param(t, kAXStyleRangeForIndexParameterizedAttribute, s.location as NSNumber)) { print("  AXStyleRangeForIndex: {\(s2.location), \(s2.length)}") }
    }
    for l in 0..<6 {
        guard let r = range(param(t, kAXRangeForLineParameterizedAttribute, l as NSNumber)) else { print("  line \(l): -"); continue }
        let s = str(param(t, kAXStringForRangeParameterizedAttribute, axRange(r.location, r.length)))
        let b = rect(param(t, kAXBoundsForRangeParameterizedAttribute, axRange(r.location, r.length)))
        print("  line \(l): {\(r.location), \(r.length)} '\(cut(s))' bounds=\(b.map { "\($0)" } ?? "-")")
    }
}
_ = attr(app, kAXWindowsAttribute as String)
usleep(500000)
let wins = (attr(app, kAXWindowsAttribute as String) as? [AXUIElement]) ?? []
if textMode {
    for w in wins { if let t = findRole(w, "AXTextArea") { text(t); break } }
    exit(0)
}
for w in wins { dump(w, 0) }
if let t = press, let w = wins.first {
    if let b = find(w, t) {
        let r = AXUIElementPerformAction(b, kAXPressAction as CFString)
        print("== pressed \(t): \(r.rawValue)")
        usleep(800000)
        for w in (attr(app, kAXWindowsAttribute as String) as? [AXUIElement]) ?? [] { dump(w, 0) }
    } else { print("no button \(t)") }
}
