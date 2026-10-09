' workspace.inc: RapidR Studio's documents area (docs/studio-wow.md; the
' S-SHELL-2 lane) — documents as tabs, as Xcode, VS Code and Delphi show
' them (no window inside the window): one tab a source file, its close
' button, a dot while it has changes, a middle click closes it, a drag
' along the strip reorders it.
'
'   Design | Code    a file with a form has the switch at the right of its
'                    tab strip (RDOCKMANAGER's AddView): its designer, its
'                    code, or both side by side; F12 toggles (Delphi's;
'                    in the code, on a name, it goes to that name's
'                    definition, VS Code's), F7 the code, Shift+F7 the
'                    designer
'   groups           a tab dragged to a side of the area splits it into
'                    groups (VS Code's editor groups), or Window > Split
'                    Right / Split Down; the splitters between them drag
'   kept             the layout (tool panes, groups, views) and the open
'                    files are kept in Studio's settings and come back
'                    when the project is opened again

DIM SessionProject AS STRING
DIM SessionFiles AS STRING

DECLARE SUB DocumentViewChanged(Name AS STRING, View AS STRING)
DECLARE SUB SaveSoon

DIM LayoutSaver AS RTimer

' (document d's pane: its code editor's name)
FUNCTION DocPane(d AS INTEGER) AS STRING
  Result = "codedoc(" + LTRIM$(STR$(d)) + ")"
END FUNCTION

' The open document a pane or a view is (its code editor or its designer),
' -1 if none.
FUNCTION DocOfPane(Name AS STRING) AS INTEGER
  DIM d AS INTEGER, k AS STRING
  k = LCASE$(Name)
  Result = -1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      IF k = "codedoc(" + LTRIM$(STR$(d)) + ")" OR k = "designdoc(" + LTRIM$(STR$(d)) + ")" THEN
        Result = d
        EXIT FUNCTION
      END IF
    END IF
  NEXT
END FUNCTION

' The documents area: tabs (Studio's one way to show documents; RDOCKMANAGER
' keeps its MDI windows for programs), the views' event, the layout kept.
SUB SetUpDocuments
  LayoutSaver.Interval = 800
  LayoutSaver.Enabled = 0
  LayoutSaver.OnTimer = SaveSoon
  Dock.DocumentMode = "tabs"
  SetUpTemplates
  Dock.OnDocumentView = DocumentViewChanged
  Dock.OnLayoutChange = LayoutChanged
END SUB

' Document d's tab: its code; a form's file gets its designer as a view and
' opens on it (Delphi), the switch saying Design | Code.
SUB AddDocumentTab(d AS INTEGER)
  DIM title AS STRING
  title = FileTitle(DocFile(d))
  IF DocHasForm(d) THEN
    Dock.AddPane(CodeDoc(d), title, "documents", "form")
    Dock.AddView(DocPane(d), DesignDoc(d), "Design")
    Dock.AddView(DocPane(d), CodeDoc(d), "Code")
    DesignDoc(d).OnSelect = DesignPicked
    DocViewNow(d) = "Design"
    DocKind(d) = "design"
  ELSE
    Dock.AddPane(CodeDoc(d), title, "documents", "code")
    DocViewNow(d) = "Code"
    DocKind(d) = "code"
  END IF
END SUB

' (the switch, F7, Shift+F7: what document d shows now)
SUB DocumentViewChanged(Name AS STRING, View AS STRING)
  DIM d AS INTEGER
  d = DocOfPane(Name)
  IF d < 0 THEN EXIT SUB
  DocViewNow(d) = View
  IF View = "Code" THEN DocKind(d) = "code" ELSE DocKind(d) = "design"
  ' (the designer reads what was typed before it changes anything)
  IF View <> "Code" AND DocHasForm(d) <> 0 THEN DesignRead d
  ActiveDoc = d
  UpdateCommands
END SUB

' (side by side: the designer clicked makes Format and Edit the designer's)
SUB DesignPicked(Index AS INTEGER, Sender AS RDesignSurface)
  DIM d AS INTEGER
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocKind(d) <> "design" THEN
    DocKind(d) = "design"
    UpdateCommands
  END IF
END SUB

' (side by side: typing in the code makes Edit the code's)
SUB CodeTyped(d AS INTEGER)
  IF DocViewNow(d) = "Split" AND DocKind(d) <> "code" THEN
    DocKind(d) = "code"
    UpdateCommands
  END IF
END SUB

