' decl.inc: RapidR Studio's shared state — the command table, the open
' documents, the run state — and the SUBs the window's components call
' (declared before the window is made, as RapidQ wants).

' ---- the command table (docs/ide-plan.md I1) -------------------------------
' One row per command: what the menus, the tool bar, the keyboard and the
' command palette all show and run. Shortcut is the key bound in the menus
' (the usual keys: F5 Run, Shift+F5 Stop, F9 breakpoint, F8 / F10 / F11
' stepping, Ctrl+S, Ctrl+O, Ctrl+N, Ctrl+Shift+P); Shown is a key the
' focused control keeps (Ctrl+C in an editor), shown in the menu only.
' EnabledWhen names the state the command needs: "" always, "project",
' "document", "code", "design", "idle" (nothing running or paused),
' "startable" (Run / Continue: idle or paused), "steppable" (F10 / F11:
' idle or paused), "cursor" (code, not running), "running", "paused",
' "pausedcode", "session" (running or paused).
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

CONST MAXCMDS = 240
CONST MAXMENUS = 280
CONST MAXDOCS = 32

DIM Cmd(MAXCMDS) AS StudioCommand
DIM CmdCount AS INTEGER
' (1 once the Output and Problems pages were told not to wrap long lines: panels.inc)
DIM OutputNoWrap AS INTEGER

' The menus: every item (top menus, submenus, commands, separators) is one
' of these, made from the table at start-up.
DIM MI(MAXMENUS) AS RMenuItem
DIM MICount AS INTEGER
' (the menus by category name, "File", "View/Theme" …, and their item)
DIM MenuName(40) AS STRING
DIM MenuItemOf(40) AS INTEGER
DIM MenuCount AS INTEGER


' ---- documents ---------------------------------------------------------------
' Each open document is a pane of the dock manager's documents area: a code
' editor for a source file, a designer for a file's form, the start page.
DIM CodeDoc(MAXDOCS) AS RCodeEditor
DIM DesignDoc(MAXDOCS) AS RDesignSurface
DIM DocFile(MAXDOCS) AS STRING
DIM DocKind(MAXDOCS) AS STRING
DIM DocOpen(MAXDOCS) AS INTEGER
DIM DocDirty(MAXDOCS) AS INTEGER
DIM DocHasForm(MAXDOCS) AS INTEGER
' (which view a form's file shows: "Design", "Code" or "Split" — workspace.inc)
DIM DocViewNow(MAXDOCS) AS STRING
' (a file being loaded: its editor's OnChange isn't the user's)
DIM Loading AS INTEGER
DIM DocCount AS INTEGER
DIM ActiveDoc AS INTEGER
' (Markdown files, markdown.inc: formatted — Preview — beside their source,
' as a form's file has its designer beside its code)
DIM MdDoc(MAXDOCS) AS RMarkdownView
DIM DocIsMd(MAXDOCS) AS INTEGER

' ---- what Studio holds now -----------------------------------------------------
DIM HomeDir AS STRING
DIM ProjectName AS STRING
DIM ProjectMain AS STRING
DIM ProjectFile AS STRING
' (the open project is the user's copy of this example, installed where it
' can't be changed: project.inc; "" otherwise)
DIM ExampleOriginal AS STRING
DIM ExampleJustCopied AS STRING
DECLARE FUNCTION AskCopyAgain(Question AS STRING, Detail AS STRING) AS INTEGER
DECLARE FUNCTION AskOpenItsFolder(Question AS STRING, Detail AS STRING) AS INTEGER
DECLARE FUNCTION ProjectFilesElsewhere(Path AS STRING) AS INTEGER
DIM RecentList AS STRING
' (S2) at start-up, offer to reopen the last project and its files (a
' setting, on unless turned off: Tools > Settings > General)
DIM ReopenLast AS INTEGER
' (the code's helpers: off, quiet, helpful — the default — or everything;
' Tools > Settings, editor.inc)
DIM EditorHelpers AS STRING
DIM ReopenPath AS STRING
CONST MAXRECENT = 10
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
DIM StateText AS STRING
' (the status bar's text measured: a long file name shortened in its middle)
DIM StatusMeasure AS RBitmap
DECLARE FUNCTION FitMiddle(Text AS STRING, Room AS INTEGER) AS STRING
DECLARE FUNCTION StateRoom AS INTEGER
DECLARE SUB ShowStatusHint

