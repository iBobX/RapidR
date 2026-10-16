// What a user — and VoiceOver — gets of a RapidR program's windows, as
// JSON, for tests that drive the program with real input
// (tools/real_input.py) and check what it shows (tests/studio_ai_real.mjs):
//
//   swiftc -O tools/macos/ui_probe.swift -o <dir>/ui_probe
//   ui_probe tree <pid>        every window's accessibility tree: role,
//                              title, description, value, enabled, focused,
//                              expanded, its frame on the screen (points)
//   ui_probe ocr <image.png>   the text macOS' Vision reads in a picture
//                              (a capture of a window): each line and its
//                              box in the picture's pixels, top-left origin
//
// The process running it needs the Accessibility permission (System
// Settings → Privacy & Security) for `tree`. AccessKit's adapter wakes on
// the first query: the tree is read after a short wait.
import Cocoa
import Vision

func attr(_ e: AXUIElement, _ a: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(e, a as CFString, &v) == .success ? v : nil
}
func str(_ o: AnyObject?) -> String {
    guard let o = o else { return "" }
    if let s = o as? String { return s }
    if let n = o as? NSNumber { return n.stringValue }
    return "\(o)"
}
func point(_ o: AnyObject?) -> CGPoint? {
    guard let o = o, CFGetTypeID(o) == AXValueGetTypeID() else { return nil }
    var p = CGPoint.zero
    return AXValueGetValue(o as! AXValue, .cgPoint, &p) ? p : nil
}
func size(_ o: AnyObject?) -> CGSize? {
    guard let o = o, CFGetTypeID(o) == AXValueGetTypeID() else { return nil }
    var s = CGSize.zero
    return AXValueGetValue(o as! AXValue, .cgSize, &s) ? s : nil
}
func node(_ e: AXUIElement, _ depth: Int) -> [String: Any] {
    var n: [String: Any] = [
        "role": str(attr(e, kAXRoleAttribute)),
        "title": str(attr(e, kAXTitleAttribute)),
        "desc": str(attr(e, kAXDescriptionAttribute)),
        "value": str(attr(e, kAXValueAttribute)),
    ]
    if let en = attr(e, kAXEnabledAttribute) as? NSNumber { n["enabled"] = en.boolValue }
    if let f = attr(e, kAXFocusedAttribute) as? NSNumber { n["focused"] = f.boolValue }
    if let x = attr(e, kAXExpandedAttribute) as? NSNumber { n["expanded"] = x.boolValue }
    let help = str(attr(e, kAXHelpAttribute))
    if !help.isEmpty { n["help"] = help }
    if let p = point(attr(e, kAXPositionAttribute)), let s = size(attr(e, kAXSizeAttribute)) {
        n["x"] = Double(p.x); n["y"] = Double(p.y); n["w"] = Double(s.width); n["h"] = Double(s.height)
    }
    if depth < 24 {
        let kids = (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? []
        if !kids.isEmpty { n["children"] = kids.map { node($0, depth + 1) } }
    }
    return n
}
func emit(_ v: Any) {
    let data = try! JSONSerialization.data(withJSONObject: v, options: [])
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
}

let args = CommandLine.arguments
guard args.count >= 3 else {
    FileHandle.standardError.write("usage: ui_probe tree <pid> | ocr <image.png>\n".data(using: .utf8)!)
    exit(2)
}
switch args[1] {
case "tree":
    let app = AXUIElementCreateApplication(pid_t(Int32(args[2])!))
    _ = attr(app, kAXWindowsAttribute as String)
    usleep(300000)
    let wins = (attr(app, kAXWindowsAttribute as String) as? [AXUIElement]) ?? []
    emit(wins.map { node($0, 0) })
case "ocr":
    guard let img = NSImage(contentsOfFile: args[2]), let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
        FileHandle.standardError.write("can't read \(args[2])\n".data(using: .utf8)!)
        exit(1)
    }
    let req = VNRecognizeTextRequest()
    req.recognitionLevel = .accurate
    req.usesLanguageCorrection = false
    try VNImageRequestHandler(cgImage: cg, options: [:]).perform([req])
    let W = Double(cg.width), H = Double(cg.height)
    var out: [[String: Any]] = []
    for o in req.results ?? [] {
        guard let t = o.topCandidates(1).first else { continue }
        let b = o.boundingBox
        out.append(["text": t.string, "x": b.minX * W, "y": (1 - b.maxY) * H, "w": b.width * W, "h": b.height * H])
    }
    emit(out)
default:
    FileHandle.standardError.write("unknown command \(args[1])\n".data(using: .utf8)!)
    exit(2)
}
