' markdown.inc: Markdown files (.md) in RapidR Studio — a README, the
' RapidQ import's report — shown formatted by default, as GitHub and VS
' Code's preview show them, with their source a click away.
'
'   Preview | Source   a .md file's tab has the switch at the right of its
'                      tab strip, as a form's file has Design | Code: its
'                      text formatted (an RMarkdownView), its source (the
'                      code editor), or both side by side; F12 toggles,
'                      F7 the source, Shift+F7 the preview
'   live               what is typed in the source shows in the preview
'   links              a web link opens in the browser; a link to another
'                      file opens it in Studio (a .md one in Preview); a
'                      #heading link scrolls to that heading
'   outline            the headings, nested; a click shows that heading
'                      (in the preview, or at its line in the source)

' (a Markdown file: .md, .markdown)
FUNCTION IsMarkdownFile(Path AS STRING) AS INTEGER
  SELECT CASE FileExt(Path)
    CASE "md", "markdown": Result = 1
  END SELECT
END FUNCTION

' File > Open (or `rapidr ide README.md`) on a Markdown file: it opens on
' its own, in Preview, with or without a project. True when Path was one.
FUNCTION OpenMarkdownFile(Path AS STRING) AS INTEGER
  IF IsMarkdownFile(Path) = 0 THEN EXIT FUNCTION
  IF FILEEXISTS(Path) = 0 THEN EXIT FUNCTION
  OpenDocument Path
  SetState "stopped", "Ready"
  Result = 1
END FUNCTION

' Document d opened: a Markdown file's preview shows the code editor's
' text, which isn't coloured as BASIC. (Any other file keeps the language
' its name gives the code editor — rapidr-basic for BASIC, with the
' language service: S-EDITOR's ids; "basic" was the old editor's.)
SUB OpenMarkdown(d AS INTEGER)
  IF DocIsMd(d) = 0 THEN EXIT SUB
  CodeDoc(d).Language = "markdown"
  MdDoc(d).Tag = d
  MdDoc(d).AccessibleName = FileTitle(DocFile(d))
  MdDoc(d).OnLinkClick = MarkdownLink
  MdDoc(d).Text = CodeDoc(d).Text
END SUB

' Document d's tab: the preview first (shown), then the source.
SUB AddMarkdownTab(d AS INTEGER)
  Dock.AddPane(CodeDoc(d), FileTitle(DocFile(d)), "documents", "markdown")
  Dock.AddView(DocPane(d), MdDoc(d), "Preview")
  Dock.AddView(DocPane(d), CodeDoc(d), "Source")
  DocViewNow(d) = "Preview"
  DocKind(d) = "preview"
END SUB

' (what document d shows, by the switch's caption)
SUB MarkdownViewShown(d AS INTEGER, View AS STRING)
  DocViewNow(d) = View
  IF View = "Preview" THEN DocKind(d) = "preview" ELSE DocKind(d) = "code"
END SUB