DECLARE SUB RunCommand(Id AS STRING)
DECLARE SUB LayOutDialogs
DECLARE SUB MenuPicked(Sender AS RMenuItem)
DECLARE SUB ToolClicked(Name AS STRING, Command AS STRING)
DECLARE SUB StudioResize
DECLARE SUB StudioShown
DECLARE SUB StudioClose(Action AS INTEGER)
DECLARE SUB PaneChanged(Name AS STRING)
DECLARE SUB DocumentActivated(Name AS STRING)
DECLARE SUB DocumentClosing(Name AS STRING, Cancel AS INTEGER)
DECLARE SUB WelcomeLink(Sender AS RLabel)
DECLARE SUB StartRowPicked(Sender AS RPanel)
DECLARE SUB StartAction(Which AS INTEGER)
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
' (the project's properties in the Properties pane: panels.inc)
DIM ProjectInspecting AS INTEGER
DIM ProjectInspSig AS STRING
' (the main file's document the look was just written into, plus one:
' opened for it, it is the active tab but the project's properties stay)
DIM ProjectKeepDoc AS INTEGER
' (while that document opens: its tab and its designer say nothing yet)
DIM ProjectOpening AS INTEGER
DECLARE SUB ShowProjectInspector
DECLARE SUB LeaveProjectInspector
DECLARE SUB FillProjectInspector
DECLARE SUB ProjectPropChanged(Prop AS STRING, Value AS STRING)
DECLARE SUB RefreshProgramLook(ReadDisk AS INTEGER)
DECLARE SUB ApplyDesignerSettings
DECLARE SUB InspectorEvent(EventName AS STRING)
DECLARE SUB OutputLink(InFile AS STRING, LineNo AS INTEGER)
DECLARE SUB DesignSelected(Index AS INTEGER)
DECLARE SUB DesignSourceEdit(StartLine AS INTEGER, StartCol AS INTEGER, EndLine AS INTEGER, EndCol AS INTEGER, Text AS STRING, Sender AS RDesignSurface)
DECLARE SUB DesignChanged(Sender AS RDesignSurface)
DECLARE SUB DesignDblClick(Index AS INTEGER, Sender AS RDesignSurface)
DECLARE SUB DesignRead(d AS INTEGER)
DECLARE SUB CaretAtHandler(d AS INTEGER, L AS INTEGER)
DECLARE SUB DesignNames(d AS INTEGER)
' (the toolbox's form being added: "form" / "formmdi", the hint after it — panels.inc)
DIM PendingFormKind AS STRING
DIM PendingFormHint AS STRING
DIM PendingFormAt AS DOUBLE
DECLARE FUNCTION IsFormType(TypeName AS STRING) AS INTEGER
DECLARE SUB AddFormOfType(TypeName AS STRING, Hint AS STRING)
DECLARE SUB IncludeFollows(OldPath AS STRING, NewPath AS STRING)
DECLARE SUB IncludeLeaves(Path AS STRING)
DECLARE SUB FillProblemsOf(Path AS STRING)
DECLARE FUNCTION InProject(Path AS STRING) AS INTEGER
DECLARE FUNCTION ProblemsText(Path AS STRING) AS STRING
DECLARE FUNCTION ProjectRel(Path AS STRING) AS STRING
DECLARE FUNCTION FileIconName(Path AS STRING) AS STRING
DECLARE FUNCTION FileKindNow(Path AS STRING, Listed AS STRING) AS STRING
DECLARE SUB AddFileHeading(InFile AS STRING, Counts AS STRING, Total AS INTEGER)
DECLARE FUNCTION ProgramErrors AS INTEGER
DECLARE SUB DesignPicked(Index AS INTEGER, Sender AS RDesignSurface)
DECLARE SUB DesignStep(Continues AS INTEGER, Sender AS RDesignSurface)
DECLARE SUB DesignUndo(Redo AS INTEGER, Sender AS RDesignSurface)
DECLARE SUB FileUndo(d AS INTEGER, Redo AS INTEGER)
DECLARE FUNCTION DesignCommand(Id AS STRING) AS INTEGER
DECLARE SUB OutlineOpen
DECLARE SUB PaletteCommand(Id AS STRING)
DECLARE SUB CodeChanged(Sender AS RCodeEditor)
DECLARE SUB AnalyzerTick
DECLARE SUB ProgramOutput(Text AS STRING)
DECLARE SUB ProgramStopped(Reason AS STRING, InFile AS STRING, LineNo AS INTEGER)
DECLARE SUB ProgramContinued
DECLARE SUB ProgramExited(Code AS INTEGER)
DECLARE SUB ImmediateKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE FUNCTION DebugCommand(Id AS STRING) AS INTEGER
DECLARE FUNCTION DebugStep(Stp AS STRING) AS INTEGER
DECLARE SUB DebugDocOpened(d AS INTEGER)
DECLARE SUB SetUpDebug
DECLARE SUB ApplyDebugColors
DECLARE SUB ApplyDebugFonts(Rows AS INTEGER)
DECLARE SUB PerfReadyTick
DECLARE SUB StepRaiseTick
DECLARE SUB StaleTick
DECLARE SUB RaiseRetryTick
DECLARE SUB ProgramFormShown(Id AS STRING)
DECLARE SUB StopProgram
DECLARE SUB CodeGutterClick(LineNo AS INTEGER, Area AS STRING, Sender AS RCodeEditor)
DECLARE SUB CodeHoverRequest(LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCodeEditor)
DECLARE SUB SessionVariables(Ref AS INTEGER)
DECLARE SUB SessionEvaluated(Id AS INTEGER, Answer AS STRING)
DECLARE SUB VarsExpanding(Index AS INTEGER, AllowExpansion AS INTEGER)
DECLARE SUB VarsCollapsed(Index AS INTEGER)
DECLARE SUB WatchExpanding(Index AS INTEGER, AllowExpansion AS INTEGER)
DECLARE SUB WatchCollapsed(Index AS INTEGER)
DECLARE SUB WatchEditKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB WatchTreeKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB VarsEditing(Index AS INTEGER, AllowEdit AS INTEGER)
DECLARE SUB VarsEdited(Index AS INTEGER, S AS STRING)
DECLARE SUB WatchEditing(Index AS INTEGER, AllowEdit AS INTEGER)
DECLARE SUB WatchEdited(Index AS INTEGER, S AS STRING)
DECLARE SUB StackPicked
DECLARE SUB BpTreeClick
DECLARE SUB BpTreeOpen
DECLARE SUB BpTreeKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB BpEditKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB BpOnClicked
DECLARE SUB NewBrowse
DECLARE SUB NewCreate
DECLARE SUB NewCancel
DECLARE SUB CardPicked(Sender AS RPanel)
DECLARE SUB CardLabelPicked(Sender AS RLabel)
DECLARE SUB RecentPicked(Sender AS RLabel)
DECLARE SUB StudioKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB LayOutWelcome
DECLARE SUB MacroTick
DECLARE SUB CodeCaretMoved(Line AS INTEGER, Column AS INTEGER)
DECLARE SUB EditorFacts(d AS INTEGER)
DECLARE SUB FocusCodeSoon(d AS INTEGER)
DECLARE SUB DesignerSourceEdit(d AS INTEGER, StartLine AS INTEGER, StartCol AS INTEGER, EndLine AS INTEGER, EndCol AS INTEGER, Text AS STRING)
DECLARE SUB EditorFocusTick
DECLARE SUB EditorSettings(d AS INTEGER)
DECLARE SUB EditorProjectFiles(d AS INTEGER)
DECLARE SUB ShowPausedValues
DECLARE FUNCTION PausedRows(Ref AS INTEGER) AS STRING
DECLARE SUB CodeNavigate(File AS STRING, LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCodeEditor)
DECLARE SUB CodeReferences(Json AS STRING, Sender AS RCodeEditor)
DECLARE SUB CodeFileEdits(File AS STRING, Json AS STRING, Sender AS RCodeEditor)
DECLARE FUNCTION CodeDocOf(Path AS STRING) AS INTEGER
DECLARE FUNCTION JsonField(Item AS STRING, Key AS STRING) AS STRING
DECLARE FUNCTION PatchText(Text AS STRING) AS STRING
DECLARE FUNCTION EditorCommand(Id AS STRING) AS INTEGER
DECLARE FUNCTION EditorStep(Id AS STRING) AS INTEGER
' (workspace.inc, search.inc)
DECLARE FUNCTION DocPane(d AS INTEGER) AS STRING
DECLARE SUB AddDocumentTab(d AS INTEGER)
DECLARE SUB CodeTyped(d AS INTEGER)
DECLARE SUB SetUpDocuments
DECLARE SUB SetUpSearch
DECLARE SUB ApplySearchChrome
' (search.inc: Go to Definition and Back, Find References in the Search tab,
' the code editors' context menu)
DECLARE SUB GoToDefinition(d AS INTEGER)
DECLARE SUB NavCaretMoved(L AS INTEGER, C AS INTEGER)
DECLARE SUB NavigateTo(File AS STRING, LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCodeEditor)
DECLARE SUB ShowReferences(Json AS STRING, Name AS STRING)
DECLARE SUB CodeMenuFor(d AS INTEGER)
DECLARE SUB ShowWelcome
' (markdown.inc)
DECLARE FUNCTION IsMarkdownFile(Path AS STRING) AS INTEGER
DECLARE SUB OpenMarkdown(d AS INTEGER)
DECLARE SUB AddMarkdownTab(d AS INTEGER)
DECLARE SUB MarkdownViewShown(d AS INTEGER, View AS STRING)
DECLARE SUB MarkdownLink(Url AS STRING, Sender AS RMarkdownView)
DECLARE SUB MarkdownOutline(d AS INTEGER)
DECLARE FUNCTION MarkdownOutlineOpen(d AS INTEGER, i AS INTEGER) AS INTEGER
DECLARE FUNCTION MarkdownCommand(Id AS STRING) AS INTEGER
DECLARE FUNCTION OpenMarkdownFile(Path AS STRING) AS INTEGER
' (File > New Project's template gallery: workspace.inc)
CONST TPLS = 5
CONST TPL_W = 150
CONST TPL_H = 120
DIM TplId(TPLS) AS STRING
DIM TplChosen AS INTEGER
DIM TemplatesWidth AS INTEGER
DECLARE SUB LayOutTemplates(X AS INTEGER, Y AS INTEGER)
DECLARE SUB PickTemplate(i AS INTEGER)
DECLARE FUNCTION AskToSave(d AS INTEGER) AS INTEGER
DECLARE FUNCTION AskToSaveAll AS INTEGER
' (prompts.inc)
DECLARE FUNCTION AskSave(Question AS STRING, Detail AS STRING, Many AS INTEGER) AS INTEGER
DECLARE SUB UpdateModified
DECLARE SUB SaveSession
DECLARE SUB LayoutChanged
DECLARE SUB SaveDebugState
DECLARE SUB LoadDebugState
DECLARE SUB DebugStateChanged
DECLARE SUB DebugLayoutLoaded
' (the editing and the debugging layout: debug.inc; Settings > General's
' Debug panes — "auto", "always", "never" — and each layout's sizes,
' kept with the settings: project.inc)
DIM DebugPanesMode AS STRING
DIM EditBottom AS INTEGER
DIM DebugBottom AS INTEGER
DIM DebugWidth AS INTEGER
DECLARE SUB RememberLayoutSizes
DECLARE SUB LayoutNoteGone
DECLARE SUB LayOutStatus
DECLARE SUB DebugLayoutRunStarts
DECLARE SUB DebugLayoutPaused
DECLARE SUB DebugLayoutRunEnds
DECLARE SUB ToggleDebugPanes
DECLARE SUB NeedDebugPanes
DECLARE SUB SetDebugPanesMode(Mode AS STRING)
DECLARE SUB ImmAnswer(Text AS STRING)
DECLARE SUB BreakpointPlaced(InFile AS STRING, LineNo AS INTEGER, NewLine AS INTEGER)
' (Studio's settings were read: not --fresh)
DIM SettingsRead AS INTEGER
' (the project's breakpoints and watches being restored: nothing to save)
DIM DebugLoading AS INTEGER
DECLARE SUB LoadSession
DECLARE SUB RestoreSession(Path AS STRING)
' (S1, S2: project.inc, documents.inc, panes.inc)
DECLARE SUB ProjectFollowsCode
DECLARE SUB OpenRecent(k AS INTEGER)
DECLARE SUB ClearRecent
DECLARE SUB ForgetRecent(Path AS STRING)
DECLARE FUNCTION ReopenCandidate AS STRING
DECLARE SUB ReopenLastProject
DECLARE SUB ShowReopen
DECLARE SUB HideReopen
DECLARE FUNCTION WorkspaceCommand(Id AS STRING) AS INTEGER
DECLARE FUNCTION SearchCommand(Id AS STRING) AS INTEGER

