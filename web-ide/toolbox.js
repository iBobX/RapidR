// Toolbox catalog — single source of truth for what tools exist
// and what their default property-bag looks like when a widget is dropped.
//
// Components fall into two visibility classes:
//   visible:    rendered on the form (Button, Label, Image, etc.)
//   invisible:  rendered in a "non-visual tray" below the form (Timer,
//               SQLite, HTTP, Num, etc.). The runtime instantiates them
//               just like visible widgets but they have no on-form layout.

export const TOOLBOX_GROUPS = [
  {
    name: "Standard",
    items: [
      { type: "RButton",     icon: "▭",  label: "Button",      visible: true },
      { type: "RLabel",      icon: "A",  label: "Label",       visible: true },
      { type: "REdit",       icon: "▤",  label: "Text Box",    visible: true },
      { type: "RMemo",       icon: "❡",  label: "Memo",        visible: true },
      { type: "RCheckBox",   icon: "☑",  label: "Check Box",   visible: true },
      { type: "RRadioBtn",   icon: "◉",  label: "Radio Button",visible: true },
      { type: "RComboBox",   icon: "▾",  label: "Combo Box",   visible: true },
      { type: "RListBox",    icon: "≣",  label: "List Box",    visible: true },
      { type: "RPanel",      icon: "▢",  label: "Panel",       visible: true },
      { type: "RGroupBox",   icon: "▣",  label: "Group Box",   visible: true },
    ],
  },
  {
    name: "Advanced",
    items: [
      { type: "RImage",         icon: "🖼", label: "Image",        visible: true },
      { type: "RCanvas",        icon: "🎨", label: "Canvas",       visible: true },
      { type: "RProgressBar",   icon: "⎯",  label: "Progress Bar", visible: true },
      { type: "RTrackBar",      icon: "⊟",  label: "Track Bar",    visible: true },
      { type: "RScrollBar",     icon: "║",  label: "Scroll Bar",   visible: true },
      { type: "RUpDown",        icon: "↕",  label: "Up/Down",      visible: true },
      { type: "RDateTimePicker",icon: "📅", label: "Date/Time",    visible: true },
      { type: "RStringGrid",    icon: "▦",  label: "String Grid",  visible: true },
      { type: "RListView",      icon: "☷",  label: "List View",    visible: true },
      { type: "RTreeView",      icon: "⊟",  label: "Tree View",    visible: true },
      { type: "RTabControl",    icon: "▤",  label: "Tab Control",  visible: true },
      { type: "RScrollBox",     icon: "▥",  label: "Scroll Box",   visible: true },
      { type: "RSplitter",      icon: "║",  label: "Splitter",     visible: true },
      { type: "RCodeEditor",    icon: "≡",  label: "Code Editor",  visible: true },
      { type: "RRichEdit",      icon: "📝", label: "Rich Edit",    visible: true },
      { type: "RCoolBtn",       icon: "▭",  label: "Cool Button",  visible: true },
      { type: "ROvalBtn",       icon: "○",  label: "Oval Button",  visible: true },
      { type: "RLine",          icon: "—",  label: "Line",         visible: true },
    ],
  },
  {
    name: "Menus",
    items: [
      { type: "RMainMenu",   icon: "≡",  label: "Main Menu",  visible: true  },
      { type: "RPopupMenu",  icon: "⋮",  label: "Popup Menu", visible: false },
      { type: "RToolBar",    icon: "▥",  label: "Tool Bar",   visible: true  },
      { type: "RStatusBar",  icon: "▬",  label: "Status Bar", visible: true  },
    ],
  },
  {
    name: "Data",
    items: [
      { type: "RDataFrame",  icon: "▦",  label: "Data Frame",     visible: true  },
      { type: "RPlot",       icon: "📈", label: "Plot",           visible: true  },
      { type: "RNum",        icon: "ƒ",  label: "Numeric Array",  visible: false },
      { type: "RJson",       icon: "{}", label: "JSON",           visible: false },
      { type: "RStringList", icon: "≡",  label: "String List",    visible: false },
    ],
  },
  {
    name: "Database",
    items: [
      { type: "RSqlite",     icon: "⛁",  label: "SQLite", visible: false },
      { type: "RMySql",      icon: "⛃",  label: "MySQL",  visible: false },
    ],
  },
  {
    name: "Network",
    items: [
      { type: "RHttp",         icon: "🌐", label: "HTTP Client",   visible: false },
      { type: "RSocket",       icon: "⇄",  label: "TCP Socket",    visible: false },
      { type: "RServerSocket", icon: "⇆",  label: "Server Socket", visible: false },
    ],
  },
  {
    name: "I/O",
    items: [
      { type: "RTimer",        icon: "⏱",  label: "Timer",         visible: false },
      { type: "RFileStream",   icon: "▤",  label: "File Stream",   visible: false },
      { type: "RIni",          icon: "⚙",  label: "INI File",      visible: false },
      { type: "RMemoryStream", icon: "▦",  label: "Memory Stream", visible: false },
      { type: "RPrinter",      icon: "🖨", label: "Printer",       visible: false },
      { type: "ROpenDialog",   icon: "📂", label: "Open Dialog",   visible: false },
      { type: "RSaveDialog",   icon: "💾", label: "Save Dialog",   visible: false },
      { type: "RColorDialog",  icon: "🎨", label: "Color Dialog",  visible: false },
      { type: "RFontDialog",   icon: "F",  label: "Font Dialog",   visible: false },
    ],
  },
  {
    name: "Web",
    items: [
      { type: "RWebView",         icon: "🌍", label: "Web View",     visible: true  },
      { type: "RDOM",             icon: "<>", label: "DOM Element",  visible: true  },
      { type: "RWebVideo",        icon: "🎬", label: "Video",        visible: true  },
      { type: "RWebAudio",        icon: "♫",  label: "Audio",        visible: false },
      { type: "RJavaScript",      icon: "JS", label: "JavaScript",   visible: false },
      { type: "RWebStorage",      icon: "💾", label: "Web Storage",  visible: false },
      { type: "RWebNotification", icon: "🔔", label: "Notification", visible: false },
      { type: "RWebGeolocation",  icon: "📍", label: "Geolocation",  visible: false },
      { type: "RRouter",          icon: "↗",  label: "Router",       visible: false },
    ],
  },
];