' Shows view View ("Design", "Code", "Split") of the active document.
SUB ShowView(View AS STRING)
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN EXIT SUB
  Dock.DocumentView(DocPane(d), View)
  IF View = "Code" THEN Dock.FocusPane(DocPane(d))
  IF View = "Design" THEN Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
END SUB

' F12: the form, then its code, as Delphi — and in the code, on a name,
' that name's definition, as VS Code (Edit > Go to Definition); off a name
' (or in a file without a form) the toggle.
SUB ToggleDesign
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  IF DocKind(d) = "code" AND DocViewNow(d) <> "Design" THEN
    IF CodeDoc(d).WordAt(CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn) <> "" OR DocHasForm(d) = 0 THEN
      CodeDoc(d).GotoDefinition
      EXIT SUB
    END IF
  END IF
  IF DocHasForm(d) = 0 THEN EXIT SUB
  IF DocViewNow(d) = "Design" THEN ShowView "Code" ELSE ShowView "Design"
END SUB

' The documents' and the views' commands; True when Id was one.
FUNCTION WorkspaceCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER
  d = ActiveCode
  Result = 1
  SELECT CASE Id
    CASE "view.toggleDesigner": ToggleDesign
    CASE "view.sideBySide": ShowView "Split"
    CASE "view.splitVertically": IF d >= 0 THEN Dock.SplitDocument(DocPane(d), "right")
    CASE "view.splitHorizontally": IF d >= 0 THEN Dock.SplitDocument(DocPane(d), "bottom")
    CASE "view.welcome", "help.examples": ShowWelcome
    CASE "window.nextArea": NextArea
    CASE ELSE
      IF LEFT$(Id, 5) = "open:" THEN
        ' (a test step: open:<file> — a project file opened, as from the tree)
        OpenDocument Proj.Folder + "/" + MID$(Id, 6)
      ELSEIF LEFT$(Id, 5) = "view:" THEN
        ' (view:Design / Code / Split — the switch's segment)
        ShowView MID$(Id, 6)
      ELSE
        Result = SearchCommand(Id)
      END IF
  END SELECT
END FUNCTION

' Document d's changes, before it closes: saved, dropped, or the close
' called off — Save (Enter), Don't Save, Cancel (Escape): prompts.inc. The
' document is brought to the front first: the question is about what shows.
' 0 when the user cancelled.
FUNCTION AskToSave(d AS INTEGER) AS INTEGER
  Result = 1
  IF d < 0 THEN EXIT FUNCTION
  IF DocOpen(d) = 0 OR DocDirty(d) = 0 THEN EXIT FUNCTION
  Dock.ActiveDocument = DocPane(d)
  SELECT CASE AskSave("Do you want to save the changes you made to " + FileTitle(DocFile(d)) + "?", "Your changes will be lost if you don't save them.", 0)
    CASE PROMPT_SAVE
      SaveDocument d
    CASE PROMPT_DONT
      DocDirty(d) = 0
      UpdateModified
    CASE ELSE
      Result = 0
  END SELECT
END FUNCTION

' Every document's changes before Studio closes; 0 when the user cancelled.
' One document changed: its own question. Several: Save All (Enter), Review
' Changes... (each one asked: a Cancel there stops quitting), Discard
' Changes, Cancel (Escape).
FUNCTION AskToSaveAll AS INTEGER
  DIM d AS INTEGER, n AS INTEGER, one AS INTEGER
  Result = 1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN
      n = n + 1
      one = d
    END IF
  NEXT
  IF n = 0 THEN EXIT FUNCTION
  IF n = 1 THEN
    Result = AskToSave(one)
    EXIT FUNCTION
  END IF
  SELECT CASE AskSave("You have " + LTRIM$(STR$(n)) + " documents with unsaved changes. Do you want to save them before quitting?", "Review Changes asks about each one. If you discard them, all your changes will be lost.", 1)
    CASE PROMPT_SAVE
      SaveAll
    CASE PROMPT_DONT
      FOR d = 0 TO DocCount - 1
        DocDirty(d) = 0
      NEXT
      UpdateModified
    CASE PROMPT_REVIEW
      FOR d = 0 TO DocCount - 1
        IF AskToSave(d) = 0 THEN
          Result = 0
          EXIT FUNCTION
        END IF
      NEXT
    CASE ELSE
      Result = 0
  END SELECT
END FUNCTION

