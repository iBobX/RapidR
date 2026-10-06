' documents.inc: opening source files as documents — a code editor, and a
' designer for the form the file creates — and what the side panes show
' about the active one (its outline, its form's properties).

DIM FormName AS STRING
DIM FormType AS STRING
DIM FormCaption AS STRING
DIM PropKey(60) AS STRING
DIM PropValue(60) AS STRING
DIM PropCount AS INTEGER

' (a path's file name)
FUNCTION FileTitle(Path AS STRING) AS STRING
  DIM p AS STRING, i AS INTEGER
  p = REPLACESUBSTR$(Path, "\", "/")
  i = RINSTR(p, "/")
  IF i > 0 THEN Result = MID$(p, i + 1) ELSE Result = p
END FUNCTION

' (a file name without its extension)
FUNCTION BaseName(Path AS STRING) AS STRING
  DIM t AS STRING, i AS INTEGER
  t = FileTitle(Path)
  i = RINSTR(t, ".")
  IF i > 1 THEN Result = LEFT$(t, i - 1) ELSE Result = t
END FUNCTION

' (the value of `Name = value` in a statement, "" if it isn't that property)
FUNCTION Assigned(Stmt AS STRING, Name AS STRING) AS STRING
  DIM s AS STRING, i AS INTEGER
  s = LTRIM$(RTRIM$(Stmt))
  i = INSTR(s, "=")
  IF i < 2 THEN EXIT FUNCTION
  IF UCASE$(RTRIM$(LEFT$(s, i - 1))) = UCASE$(Name) THEN Result = LTRIM$(MID$(s, i + 1))
END FUNCTION

' (a string literal's text, quotes off)
FUNCTION Unquote(v AS STRING) AS STRING
  IF LEFT$(v, 1) = CHR$(34) AND RIGHT$(v, 1) = CHR$(34) AND LEN(v) >= 2 THEN Result = MID$(v, 2, LEN(v) - 2) ELSE Result = v
END FUNCTION

' ---- reading a file's outline and form -------------------------------------------
' (Until RLanguageService: the CREATE blocks, SUBs and FUNCTIONs found line
' by line — a file's form is its first top-level CREATE … AS QFORM.)

DIM OutlineLine(400) AS INTEGER
DIM OutlineCount AS INTEGER