// Backwards-compat: flat catalog used by older callers.
export const TOOLBOX = TOOLBOX_GROUPS.flatMap(g => g.items);

export function isVisibleType(type) {
  for (const g of TOOLBOX_GROUPS)
    for (const it of g.items)
      if (it.type === type) return it.visible;
  return true;   // unknown types default to visible
}

// Defaults for new widgets. Geometry defaults are filled in at drop-time
// from the click point.
const COMMON = (n) => ({
  left: 0, top: 0, width: 88, height: 24,
  visible: 1, enabled: 1, fontname: "Tahoma", fontsize: 11,
});

// Invisible (tray) widgets get a tiny placeholder geometry.
const TRAY = (n) => ({
  left: 0, top: 0, width: 32, height: 32,
  visible: 0, enabled: 1,
});

export function defaultsFor(type, name) {
  switch (type) {
    // ── Visible controls ─────────────────────────────────────────
    case "RButton":     return { ...COMMON(name), caption: name };
    case "RLabel":      return { ...COMMON(name), height: 18, caption: name };
    case "REdit":       return { ...COMMON(name), text: "" };
    case "RMemo":       return { ...COMMON(name), width: 200, height: 80, text: "" };
    case "RCheckBox":   return { ...COMMON(name), caption: name, checked: 0 };
    case "RRadioBtn":   return { ...COMMON(name), caption: name, checked: 0 };
    case "RComboBox":   return { ...COMMON(name), items: "" };
    case "RListBox":    return { ...COMMON(name), height: 120, items: "" };
    case "RImage":      return { ...COMMON(name), height: 80, picture: "" };
    case "RCanvas":     return { ...COMMON(name), width: 240, height: 160 };
    case "RPanel":      return { ...COMMON(name), width: 160, height: 80 };
    case "RGroupBox":   return { ...COMMON(name), width: 160, height: 80, caption: name };
    case "RProgressBar":return { ...COMMON(name), width: 160, height: 18, value: 0, min: 0, max: 100 };
    case "RTrackBar":   return { ...COMMON(name), width: 160, height: 24, value: 0, min: 0, max: 100, step: 1 };
    case "RScrollBar":  return { ...COMMON(name), width: 160, height: 18, value: 0, min: 0, max: 100 };
    case "RUpDown":     return { ...COMMON(name), width: 80,  height: 22, value: 0, min: 0, max: 100 };
    case "RDateTimePicker": return { ...COMMON(name), width: 160, height: 24 };
    case "RStringGrid": return { ...COMMON(name), width: 280, height: 160, rowcount: 5, colcount: 3 };
    case "RListView":   return { ...COMMON(name), width: 240, height: 160 };
    case "RTreeView":   return { ...COMMON(name), width: 200, height: 160 };
    case "RTabControl": return { ...COMMON(name), width: 280, height: 180 };
    case "RScrollBox":  return { ...COMMON(name), width: 200, height: 120 };
    case "RSplitter":   return { ...COMMON(name), width: 4,   height: 160 };
    case "RCodeEditor": return { ...COMMON(name), width: 320, height: 200, text: "" };
    case "RRichEdit":   return { ...COMMON(name), width: 240, height: 160, text: "" };
    case "RCoolBtn":    return { ...COMMON(name), caption: name };
    case "ROvalBtn":    return { ...COMMON(name), caption: name };
    case "RLine":       return { ...COMMON(name), width: 160, height: 2, color: "#888" };
    case "RDataFrame":  return { ...COMMON(name), width: 280, height: 160, source: "" };
    case "RPlot":       return { ...COMMON(name), width: 280, height: 200, dataset: "" };
    case "RMainMenu":   return { ...COMMON(name), width: 0,   height: 28, caption: "" };
    case "RToolBar":    return { ...COMMON(name), width: 0,   height: 32 };
    case "RStatusBar":  return { ...COMMON(name), width: 0,   height: 24, caption: "" };
    case "RWebView":    return { ...COMMON(name), width: 320, height: 200, url: "about:blank" };
    case "RDOM":        return { ...COMMON(name), width: 160, height: 80, tagname: "div" };
    case "RWebVideo":   return { ...COMMON(name), width: 320, height: 200, src: "" };

    // ── Invisible (tray) controls ────────────────────────────────
    case "RTimer":         return { ...TRAY(name), interval: 1000, enabled: 0 };
    case "RSqlite":        return { ...TRAY(name), database: "" };
    case "RMySql":         return { ...TRAY(name), host: "localhost", database: "", user: "" };
    case "RHttp":          return { ...TRAY(name), url: "" };
    case "RSocket":        return { ...TRAY(name), host: "", port: 0 };
    case "RServerSocket":  return { ...TRAY(name), host: "0.0.0.0", port: 8080 };
    case "RFileStream":    return { ...TRAY(name), filename: "" };
    case "RNum":           return { ...TRAY(name), value: "" };
    case "RJson":          return { ...TRAY(name), value: "" };
    case "RStringList":    return { ...TRAY(name), items: "" };
    case "RWebStorage":    return { ...TRAY(name), key: "" };
    case "RJavaScript":    return { ...TRAY(name) };
    case "RWebNotification": return { ...TRAY(name), title: "" };
    case "RWebGeolocation":  return { ...TRAY(name) };
    case "RRouter":        return { ...TRAY(name) };
    case "RWebAudio":      return { ...TRAY(name), src: "" };
    case "RIni":           return { ...TRAY(name), filename: "" };
    case "RMemoryStream":  return { ...TRAY(name) };
    case "RPrinter":       return { ...TRAY(name) };
    case "ROpenDialog":    return { ...TRAY(name), filter: "All files|*.*" };
    case "RSaveDialog":    return { ...TRAY(name), filter: "All files|*.*" };
    case "RColorDialog":   return { ...TRAY(name) };
    case "RFontDialog":    return { ...TRAY(name) };
    case "RPopupMenu":     return { ...TRAY(name) };

    default:               return { ...COMMON(name) };
  }
}