' Window > Next Area (F6 in the panes themselves): the project, the
' document, the properties, the output, round.
SUB NextArea
  DIM a AS STRING, k AS INTEGER, i AS INTEGER, areas(4) AS STRING
  areas(0) = "projecttree": areas(1) = "": areas(2) = "inspector": areas(3) = "outputbox"
  IF ActiveCode >= 0 THEN areas(1) = DocPane(ActiveCode)
  a = LCASE$(Dock.ActivePane)
  k = -1
  FOR i = 0 TO 3
    IF areas(i) = a THEN k = i
  NEXT
  FOR i = 1 TO 4
    IF areas((k + i + 4) MOD 4) <> "" THEN
      Dock.FocusPane(areas((k + i + 4) MOD 4))
      EXIT SUB
    END IF
  NEXT
END SUB

' ---- File > New Project: the templates' gallery (docs/studio-wow.md WEL-2) ----------
' A card each (its icon, its name, what it makes; RPROJECT's New writes it):
' a click picks one, a double click (or Enter) makes the project.

DIM TplCard(TPLS) AS RPanel
DIM TplIcon(TPLS) AS RImage
DIM TplBmp(TPLS) AS RBitmap
DIM TplTitle(TPLS) AS RLabel
DIM TplText(TPLS) AS RLabel
DIM TplIconName(TPLS) AS STRING

DECLARE SUB TemplateClicked(Sender AS RPanel)
DECLARE SUB TemplateLabelClicked(Sender AS RLabel)
DECLARE SUB TemplateChosen(Sender AS RLabel)

SUB AddTemplate(i AS INTEGER, Id AS STRING, Icon AS STRING, Title AS STRING, What AS STRING)
  TplId(i) = Id
  TplIconName(i) = Icon
  TplCard(i).Parent = NewDialog
  TplCard(i).BevelOuter = 0
  TplCard(i).Tag = i
  TplCard(i).Cursor = -21
  TplCard(i).AccessibleName = Title + ": " + What
  TplCard(i).OnClick = TemplateClicked
  TplCard(i).OnDblClick = NewCreate
  TplIcon(i).Parent = TplCard(i)
  TplIcon(i).Left = 12: TplIcon(i).Top = 12: TplIcon(i).Width = 24: TplIcon(i).Height = 24
  TplTitle(i).Parent = TplCard(i)
  TplTitle(i).Left = 12: TplTitle(i).Top = 42: TplTitle(i).Width = TPL_W - 20: TplTitle(i).Height = 18
  TplTitle(i).Caption = Title
  TplTitle(i).Tag = i
  TplTitle(i).OnClick = TemplateLabelClicked
  TplTitle(i).OnDblClick = TemplateChosen
  TplText(i).Parent = TplCard(i)
  TplText(i).AutoSize = 0
  TplText(i).WordWrap = 1
  TplText(i).Caption = What
  TplText(i).Left = 12: TplText(i).Top = 62: TplText(i).Width = TPL_W - 20: TplText(i).Height = TPL_H - 66
  TplText(i).Tag = i
  TplText(i).OnClick = TemplateLabelClicked
  TplText(i).OnDblClick = TemplateChosen
END SUB

SUB SetUpTemplates
  AddTemplate 0, "gui", "RFORM", "Form app", "A window to design, with its code beside it."
  AddTemplate 1, "console", "view.terminal", "Console", "PRINT and INPUT in a text window."
  AddTemplate 2, "rapidq", "files/bas", "RapidQ form app", "A .bas with RapidQ's names only, checked as RapidQ."
  AddTemplate 3, "data", "RPLOT", "Data dashboard", "A table in a grid and a chart of it."
  AddTemplate 4, "mdi", "RFORMMDI", "MDI app", "Child windows, File and Window menus."
END SUB

' (the cards in a row from (X, Y) of the dialog)
SUB LayOutTemplates(X AS INTEGER, Y AS INTEGER)
  DIM i AS INTEGER
  FOR i = 0 TO TPLS - 1
    TplCard(i).Left = X + i * (TPL_W + 8): TplCard(i).Top = Y
    TplCard(i).Width = TPL_W: TplCard(i).Height = TPL_H
  NEXT
  TemplatesWidth = TPLS * (TPL_W + 8) - 8
END SUB

' (the picked card stands out; its icons and colours in the current theme)
SUB PickTemplate(i AS INTEGER)
  DIM k AS INTEGER
  TplChosen = i
  FOR k = 0 TO TPLS - 1
    IF k = i THEN TplCard(k).Color = TC("editor.selection") ELSE TplCard(k).Color = TC("card")
    TplTitle(k).Font = CardFont
    TplTitle(k).Font.Color = TC("page.text")
    TplText(k).Font.Color = TC("page.dim")
    TplBmp(k).LoadIcon TplIconName(k), 24
    TplIcon(k).BMPHandle = TplBmp(k)
  NEXT