' ---- the AI assistant (ai.inc; settings.inc's AI tab) ----
' The assistant is an RAI (the component programs have) with the project
' as its tools; DesignAI lists an RAI-on-a-form's models in the designer.
DIM AI AS RAI
DIM AITools AS RAIProjectTools
DIM DesignAI AS RAI
' (Tools > Settings > AI's own: what the tab tries — a provider's list, a
' key typed — never touches the assistant until OK)
DIM SetAI AS RAI
' Tools > Settings > AI's choices (never a key: the keychain keeps those)
DIM AIOn AS INTEGER
DIM AIProvider AS STRING
DIM AIBase AS STRING
DIM AIWorkspace AS STRING
DIM AIMaxTokens AS INTEGER
DIM AIPermission AS STRING
' ("Ask before the assistant creates or changes files": a new program
' previewed before it is created; off — the default — it opens at once,
' with a Run it / Undo banner: docs/ai-ux-plan.md)
DIM AIReviewFirst AS INTEGER
DIM AIPriceIn AS DOUBLE
DIM AIPriceOut AS DOUBLE
' (the model chosen for each provider: "id<TAB>model" lines)
DIM AIModels AS STRING
DIM AIProvIds(8) AS STRING
DIM AIProvNames(8) AS STRING
DIM AIProvCount AS INTEGER
DECLARE SUB SetUpAI
DECLARE SUB LayOutAIPanel
DECLARE SUB AIPanelFonts
DECLARE FUNCTION AIThousands(N AS INTEGER) AS STRING
DECLARE SUB AIApplySettings
DECLARE FUNCTION AIModelOf(Id AS STRING) AS STRING
DECLARE SUB AISetModelOf(Id AS STRING, Model AS STRING)
DECLARE SUB ShowWhatWasSent
DECLARE SUB SetAIOn(IsOn AS INTEGER)
DECLARE SUB ShowAIPanel
DECLARE SUB LoadAISettings
DECLARE SUB SaveAISettings
DECLARE SUB ShowSettingsPage(k AS INTEGER)
DECLARE SUB ShowAISetup
DECLARE SUB AIEnableSend
DECLARE FUNCTION AIModelItem(Line AS STRING) AS STRING
DECLARE FUNCTION AIModelId(Text AS STRING) AS STRING
DECLARE FUNCTION AICommand(Id AS STRING) AS INTEGER
DECLARE FUNCTION AIReady AS INTEGER
DECLARE FUNCTION AIKeySourceOf(Id AS STRING) AS STRING
DECLARE SUB AIShowOrHide
DECLARE FUNCTION DesignAIChanged(Prop AS STRING, Value AS STRING) AS INTEGER
DECLARE SUB AIStep(Stp AS STRING)
DECLARE SUB InspectorRowSelected(Prop AS STRING)
' (the new program's banner: its focus after the editor's, gone when
' another project opens)
DECLARE FUNCTION AIBannerTakesFocus AS INTEGER
DECLARE SUB AIProjectChanged

' ---- Studio's services: public components (rapidr-studio) ----
DIM Proj AS RProject
DIM Lang AS RLanguageService
DIM Session AS RProgramSession
DIM Reg AS RRegistry
DIM OpenDlg AS ROpenDialog
' (File > Import RapidQ Project or File: several files may be picked — in
' a browser the program and its include files, read in together)
DIM ImportDlg AS RFileDialog
DIM SaveDlg AS RSaveDialog
