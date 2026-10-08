' decl.inc: RapidR Studio's shared state — the command table, the open
' documents, the run state — and the SUBs the window's components call
' (declared before the window is made, as RapidQ wants).

' ---- the command table (docs/ide-plan.md I1) -------------------------------
' One row per command: what the menus, the tool bar, the keyboard and the
' command palette all show and run. Shortcut is the key bound in the menus
' (VB6 / Delphi's: F5 Run, Shift+F5 Stop, F9 breakpoint, F8 / F10 / F11
' stepping, Ctrl+S, Ctrl+O, Ctrl+N, Ctrl+Shift+P); Shown is a key the
' focused control keeps (Ctrl+C in an editor), shown in the menu only.
' EnabledWhen names the state the command needs: "" always, "project",
' "document", "code", "design", "idle" (nothing running), "running",
' "paused", "session" (running or paused).
TYPE StudioCommand
  Id AS STRING
  Title AS STRING
  Category AS STRING
  Shortcut AS STRING
  Shown AS STRING
  EnabledWhen AS STRING
  Icon AS STRING
  Item AS INTEGER
  Tool AS INTEGER
END TYPE

CONST MAXCMDS = 200
CONST MAXMENUS = 240
CONST MAXDOCS = 32

DIM Cmd(MAXCMDS) AS StudioCommand
DIM CmdCount AS INTEGER

' The menus: every item (top menus, submenus, commands, separators) is one
' of these, made from the table at start-up.
DIM MI(MAXMENUS) AS QMENUITEM
DIM MICount AS INTEGER
' (the menus by category name, "File", "View/Theme" …, and their item)
DIM MenuName(40) AS STRING
DIM MenuItemOf(40) AS INTEGER
DIM MenuCount AS INTEGER


' ---- documents ---------------------------------------------------------------
' Each open document is a pane of the dock manager's documents area: a code
' editor for a source file, a designer for a file's form, the start page.
DIM CodeDoc(MAXDOCS) AS RCODEEDITOR
DIM DesignDoc(MAXDOCS) AS RDESIGNSURFACE
DIM DocFile(MAXDOCS) AS STRING
DIM DocKind(MAXDOCS) AS STRING
DIM DocOpen(MAXDOCS) AS INTEGER
DIM DocDirty(MAXDOCS) AS INTEGER
DIM DocHasForm(MAXDOCS) AS INTEGER
' (a file being loaded: its editor's OnChange isn't the user's)
DIM Loading AS INTEGER
DIM DocCount AS INTEGER
DIM ActiveDoc AS INTEGER

' ---- what Studio holds now -----------------------------------------------------
DIM HomeDir AS STRING
DIM ProjectName AS STRING
DIM ProjectMain AS STRING
DIM ProjectFile AS STRING
DIM RecentList AS STRING
DIM SavedLayout AS STRING
DIM DefaultFolder AS STRING
DIM RunStarted AS DOUBLE
DIM DoList AS STRING
DIM ProjectOpen AS INTEGER
DIM RunState AS STRING
DIM ThemeName AS STRING
' (what the user chose: "rapidr" — RapidR's look as the system is: light,
' dark or high contrast; the default — "rapidr light", "rapidr dark",
' "rapidr high contrast", "classic"; ThemeName is the theme drawn)
DIM ThemeChoice AS STRING
DIM DocumentMode AS STRING
DIM StateText AS STRING

DECLARE SUB RunCommand(Id AS STRING)
DECLARE SUB LayOutDialogs
DECLARE SUB MenuPicked(Sender AS QMENUITEM)
DECLARE SUB ToolClicked(Name AS STRING, Command AS STRING)
DECLARE SUB StudioResize
DECLARE SUB StudioClose(Action AS INTEGER)
DECLARE SUB PaneChanged(Name AS STRING)
DECLARE SUB DocumentActivated(Name AS STRING)
DECLARE SUB DocumentClosing(Name AS STRING, Cancel AS INTEGER)
DECLARE SUB WelcomeLink(Sender AS QLABEL)
DECLARE SUB ProjectNodeOpen(Path AS STRING)
DECLARE SUB ProjectNodeSelect(Path AS STRING)
DECLARE SUB ProjectNewFile(Path AS STRING, Kind AS STRING)
DECLARE SUB ProjectRename(OldPath AS STRING, NewPath AS STRING, Cancel AS INTEGER)
DECLARE SUB ProjectDelete(Path AS STRING, Cancel AS INTEGER)
DECLARE SUB ProjectMove(Path AS STRING, NewPath AS STRING, Index AS INTEGER)
DECLARE SUB ToolboxPick(TypeName AS STRING)
DECLARE SUB ToolboxSelect(TypeName AS STRING)
DECLARE SUB ToolboxDragStart(TypeName AS STRING)
DECLARE SUB ToolboxDrop(TypeName AS STRING, Target AS STRING, X AS INTEGER, Y AS INTEGER)
DECLARE SUB InspectorChanged(Prop AS STRING, Value AS STRING)
DECLARE SUB InspectorEvent(EventName AS STRING)
DECLARE SUB OutputLink(InFile AS STRING, LineNo AS INTEGER)
DECLARE SUB DesignSelected(Index AS INTEGER)
DECLARE SUB OutlineOpen
DECLARE SUB PaletteCommand(Id AS STRING)
DECLARE SUB CodeChanged(Sender AS RCODEEDITOR)
DECLARE SUB AnalyzerTick
DECLARE SUB ProgramOutput(Text AS STRING)
DECLARE SUB ProgramStopped(Reason AS STRING, InFile AS STRING, LineNo AS INTEGER)
DECLARE SUB ProgramContinued
DECLARE SUB ProgramExited(Code AS INTEGER)
DECLARE SUB ImmediateKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB NewBrowse
DECLARE SUB NewCreate
DECLARE SUB NewCancel
DECLARE SUB CardPicked(Sender AS QPANEL)
DECLARE SUB CardLabelPicked(Sender AS QLABEL)
DECLARE SUB RecentPicked(Sender AS QLABEL)
DECLARE SUB StudioKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB LayOutWelcome
DECLARE SUB MacroTick

' ---- Studio's services: public components (rapidr-studio) ----
DIM Proj AS RPROJECT
DIM Lang AS RLANGUAGESERVICE
DIM Session AS RPROGRAMSESSION
DIM Reg AS QREGISTRY
DIM OpenDlg AS QOPENDIALOG
DIM SaveDlg AS QSAVEDIALOG