' (a path with its "./" and "folder/../" taken out)
FUNCTION TidyPath(Path AS STRING) AS STRING
  DIM p AS STRING, i AS INTEGER, j AS INTEGER
  p = REPLACESUBSTR$(Path, "\", "/")
  p = REPLACESUBSTR$(p, "/./", "/")
  IF LEFT$(p, 2) = "./" THEN p = MID$(p, 3)
  i = INSTR(p, "/../")
  WHILE i > 1
    j = RINSTR(LEFT$(p, i - 1), "/")
    p = LEFT$(p, j) + MID$(p, i + 4)
    i = INSTR(p, "/../")
  WEND
  Result = p
END FUNCTION

' A link clicked in a preview: another file of the project opens (at its
' heading when the link names one); the view itself scrolled to a
' #heading, and opened a web link in the browser.
SUB MarkdownLink(Url AS STRING, Sender AS RMarkdownView)
  DIM d AS INTEGER, target AS STRING, anchor AS STRING, i AS INTEGER, k AS INTEGER, p AS STRING
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF LEFT$(Url, 1) = "#" THEN EXIT SUB
  IF INSTR(Url, "://") > 0 OR LCASE$(LEFT$(Url, 7)) = "mailto:" THEN
    OutputLine "Opened " + Url + " in the browser"
    EXIT SUB
  END IF
  target = Url
  i = INSTR(target, "#")
  IF i > 0 THEN
    anchor = MID$(target, i)
    target = LEFT$(target, i - 1)
  END IF
  p = REPLACESUBSTR$(DocFile(d), "\", "/")
  k = RINSTR(p, "/")
  IF LEFT$(target, 1) <> "/" AND k > 0 THEN target = LEFT$(p, k) + target
  target = TidyPath(target)
  IF FILEEXISTS(target) = 0 THEN
    OutputLine "Can't open " + Url + ": there is no " + target
    EXIT SUB
  END IF
  OpenDocument target
  k = DocOf(target)
  IF k >= 0 AND anchor <> "" THEN
    IF DocIsMd(k) THEN MdDoc(k).ScrollTo anchor
  END IF
END SUB

' Document d's headings as the outline (OutlineLine: each one's line in
' the source).
SUB MarkdownOutline(d AS INTEGER)
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, depth AS INTEGER, img AS INTEGER, top AS INTEGER
  DIM parentNode(7) AS INTEGER
  OutlineTree.Clear
  SymbolIcons.Clear
  OutlineCount = 0
  text = MdDoc(d).Headings
  n = TALLY(text, CHR$(10))
  SymbolIcons.AddIcon "symbols/module"
  img = SymbolIcons.Count - 1
  top = 9
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    IF VAL(FIELD$(row, CHR$(9), 1)) < top THEN top = VAL(FIELD$(row, CHR$(9), 1))
  NEXT
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    depth = VAL(FIELD$(row, CHR$(9), 1)) - top
    IF depth > 5 THEN depth = 5
    ' (a level skipped: under the nearest heading above it)
    WHILE depth > 0 AND parentNode(depth - 1) < 0
      depth = depth - 1
    WEND
    IF depth = 0 OR OutlineTree.ItemCount = 0 THEN
      OutlineTree.AddItems FIELD$(row, CHR$(9), 2)
      depth = 0
    ELSE
      OutlineTree.AddChildItems parentNode(depth - 1), FIELD$(row, CHR$(9), 2)
    END IF
    OutlineTree.Item(OutlineTree.ItemCount - 1).ImageIndex = img
    OutlineTree.Item(OutlineTree.ItemCount - 1).SelectedIndex = img
    parentNode(depth) = OutlineTree.ItemCount - 1
    IF depth < 5 THEN parentNode(depth + 1) = -1
    IF OutlineTree.ItemCount <= 400 THEN OutlineLine(OutlineTree.ItemCount - 1) = VAL(FIELD$(row, CHR$(9), 4)) - 1
  NEXT
  OutlineCount = OutlineTree.ItemCount
  OutlineTree.FullExpand
END SUB

' An outline row clicked on a Markdown document's: the preview shows that
' heading (the source shows its line, as for code). True when it did.
FUNCTION MarkdownOutlineOpen(d AS INTEGER, i AS INTEGER) AS INTEGER
  DIM text AS STRING
  IF DocIsMd(d) = 0 THEN EXIT FUNCTION
  IF DocViewNow(d) <> "Preview" THEN EXIT FUNCTION
  text = MdDoc(d).Headings
  MdDoc(d).ScrollTo FIELD$(FIELD$(text, CHR$(10), i + 1), CHR$(9), 3)
  Result = 1
END FUNCTION

' The View and Edit commands on a Markdown document (the switch's
' Preview | Source in place of Design | Code); True when Id was one.
FUNCTION MarkdownCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER, v AS STRING
  d = ActiveCode
  IF d < 0 THEN EXIT FUNCTION
  IF DocIsMd(d) = 0 THEN EXIT FUNCTION
  Result = 1
  SELECT CASE Id
    CASE "view.code", "view:Code", "view:Source": v = "Source"
    CASE "view.designer", "view:Design", "view:Preview": v = "Preview"
    CASE "view.toggleDesigner"
      IF DocViewNow(d) = "Preview" THEN v = "Source" ELSE v = "Preview"
    CASE "view.sideBySide", "view:Split": v = "Split"
    CASE "edit.copy"
      IF DocKind(d) = "preview" THEN Clipboard.Text = MdDoc(d).SelText ELSE Result = 0
    CASE "edit.selectAll"
      IF DocKind(d) = "preview" THEN MdDoc(d).SelectAll ELSE Result = 0
    CASE ELSE
      Result = 0
  END SELECT
  IF v = "" THEN EXIT FUNCTION
  Dock.DocumentView(DocPane(d), v)
  IF v = "Source" THEN Dock.FocusPane(DocPane(d))
  IF v = "Preview" THEN Dock.FocusPane("mddoc(" + LTRIM$(STR$(d)) + ")")
END FUNCTION