END SUB

SUB TemplateClicked(Sender AS RPanel)
  PickTemplate Sender.Tag
END SUB

SUB TemplateLabelClicked(Sender AS RLabel)
  PickTemplate Sender.Tag
END SUB

SUB TemplateChosen(Sender AS RLabel)
  PickTemplate Sender.Tag
  NewCreate
END SUB

' The start page as a document again (closed when a project opened).
SUB ShowWelcome
  IF Dock.PaneState("welcome") = "" THEN Dock.AddPane(Welcome, "Welcome", "documents", "list")
  Dock.FocusPane("welcome")
END SUB

' ---- the layout and the open files, kept --------------------------------------------
' Saved a moment after the layout changes (and when Studio closes): the
' layout, the project, its open files by their documents' numbers.

SUB LayoutChanged
  LayoutSaver.Enabled = 0
  LayoutSaver.Enabled = 1
END SUB

SUB SaveSoon
  LayoutSaver.Enabled = 0
  SaveSettings
END SUB

' (the open files: "number<TAB>path" a line)
FUNCTION OpenFilesText AS STRING
  DIM d AS INTEGER, t AS STRING
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN t = t + LTRIM$(STR$(d)) + CHR$(9) + DocFile(d) + CHR$(10)
  NEXT
  Result = t
END FUNCTION

SUB SaveSession
  IF ProjectOpen THEN
    SessionProject = ProjectFile
    SessionFiles = OpenFilesText
  END IF
  Reg.WriteString("SessionProject", SessionProject)
  Reg.WriteString("SessionFiles", SessionFiles)
  ' (the window's place and size, unless maximized: then that)
  IF Studio.WindowState = 2 THEN
    Reg.WriteString("Window", "max")
  ELSEIF Studio.WindowState = 0 THEN
    Reg.WriteString("Window", STR$(Studio.Left) + "," + STR$(Studio.Top) + "," + STR$(Studio.Width) + "," + STR$(Studio.Height))
  END IF
END SUB

SUB LoadSession
  DIM w AS STRING
  IF Reg.ValueExists("SessionProject") THEN SessionProject = Reg.ReadString("SessionProject")
  IF Reg.ValueExists("SessionFiles") THEN SessionFiles = Reg.ReadString("SessionFiles")
  IF Reg.ValueExists("Window") THEN
    w = Reg.ReadString("Window")
    IF w = "max" THEN
      Studio.WindowState = 2
    ELSEIF VAL(FIELD$(w, ",", 3)) >= 640 AND VAL(FIELD$(w, ",", 4)) >= 400 THEN
      Studio.Left = VAL(FIELD$(w, ",", 1)): Studio.Top = VAL(FIELD$(w, ",", 2))
      Studio.Width = VAL(FIELD$(w, ",", 3)): Studio.Height = VAL(FIELD$(w, ",", 4))
    END IF
  END IF
END SUB

' Opening the project Studio last had: its files open again, then the
' layout (groups, views, the active one) as it was — each file's old
' document number turned into its new one in the layout's text.
SUB RestoreSession(Path AS STRING)
  DIM n AS INTEGER, i AS INTEGER, row AS STRING, f AS STRING, d AS INTEGER, lay AS STRING
  IF SessionFiles = "" OR LCASE$(Path) <> LCASE$(SessionProject) THEN EXIT SUB
  n = TALLY(SessionFiles, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(SessionFiles, CHR$(10), i)
    f = FIELD$(row, CHR$(9), 2)
    IF f <> "" AND DocOf(f) < 0 THEN OpenDocument f
  NEXT
  IF SavedLayout = "" THEN EXIT SUB
  lay = SavedLayout
  FOR i = 1 TO n
    row = FIELD$(SessionFiles, CHR$(10), i)
    lay = REPLACESUBSTR$(lay, "codedoc(" + FIELD$(row, CHR$(9), 1) + ")", "codedoc(#" + LTRIM$(STR$(i)) + ")")
  NEXT
  FOR i = 1 TO n
    row = FIELD$(SessionFiles, CHR$(10), i)
    d = DocOf(FIELD$(row, CHR$(9), 2))
    IF d >= 0 THEN lay = REPLACESUBSTR$(lay, "codedoc(#" + LTRIM$(STR$(i)) + ")", DocPane(d))
  NEXT
  Dock.LoadLayout(lay)
  DebugLayoutLoaded
END SUB