SUB ScanSource(d AS INTEGER)
  DIM n AS INTEGER, i AS INTEGER, s AS STRING, u AS STRING, k AS INTEGER, depth AS INTEGER
  DIM nm AS STRING, ty AS STRING, j AS INTEGER, part AS STRING, parts AS INTEGER
  DIM cx AS INTEGER, cy AS INTEGER, cw AS INTEGER, ch AS INTEGER, cname AS STRING, ctype AS STRING, ccap AS STRING
  DIM inForm AS INTEGER, formDepth AS INTEGER, v AS STRING
  DIM parentNode(20) AS INTEGER
  OutlineTree.Clear
  SymbolIcons.Clear
  SymbolIcons.AddIcon "symbols/sub"
  SymbolIcons.AddIcon "symbols/function"
  SymbolIcons.AddIcon "symbols/type"
  SymbolIcons.AddIcon "symbols/component"
  OutlineCount = 0
  FormName = "": FormType = "": FormCaption = ""
  PropCount = 0
  DesignDoc(d).ClearAll
  cname = ""
  depth = 0
  parentNode(0) = -1
  n = CodeDoc(d).LineCount
  FOR i = 0 TO n - 1
    s = LTRIM$(RTRIM$(CodeDoc(d).Line(i)))
    u = UCASE$(s)
    IF LEFT$(u, 7) = "CREATE " THEN
      ' (a component: CREATE Name AS Type)
      k = INSTR(u, " AS ")
      nm = RTRIM$(MID$(s, 8, k - 8))
      ty = UCASE$(LTRIM$(RTRIM$(MID$(s, k + 4))))
      IF parentNode(depth) < 0 THEN
        OutlineTree.AddItems nm + ": " + ty
      ELSE
        OutlineTree.AddChildItems parentNode(depth), nm + ": " + ty
      END IF
      SymbolIcons.AddIcon ty
      OutlineTree.Item(OutlineTree.ItemCount - 1).ImageIndex = SymbolIcons.Count - 1
      OutlineTree.Item(OutlineTree.ItemCount - 1).SelectedIndex = SymbolIcons.Count - 1
      OutlineLine(OutlineTree.ItemCount - 1) = i
      IF depth < 19 THEN depth = depth + 1
      parentNode(depth) = OutlineTree.ItemCount - 1
      IF FormName = "" AND (ty = "QFORM" OR ty = "RFORM") THEN
        FormName = nm: FormType = ty: inForm = 1: formDepth = depth
        PropKey(0) = "Name": PropValue(0) = nm
        PropKey(1) = "Type": PropValue(1) = ty
        PropCount = 2
      ELSEIF inForm AND depth = formDepth + 1 THEN
        cname = nm: ctype = ty: cx = 8: cy = 8: cw = 80: ch = 25: ccap = ""
      END IF
    ELSEIF LEFT$(u, 10) = "END CREATE" THEN
      IF inForm AND depth = formDepth + 1 AND cname <> "" THEN
        DesignDoc(d).AddComponent ctype, cname, cx, cy, cw, ch
        IF ccap <> "" THEN DesignDoc(d).SetProp DesignDoc(d).Count - 1, "Caption", ccap
        cname = ""
      END IF
      IF inForm AND depth = formDepth THEN inForm = 0
      IF depth > 0 THEN depth = depth - 1
    ELSEIF LEFT$(u, 4) = "SUB " OR LEFT$(u, 9) = "FUNCTION " THEN
      k = INSTR(s, " ")
      nm = LTRIM$(MID$(s, k + 1))
      IF INSTR(nm, "(") > 0 THEN nm = RTRIM$(LEFT$(nm, INSTR(nm, "(") - 1))
      OutlineTree.AddItems nm
      IF LEFT$(u, 4) = "SUB " THEN j = 0 ELSE j = 1
      OutlineTree.Item(OutlineTree.ItemCount - 1).ImageIndex = j
      OutlineTree.Item(OutlineTree.ItemCount - 1).SelectedIndex = j
      OutlineLine(OutlineTree.ItemCount - 1) = i
    ELSEIF LEFT$(u, 5) = "TYPE " THEN
      OutlineTree.AddItems LTRIM$(MID$(s, 6))
      OutlineTree.Item(OutlineTree.ItemCount - 1).ImageIndex = 2
      OutlineTree.Item(OutlineTree.ItemCount - 1).SelectedIndex = 2
      OutlineLine(OutlineTree.ItemCount - 1) = i
    ELSEIF inForm AND s <> "" AND LEFT$(s, 1) <> "'" THEN
      ' (property assignments, several per line with ":")
      parts = TALLY(s, ":") + 1
      FOR j = 1 TO parts
        part = FIELD$(s, ":", j)
        IF depth = formDepth THEN
          k = INSTR(part, "=")
          IF k > 1 AND PropCount < 60 THEN
            PropKey(PropCount) = LTRIM$(RTRIM$(LEFT$(part, k - 1)))
            PropValue(PropCount) = LTRIM$(RTRIM$(MID$(part, k + 1)))
            IF UCASE$(PropKey(PropCount)) = "CAPTION" THEN FormCaption = Unquote(PropValue(PropCount))
            PropCount = PropCount + 1
          END IF
        ELSEIF depth = formDepth + 1 AND cname <> "" THEN
          v = Assigned(part, "Left"): IF v <> "" THEN cx = VAL(v)
          v = Assigned(part, "Top"): IF v <> "" THEN cy = VAL(v)
          v = Assigned(part, "Width"): IF v <> "" THEN cw = VAL(v)
          v = Assigned(part, "Height"): IF v <> "" THEN ch = VAL(v)
          v = Assigned(part, "Caption"): IF v <> "" THEN ccap = Unquote(v)
        END IF
      NEXT
    END IF
  NEXT
  OutlineCount = OutlineTree.ItemCount
  IF FormCaption <> "" THEN DesignDoc(d).FormCaption = FormCaption ELSE DesignDoc(d).FormCaption = FormName
  OutlineTree.FullExpand
END SUB

' ---- opening, activating, closing ---------------------------------------------------

FUNCTION FreeDoc AS INTEGER
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) = 0 THEN
      Result = d
      EXIT FUNCTION
    END IF
  NEXT
  IF DocCount < MAXDOCS THEN
    Result = DocCount
    DocCount = DocCount + 1
  ELSE
    Result = -1
  END IF
END FUNCTION

' Opens a source file: its code, and its form in a designer when it makes one.
SUB OpenSource(Path AS STRING)
  DIM d AS INTEGER, title AS STRING
  d = FreeDoc
  IF d < 0 THEN EXIT SUB
  CodeDoc(d).LoadFromFile Path
  DocFile(d) = Path
  DocKind(d) = "code"
  DocOpen(d) = 1
  ScanSource d
  title = FileTitle(Path)
  IF FormName <> "" THEN
    DesignDoc(d).Width = 640: DesignDoc(d).Height = 420
    Dock.AddPane(DesignDoc(d), title + " [Design]", "documents", "form")
  END IF
  Dock.AddPane(CodeDoc(d), title, "documents", "code")
  ActiveDoc = d
  IF ProjectOpen = 0 THEN
    ProjectOpen = 1
    ProjectName = BaseName(Path)
    ProjectMain = Path
    ' (the start page makes room: the form and its code side by side)
    Dock.ClosePane("welcome")
    Dock.TileVertical
  END IF
  FillProjectTree
  ShowProperties FormName
  OutputLine "Opened " + Path
  UpdateTitle
  UpdateCommands
END SUB
