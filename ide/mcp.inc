' mcp.inc: AI apps working in Studio — Studio's own MCP server (RMcpServer),
' so an AI app on this computer (any MCP client) can use the real Studio as
' the assistant does, while the user watches (docs/manual/ai-apps.md).
'
' Off by default: Tools > Settings > AI Apps turns it on (a first-time
' confirmation says what AI apps can see; Cancel is the default), on
' 127.0.0.1 only, with a token kept in the system's keychain (Reset Token
' makes a new one). Every call shows in the AI Apps pane (Tools > AI Apps
' Activity) and the status bar as it happens, goes to the audit log the
' assistant uses, and is checked against the permission mode — one choice,
' in the pane's header and in Settings, remembered per project:
'
'   Read only            the AI app looks (reads, checks, describes,
'                        searches, the outline, problems, the output, a
'                        picture of Studio, the layout); a change, a run or
'                        looking at or using the running program is
'                        refused, said in plain words
'   Ask before changes   (the default) looking at the project is free; each
'                        change, run, look at the running program or
'                        question to RAI waits in the pane: Allow Once,
'                        Allow for This Session, Deny (Review for a change);
'                        the running program is asked about once a run
'   Allow edits          changes to files are made at once (each one undo
'                        step, listed, in the audit log); a run, the running
'                        program or RAI asks
'   Allow everything     nothing asks: a warning badge in the pane and the
'                        status bar, a confirmation when chosen; still
'                        listed and logged, Stop still there
'
' Remembered per project: a project an AI app creates or opens takes the
' mode of the project open before (said in the pane and the status bar),
' unless it has its own — except Allow everything, which is never passed
' on: that new project starts with Allow edits.
'
' In every mode: never a key or a secret, never a file outside the open
' project or hidden in it, and a project opened from outside the Projects
' folder, RapidR's examples and the recent list is always asked first.
' Stop turns the server off: every AI app is disconnected. (RAI's own
' permissions — Settings > AI — should take the same four words: a
' follow-up, docs/manual/ai-apps.md.)
'
' The tools are the assistant's (RAIProjectTools: one implementation —
' McpTools is Studio's second one, its own proposals) plus Studio's:
' studio_status, list_documents, open_project, get_problems, run_program,
' read_output, ask_rai, get_rai_steps, get_rai_transcript, get_rai_audit.
'
' Test steps (only with RAPIDR_TEST_MCP_TOKEN, which stands in for the
' keychain's token): mcp:on[=port], mcp:off, mcp:allow=change|run|rai,
' mcp:mode=<read|ask|edits|all> (no confirmation),
' mcp:decide=<once|session|deny>, mcp:settings, mcp:pane.

DECLARE SUB McpToolCalled(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER)
DECLARE SUB McpClientIn(Client AS STRING)
DECLARE SUB McpClientOut(Client AS STRING)
DECLARE SUB McpTickTimer(Sender AS RTimer)
DECLARE SUB McpStopClick
DECLARE SUB McpOnceClick
DECLARE SUB McpSessionClick
DECLARE SUB McpDenyClick
DECLARE SUB McpReviewClick
DECLARE SUB McpSetOnClick
DECLARE SUB McpCopyAddressClick
DECLARE SUB McpCopyHeaderClick
DECLARE SUB McpShowTokenClick
DECLARE SUB McpResetTokenClick
DECLARE SUB McpClearLogClick
DECLARE SUB McpRevApply
DECLARE SUB McpRevReject
DECLARE SUB McpRevPicked
DECLARE FUNCTION McpKindOf(Tool AS STRING) AS STRING
DECLARE FUNCTION McpModeName(Mode AS STRING) AS STRING
DECLARE FUNCTION McpModeIndex(Mode AS STRING) AS INTEGER
DECLARE FUNCTION McpModeAt(Index AS INTEGER) AS STRING
DECLARE FUNCTION McpModeNote(Mode AS STRING) AS STRING
DECLARE FUNCTION McpModeNameEn(Mode AS STRING) AS STRING
DECLARE FUNCTION McpModeNoteEn(Mode AS STRING) AS STRING
DECLARE SUB McpFillModes(Box AS RComboBox)
DECLARE FUNCTION McpOutcomeShown(Outcome AS STRING) AS STRING
DECLARE FUNCTION McpDialogShown(Dialog AS STRING) AS STRING
DECLARE SUB McpLanguageChanged
DECLARE FUNCTION McpDecision(Kind AS STRING) AS STRING
DECLARE SUB McpModeFollow
DECLARE FUNCTION McpModeKept(Key AS STRING) AS STRING
DECLARE SUB McpSetMode(Mode AS STRING)
DECLARE FUNCTION McpConfirmAll AS INTEGER
DECLARE FUNCTION McpConsentOk AS INTEGER
DECLARE FUNCTION McpModeIsKept(Key AS STRING) AS INTEGER
DECLARE SUB McpCarryMode(Before AS STRING, BeforeKey AS STRING)
DECLARE SUB McpKeepConsent
DECLARE SUB McpReaskTick
DECLARE SUB McpShowMode
DECLARE SUB McpModePicked
DECLARE SUB McpPageModePicked
DECLARE SUB McpLoadSettings
DECLARE SUB McpSaveSettings
DECLARE FUNCTION McpTokenNow AS STRING
DECLARE FUNCTION McpUrl AS STRING
DECLARE FUNCTION McpRowIcon(K AS INTEGER) AS STRING
DECLARE FUNCTION McpRowHead(K AS INTEGER) AS STRING
DECLARE FUNCTION McpRowBody(K AS INTEGER) AS STRING
DECLARE FUNCTION McpFootText AS STRING
DECLARE FUNCTION McpManualUrl AS STRING
DECLARE SUB McpLearnClick
DECLARE FUNCTION McpRowsHigh(W AS INTEGER) AS INTEGER
DECLARE FUNCTION McpHeadW AS INTEGER
DECLARE FUNCTION McpConsent AS INTEGER
DECLARE SUB McpLogMenuOpening
DECLARE SUB McpLogMenuPicked(Sender AS RMenuItem)
DECLARE SUB McpLogCopy(All AS INTEGER)
DECLARE FUNCTION McpPaneCommand(Id AS STRING) AS INTEGER
DECLARE SUB McpStart
DECLARE SUB McpStopServer
DECLARE SUB McpSyncTools
DECLARE SUB McpSyncProject
DECLARE SUB McpLayOutPane
DECLARE FUNCTION McpModeNoteHigh(W AS INTEGER) AS INTEGER
DECLARE FUNCTION McpOnNoteHigh(W AS INTEGER) AS INTEGER
DECLARE SUB McpModeNoteColour
DECLARE SUB McpShowState
DECLARE SUB McpStatusShow
DECLARE SUB McpLogLine(Client AS STRING, Tool AS STRING, What AS STRING, Target AS STRING, Outcome AS STRING)
DECLARE SUB McpPaneShow
DECLARE SUB McpProposeThen(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING, Perm AS STRING)
DECLARE SUB McpWait(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, Kind AS STRING, What AS STRING)
DECLARE SUB McpShowAsk
DECLARE SUB McpDecide(Choice AS INTEGER)
DECLARE FUNCTION McpApply(Said AS STRING) AS STRING
DECLARE SUB McpRejectAll
DECLARE SUB McpRun(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE FUNCTION McpShownPath(Path AS STRING) AS STRING
DECLARE FUNCTION McpStatusText AS STRING
DECLARE FUNCTION McpRel(Path AS STRING) AS STRING
DECLARE FUNCTION McpDocumentsText AS STRING
DECLARE FUNCTION McpFullPath(Path AS STRING) AS STRING
DECLARE FUNCTION McpInside(Path AS STRING, Folder AS STRING) AS INTEGER
DECLARE FUNCTION McpOpenAllowed(Path AS STRING) AS INTEGER
DECLARE FUNCTION McpDirtyFiles AS STRING
DECLARE FUNCTION McpOpenProject(Path AS STRING) AS STRING
DECLARE FUNCTION McpOpenCheck(Path AS STRING) AS STRING
DECLARE FUNCTION McpRunWords AS STRING
DECLARE FUNCTION McpOutputFrom(At AS LONG) AS STRING
DECLARE FUNCTION McpReadOutput(All AS INTEGER) AS STRING
DECLARE SUB McpRunProgram(CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE SUB McpAskRai(CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE FUNCTION McpRaiStepsText AS STRING
DECLARE FUNCTION McpRaiTranscript(Chars AS INTEGER) AS STRING
DECLARE FUNCTION McpRaiAudit(Count AS INTEGER) AS STRING
DECLARE SUB McpDebug(CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE FUNCTION McpDebugState AS STRING
DECLARE SUB McpCapture(CallId AS INTEGER, Client AS STRING)
DECLARE FUNCTION McpDialogOpen AS STRING
DECLARE SUB McpLogEnd
DECLARE FUNCTION McpSetProperty(Component AS STRING, Prop AS STRING, Value AS STRING) AS STRING

CONST MCP_DEFAULT_PORT = 47330
' (what the first-time question says: 2 since AI apps look at and use the
' running program — docs/legal/review-2026-10-29-app-eyes.md, F3 —; who
' confirmed an earlier one is asked once more)
CONST MCP_CONSENT = 2
CONST MCPWAITS = 16
' (the program's output kept for read_output: its last this many characters)
CONST MCP_OUTPUT_KEPT = 65536

DIM Mcp AS RMcpServer
DIM McpTools AS RAIProjectTools
DIM McpJ AS RJson
DIM McpSecrets AS RSecrets
DIM McpTimer AS RTimer
' (the settings: on, the port, each kind's choice — "allow", "ask", "never")
DIM McpOn AS INTEGER
DIM McpPort AS INTEGER
' (the permission mode: "read", "ask" (the default), "edits", "all"; the
' project it is for — its file, "" none —; each project's, kept as
' "file<TAB>mode" lines; the menus being filled: not the user's change)
DIM McpMode AS STRING
DIM McpModeOf AS STRING
DIM McpModes AS STRING
DIM McpModeFilling AS INTEGER
DIM McpConfirmed AS INTEGER
' (the first-time question's version the user confirmed — MCP_CONSENT —,
' and the settings' upgrades done: 2, "Allow everything" set back to Ask
' before changes once)
DIM McpConsentVersion AS INTEGER
DIM McpUpgrade AS INTEGER
DIM McpReaskTimer AS RTimer
' (a project an AI app created or opened took the mode of the one open
' before: said after that call's line — the pane, the status bar)
DIM McpCarrySaid AS STRING
' (the kinds allowed for this session from the pane: "|change|run|")
DIM McpGrants AS STRING
DIM McpToken AS STRING
DIM McpCalls AS INTEGER
DIM McpTestToken AS STRING
' (the calls waiting for the user, oldest first)
DIM McpWaitCall(MCPWAITS) AS INTEGER
DIM McpWaitTool(MCPWAITS) AS STRING
DIM McpWaitArgs(MCPWAITS) AS STRING
DIM McpWaitKind(MCPWAITS) AS STRING
DIM McpWaitClient(MCPWAITS) AS STRING
DIM McpWaitWhat(MCPWAITS) AS STRING
DIM McpWaitCount AS INTEGER
' (run_program waiting for the program: its call, until when, what it was
' asked; ask_rai waiting for RAI)
DIM McpRunCall AS INTEGER, McpRunUntil AS DOUBLE, McpRunFrom AS LONG, McpRunAction AS STRING
DIM McpRaiCall AS INTEGER, McpRaiUntil AS DOUBLE, McpRaiSaid AS STRING
' (debug waiting for the program to stop: its call, until when, the stops
' seen before it, then a moment for the variables to arrive)
DIM McpDebugCall AS INTEGER, McpDebugUntil AS DOUBLE, McpDebugStops AS LONG, McpDebugSettle AS DOUBLE
' (the program's output, and where read_output read up to)
DIM McpOutput AS STRING
DIM McpOutputDropped AS LONG
DIM McpOutputRead AS LONG
' (the status bar's panel for AI apps: added the first time they're on)
DIM McpStatusPanel AS INTEGER
' (the warning chip's icon)
DIM McpChipBmp AS RBitmap
' (Settings > AI Apps' rows: what AI apps can see and do)
DIM McpRowImg(3) AS RImage
DIM McpRowBmp(3) AS RBitmap
DIM McpRowHeadL(3) AS RLabel
DIM McpRowBodyL(3) AS RLabel
' (the activity list's menu: Copy, Copy All, Clear Activity...)
DIM McpLogMenu AS RPopupMenu
DIM McpLogItem(2) AS RMenuItem
DIM McpLastWhat AS STRING
' (the review dialog's proposals: McpTools' index of each row)
DIM McpRevIndex(32) AS INTEGER
DIM McpRevCount AS INTEGER
DIM McpRevAt AS INTEGER
DIM McpRevDecided AS INTEGER

' (mcpapp.inc: the eyes and hands on the program Studio runs — app_layout,
' app_screenshot, app_click, app_type, app_key, app_drag, app_scroll,
' app_wait)
$INCLUDE "mcpapp.inc"

' ---- the AI Apps pane: who is connected, what they do, what waits for you -------
CREATE McpPanel AS RPanel
  BevelOuter = 0
  AccessibleName = TR$("AI Apps")
  ' (the pane's header, docs/design-guidelines.md "Pane anatomy": the title
  ' and a line under it at the left; at the right, on one centre line and
  ' all compact, Allow everything's warning chip, the mode's menu and Stop:
  ' McpLayOutPane)
  CREATE McpTop AS RPanel
    Align = 1
    Height = 52
    BevelOuter = 0
    CREATE McpHead AS RLabel
      Left = 12: Top = 9: Width = 300
      AccessibleName = TR$("AI apps")
    END CREATE
    CREATE McpSub AS RLabel
      Left = 12: Top = 27: Width = 300
      AccessibleName = TR$("What the AI app does now")
    END CREATE
    CREATE McpModeBox AS RComboBox
      Style = 2
      AccessibleName = TR$("What AI apps may do")
      AccessibleDescription = TR$("Read only, Ask before changes, Allow edits or Allow everything: for this project")
      OnChange = McpModePicked
    END CREATE
    ' (the warning chip: a soft amber pill, its icon and its words — read
    ' as one warning; its parts are hidden from screen readers)
    CREATE McpBadge AS RPanel
      BevelOuter = 0
      Visible = 0
      AccessibleName = TR$("Warning: AI apps may change and run anything in this project without asking")
      CREATE McpChip AS RCanvas
        Left = 0: Top = 0
        Tag = 2
        OnPaint = ChipPaint
      END CREATE
      CREATE McpChipIcon AS RImage
        Width = 16: Height = 16
      END CREATE
      CREATE McpBadgeText AS RLabel
        Caption = TR$("No questions asked")
        Transparent = 1
      END CREATE
    END CREATE
    CREATE McpStopBtn AS RButton
      Caption = TR$("&Stop")
      AccessibleName = TR$("Stop the server: every AI app is disconnected")
      OnClick = McpStopClick
    END CREATE
  END CREATE
  ' (a question waiting for the user: a card, its 4-pixel mark in the
  ' warning colour, the question, compact buttons under it)
  CREATE McpAsk AS RPanel
    Align = 1
    Height = 90
    BevelOuter = 0
    Visible = 0
    CREATE McpAskMark AS RPanel
      Left = 0: Top = 0: Width = 4: Height = 90
      BevelOuter = 0
    END CREATE
    CREATE McpAskText AS RLabel
      Left = 16: Top = 10: WordWrap = 1: AutoSize = 0: Height = 36
      AccessibleName = TR$("What an AI app asks to do")
    END CREATE
    CREATE McpOnceBtn AS RButton
      Caption = TR$("Allow &Once")
      OnClick = McpOnceClick
    END CREATE
    CREATE McpSessionBtn AS RButton
      Caption = TR$("Allow for This Se&ssion")
      AccessibleDescription = TR$("Calls of this kind are allowed until Studio closes or the server stops")
      OnClick = McpSessionClick
    END CREATE
    CREATE McpDenyBtn AS RButton
      Caption = TR$("&Deny")
      OnClick = McpDenyClick
    END CREATE
    CREATE McpReviewBtn AS RButton
      Caption = TR$("&Review...")
      AccessibleName = TR$("Review the change before deciding")
      OnClick = McpReviewClick
    END CREATE
  END CREATE
  ' (placed by McpLayOutPane: a whole number of rows tall)
  CREATE McpLog AS RListBox
    AccessibleName = TR$("What AI apps did")
  END CREATE
END CREATE

' ---- Tools > Settings > AI Apps (settings.inc shows it as its third tab) ------
CREATE McpPage AS RPanel
  BevelOuter = 0
  AccessibleName = TR$("AI Apps")
  Visible = 0
  CREATE McpOnBox AS RCheckBox
    Caption = TR$("&Let AI apps use RapidR Studio")
    AccessibleDescription = TR$("Off by default. On: an AI app on this computer that has the address and token below can work with your open project")
    OnClick = McpSetOnClick
  END CREATE
  CREATE McpOnNote AS RLabel
    WordWrap = 1: AutoSize = 0
    Caption = TR$("When on, an AI app on this computer that has the address and token below can work with your open project; you see everything it does in the AI Apps pane (Tools > AI Apps Activity). Paste both into the AI app's MCP settings.")
  END CREATE
  CREATE McpPortLabel AS RLabel
    Caption = TR$("P&ort:")
  END CREATE
  CREATE McpPortEdit AS REdit
    AccessibleName = TR$("Port")
    AccessibleDescription = TR$("The port AI apps connect to on 127.0.0.1 (this computer only)")
  END CREATE
  CREATE McpAddrLabel AS RLabel
    Caption = TR$("Address:")
  END CREATE
  CREATE McpAddrEdit AS REdit
    ReadOnly = 1
    AccessibleName = TR$("The address AI apps connect to")
  END CREATE
  CREATE McpCopyAddrBtn AS RButton
    Caption = TR$("Copy &Address")
    OnClick = McpCopyAddressClick
  END CREATE
  CREATE McpHeaderLabel AS RLabel
    Caption = TR$("Header:")
  END CREATE
  CREATE McpHeaderEdit AS REdit
    ReadOnly = 1
    PasswordChar = "*"
    AccessibleName = TR$("The header AI apps send: Authorization, Bearer and the token")
  END CREATE
  CREATE McpCopyHeaderBtn AS RButton
    Caption = TR$("Copy &Header")
    OnClick = McpCopyHeaderClick
  END CREATE
  CREATE McpShowTokenBtn AS RButton
    Caption = TR$("Sho&w")
    AccessibleName = TR$("Show or hide the token")
    OnClick = McpShowTokenClick
  END CREATE
  CREATE McpResetBtn AS RButton
    Caption = TR$("&Reset Token")
    AccessibleDescription = TR$("A new token: every AI app that had the old one is disconnected")
    OnClick = McpResetTokenClick
  END CREATE
  CREATE McpPermHead AS RLabel
    Caption = TR$("What AI apps may &do:")
  END CREATE
  CREATE McpPageModeBox AS RComboBox
    Style = 2
    AccessibleName = TR$("What AI apps may do in this project")
    OnChange = McpPageModePicked
  END CREATE
  CREATE McpModeNoteLabel AS RLabel
    WordWrap = 1: AutoSize = 0
    AccessibleName = TR$("What the choice means")
  END CREATE
  ' (what AI apps can see and do: the rows McpRow… — the consent's own,
  ' SetUpMcp makes them —, a footer, Learn more)
  CREATE McpSeeFoot AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE McpLearnLink AS RLabel
    Caption = TR$("Learn more about AI apps")
    TabStop = 1
    AccessibleDescription = TR$("Opens the manual's page in your browser")
    OnClick = McpLearnClick
  END CREATE
  CREATE McpClearBtn AS RButton
    Caption = TR$("C&lear Activity...")
    AccessibleDescription = TR$("Removes the list of what AI apps did from this computer")
    OnClick = McpClearLogClick
  END CREATE
END CREATE

' ---- reviewing an AI app's change before allowing it ---------------------------
CREATE McpReview AS RForm
  Caption = TR$("Review the AI App's Change")
  Width = 980: Height = 640
  CREATE McpRevFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE McpRevApplyBtn AS RButton
      Caption = TR$("A&pply")
      Default = 1
      OnClick = McpRevApply
    END CREATE
    CREATE McpRevRejectBtn AS RButton
      Caption = TR$("&Reject")
      Cancel = 1
      OnClick = McpRevReject
    END CREATE
  END CREATE
  CREATE McpRevHead AS RLabel
    Align = 1
    Height = 36
    AccessibleName = TR$("What the change does")
  END CREATE
  CREATE McpRevList AS RListBox
    Align = 3
    Width = 220
    AccessibleName = TR$("Files the AI app changes")
    OnClick = McpRevPicked
  END CREATE
  CREATE McpRevDiff AS RDiffView
    Align = 5
    Mode = "split"
    AccessibleName = TR$("The change: the file as it is on the left, as proposed on the right")
  END CREATE
END CREATE

' ---- settings ------------------------------------------------------------------

FUNCTION McpKindOf(Tool AS STRING) AS STRING
  SELECT CASE Tool
    CASE "propose_edit", "write_file", "create_project", "open_project", "set_property": Result = "change"
    CASE "run_program", "debug": Result = "run"
    ' (mcpapp.inc: looking at or using the running program — it shows what
    ' the program shows: asked once a run)
    CASE "app_layout", "app_screenshot", "app_click", "app_type", "app_key", "app_drag", "app_scroll": Result = "drive"
    CASE "ask_rai": Result = "rai"
    CASE ELSE: Result = "read"
  END SELECT
END FUNCTION

' ---- the permission mode -------------------------------------------------------

' A mode as the menus say it.
FUNCTION McpModeName(Mode AS STRING) AS STRING
  SELECT CASE Mode
    CASE "read": Result = TR$("Read only")
    CASE "edits": Result = TR$("Allow edits")
    CASE "all": Result = TR$("Allow everything")
    CASE ELSE: Result = TR$("Ask before changes")
  END SELECT
END FUNCTION

' (the same in English: what an AI app reads — studio_status — and the audit log
' say, never translated)
FUNCTION McpModeNameEn(Mode AS STRING) AS STRING
  SELECT CASE Mode
    CASE "read": Result = "Read only"
    CASE "edits": Result = "Allow edits"
    CASE "all": Result = "Allow everything"
    CASE ELSE: Result = "Ask before changes"
  END SELECT
END FUNCTION

' (the menus' mode lists: filled again when Studio's language changes)
SUB McpFillModes(Box AS RComboBox)
  Box.Clear
  Box.AddItems McpModeName("read"), McpModeName("ask"), McpModeName("edits"), McpModeName("all")
END SUB

FUNCTION McpModeIndex(Mode AS STRING) AS INTEGER
  SELECT CASE Mode
    CASE "read": Result = 0
    CASE "edits": Result = 2
    CASE "all": Result = 3
    CASE ELSE: Result = 1
  END SELECT
END FUNCTION

FUNCTION McpModeAt(Index AS INTEGER) AS STRING
  SELECT CASE Index
    CASE 0: Result = "read"
    CASE 2: Result = "edits"
    CASE 3: Result = "all"
    CASE ELSE: Result = "ask"
  END SELECT
END FUNCTION

' What a mode means, in a sentence or two (Settings shows it under the menu).
FUNCTION McpModeNote(Mode AS STRING) AS STRING
  SELECT CASE Mode
    CASE "read": Result = TR$("AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program.")
    CASE "edits": Result = TR$("Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you.")
    CASE "all": Result = TR$("Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project.")
    CASE ELSE: Result = TR$("AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny.")
  END SELECT
END FUNCTION

' (the same in English, for studio_status: keep it in step with McpModeNote)
FUNCTION McpModeNoteEn(Mode AS STRING) AS STRING
  SELECT CASE Mode
    CASE "read": Result = "AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program."
    CASE "edits": Result = "Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you."
    CASE "all": Result = "Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project."
    CASE ELSE: Result = "AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny."
  END SELECT
END FUNCTION

' What a call of Kind gets in the mode: "allow", "ask" or "refuse".
FUNCTION McpDecision(Kind AS STRING) AS STRING
  IF Kind = "read" THEN
    Result = "allow"
    EXIT FUNCTION
  END IF
  SELECT CASE McpMode
    CASE "read": Result = "refuse"
    CASE "all": Result = "allow"
    CASE "edits"
      IF Kind = "change" THEN Result = "allow" ELSE Result = "ask"
    CASE ELSE: Result = "ask"
  END SELECT
  ' (Allow for This Session: that kind until Studio closes, the server
  ' stops or another project opens)
  IF Result = "ask" AND INSTR(McpGrants, "|" + Kind + "|") > 0 THEN Result = "allow"
  ' (mcpapp.inc: the running program, allowed once — until this run ends)
  IF Result = "ask" AND Kind = "drive" AND RunStarted <> 0 AND McpAppRunAllowed = RunStarted THEN Result = "allow"
END FUNCTION

' (the mode of the project open now: its own, kept, else Ask before changes)
SUB McpModeFollow
  DIM key AS STRING
  IF ProjectOpen THEN key = ProjectFile ELSE key = ""
  IF key = McpModeOf AND McpMode <> "" THEN EXIT SUB
  McpModeOf = key
  McpMode = McpModeKept(key)
  McpGrants = ""
  McpShowMode
END SUB

FUNCTION McpModeKept(Key AS STRING) AS STRING
  DIM k AS INTEGER, line AS STRING
  Result = "ask"
  FOR k = 1 TO TALLY(McpModes, CHR$(10)) + 1
    line = FIELD$(McpModes, CHR$(10), k)
    IF line <> "" AND FIELD$(line, CHR$(9), 1) = Key THEN Result = FIELD$(line, CHR$(9), 2)
  NEXT
  IF Result <> "read" AND Result <> "edits" AND Result <> "all" THEN Result = "ask"
END FUNCTION

' Whether project Key has a mode of its own, kept.
FUNCTION McpModeIsKept(Key AS STRING) AS INTEGER
  DIM k AS INTEGER, line AS STRING
  FOR k = 1 TO TALLY(McpModes, CHR$(10)) + 1
    line = FIELD$(McpModes, CHR$(10), k)
    IF line <> "" AND FIELD$(line, CHR$(9), 1) = Key THEN Result = 1
  NEXT
END FUNCTION

' A project an AI app created or opened (create_project, open_project)
' takes Before, the mode of the project open before it, as if the user had
' picked it — an AI app working alone isn't stopped by a question the
' other project didn't ask (Robert, 2026-10-29) —, except Allow everything,
' which it never takes (it starts with Allow edits); said in the pane, the
' status bar and aloud. Nothing open before: the default (Ask before
' changes). A project with a mode of its own keeps it.
SUB McpCarryMode(Before AS STRING, BeforeKey AS STRING)
  DIM key AS STRING, carried AS STRING
  IF Before = "" OR ProjectOpen = 0 THEN EXIT SUB
  key = ProjectFile
  IF key = BeforeKey OR McpModeIsKept(key) THEN EXIT SUB
  carried = Before
  ' (Allow everything was confirmed for one project, while the user
  ' watched: never passed on — docs/legal/review-2026-10-29-app-eyes.md,
  ' Addendum 2)
  IF carried = "all" THEN carried = "edits"
  McpModeOf = key
  McpMode = ""
  McpSetMode carried
  IF Before = "all" THEN
    McpCarrySaid = TR$("New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it.")
  ELSE
    McpCarrySaid = TR$("New project: same permission as before ({mode})", "mode", McpModeName(carried))
  END IF
END SUB

' The project's mode from now on: kept, said, in the audit log.
SUB McpSetMode(Mode AS STRING)
  DIM k AS INTEGER, line AS STRING, kept AS STRING
  IF Mode = McpMode THEN
    McpShowMode
    EXIT SUB
  END IF
  McpMode = Mode
  FOR k = 1 TO TALLY(McpModes, CHR$(10)) + 1
    line = FIELD$(McpModes, CHR$(10), k)
    IF line <> "" AND FIELD$(line, CHR$(9), 1) <> McpModeOf THEN kept = kept + line + CHR$(10)
  NEXT
  ' (the newest last; the 50 last projects' kept)
  kept = kept + McpModeOf + CHR$(9) + Mode
  WHILE TALLY(kept, CHR$(10)) >= 50
    kept = MID$(kept, INSTR(kept, CHR$(10)) + 1)
  WEND
  McpModes = kept
  McpGrants = ""
  McpSaveSettings
  McpShowMode
  McpLog.AddItems LEFT$(TIME$, 8) + "  " + TR$("AI apps may now: {mode}", "mode", McpModeName(Mode))
  McpLogEnd
  McpTools.AuditCall "Studio", "mode", "", McpModeNameEn(Mode)
  Application.Announce TR$("AI apps: {mode}", "mode", McpModeName(Mode))
END SUB

' (Allow everything chosen: what it means said in rows — consent.inc —,
' Cancel the default)
FUNCTION McpConfirmAll AS INTEGER
  ConsentBegin TR$("Allow everything for AI apps in this project?"), TR$("Nothing waits for you: AI apps change files, run the program, look at it, click and type in it and ask RAI (which may cost money) on their own.")
  ConsentRow "warning", TR$("Keep in mind"), TR$("A program they run can do anything you can on this computer, and what they click in it is real. Choose this only while you watch an AI app you trust.")
  ConsentRow "visible", TR$("You still see"), TR$("Everything they do, listed in the AI Apps pane; Stop ends it.")
  ConsentRow "lock", TR$("Never"), TR$("Studio's own tools never hand over your keys, or files outside the project.")
  ConsentYes.AccessibleDescription = TR$("AI apps change and run anything in this project without asking")
  ConsentNo.AccessibleDescription = TR$("The permission stays as it was")
  Result = ConsentAsk("", McpManualUrl, TR$("&Allow Everything"), TR$("Cancel"))
END FUNCTION

' The pane's menu: the user's choice (Allow everything confirmed first).
SUB McpModePicked
  DIM m AS STRING
  IF McpModeFilling THEN EXIT SUB
  m = McpModeAt(McpModeBox.ItemIndex)
  IF m = "all" AND McpMode <> "all" THEN
    IF McpConfirmAll = 0 THEN
      McpShowMode
      EXIT SUB
    END IF
  END IF
  McpSetMode m
END SUB

' (the menu, the badge and the status bar as the mode is)
SUB McpShowMode
  McpModeFilling = 1
  IF McpModeBox.ItemCount = 0 THEN McpFillModes McpModeBox
  McpModeBox.ItemIndex = McpModeIndex(McpMode)
  McpModeFilling = 0
  McpBadge.Visible = Flag(McpMode = "all")
  McpLayOutPane
  McpStatusShow
END SUB

SUB McpLoadSettings
  McpPort = MCP_DEFAULT_PORT
  McpMode = "ask"
  McpOn = 0
  ' (new settings need no upgrade; settings kept before upgrades were
  ' counted, McpModes without McpUpgrade, do)
  McpUpgrade = MCP_CONSENT
  IF SettingsRead = 0 THEN EXIT SUB
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) = 0 THEN EXIT SUB
  Reg.OpenKey(STUDIO_KEY, 0)
  IF Reg.ValueExists("McpOn") THEN McpOn = Flag(Reg.ReadString("McpOn") = "1")
  IF Reg.ValueExists("McpPort") THEN McpPort = VAL(Reg.ReadString("McpPort"))
  IF McpPort < 1024 OR McpPort > 65535 THEN McpPort = MCP_DEFAULT_PORT
  IF Reg.ValueExists("McpConfirmed") THEN McpConfirmed = Flag(Reg.ReadString("McpConfirmed") = "1")
  IF Reg.ValueExists("McpConsentVersion") THEN McpConsentVersion = VAL(Reg.ReadString("McpConsentVersion"))
  IF Reg.ValueExists("McpUpgrade") THEN
    McpUpgrade = VAL(Reg.ReadString("McpUpgrade"))
  ELSEIF Reg.ValueExists("McpModes") OR Reg.ValueExists("McpConfirmed") THEN
    McpUpgrade = 0
  END IF
  IF Reg.ValueExists("McpModes") THEN McpModes = REPLACESUBSTR$(REPLACESUBSTR$(Reg.ReadString("McpModes"), "|", CHR$(10)), "?", CHR$(9))
  ' (once: a project's "Allow everything" was confirmed before it covered
  ' looking at and using the running program — set back to Ask before
  ' changes; the user chooses it again, with what it means said)
  IF McpUpgrade < MCP_CONSENT THEN
    McpModes = REPLACESUBSTR$(McpModes + CHR$(10), CHR$(9) + "all" + CHR$(10), CHR$(9) + "ask" + CHR$(10))
    McpModes = LEFT$(McpModes, LEN(McpModes) - 1)
    McpUpgrade = MCP_CONSENT
    McpKeepConsent
  END IF
END SUB

' Whether the user confirmed the first-time question as it is now.
FUNCTION McpConsentOk AS INTEGER
  Result = Flag(McpConfirmed <> 0 AND McpConsentVersion >= MCP_CONSENT)
END FUNCTION

' (the confirmation and the upgrade kept — at once, tests too: the
' question is asked once, never again)
SUB McpKeepConsent
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("McpOn", Num(McpOn))
  Reg.WriteString("McpConfirmed", Num(McpConfirmed))
  Reg.WriteString("McpConsentVersion", Num(McpConsentVersion))
  Reg.WriteString("McpUpgrade", Num(McpUpgrade))
  Reg.WriteString("McpModes", REPLACESUBSTR$(REPLACESUBSTR$(McpModes, CHR$(10), "|"), CHR$(9), "?"))
END SUB

' AI apps on, confirmed before the question said they look at and use the
' running program: asked once more as Studio starts — Turn On starts the
' server; Cancel turns AI apps off (the server never started).
SUB McpReaskTick
  McpReaskTimer.Enabled = 0
  IF McpConsent THEN
    McpConfirmed = 1
    McpConsentVersion = MCP_CONSENT
    McpKeepConsent
    McpStart
  ELSE
    McpOn = 0
    McpKeepConsent
    OutputLine TR$("AI apps: off. Confirm again in Settings > AI Apps.")
  END IF
  McpShowState
END SUB

SUB McpSaveSettings
  IF McpTestToken <> "" THEN EXIT SUB
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("McpOn", Num(McpOn))
  Reg.WriteString("McpPort", Num(McpPort))
  Reg.WriteString("McpConfirmed", Num(McpConfirmed))
  Reg.WriteString("McpConsentVersion", Num(McpConsentVersion))
  Reg.WriteString("McpUpgrade", Num(McpUpgrade))
  ' (a project's file and its mode, a line each: | and ? stand for them)
  Reg.WriteString("McpModes", REPLACESUBSTR$(REPLACESUBSTR$(McpModes, CHR$(10), "|"), CHR$(9), "?"))
END SUB

' The token AI apps send: the system's keychain keeps it (made the first
' time); RAPIDR_TEST_MCP_TOKEN stands in for it in tests.
FUNCTION McpTokenNow AS STRING
  DIM t AS STRING
  IF McpTestToken <> "" THEN
    Result = McpTestToken
    EXIT FUNCTION
  END IF
  t = McpSecrets.Read("AI apps", "token")
  IF t = "" THEN
    t = Mcp.NewToken
    IF McpSecrets.Save("AI apps", "token", t) = 0 THEN OutputLine TR$("AI apps: the token couldn't be kept in the keychain ({error}): it is new each time Studio starts.", "error", McpSecrets.Error)
  END IF
  Result = t
END FUNCTION

FUNCTION McpUrl AS STRING
  Result = "http://127.0.0.1:" + Num(McpPort) + "/mcp"
END FUNCTION

' ---- starting and stopping -------------------------------------------------------

SUB SetUpMcp
  DIM k AS INTEGER
  McpTestToken = ENVIRON$("RAPIDR_TEST_MCP_TOKEN")
  McpLoadSettings
  McpTools.Permission = "propose"
  Mcp.OnToolCall = McpToolCalled
  Mcp.OnConnect = McpClientIn
  Mcp.OnDisconnect = McpClientOut
  ' (mcpapp.inc: the running program's answers)
  McpAppSetUp
  Mcp.ServerName = "rapidr-studio"
  Mcp.ServerTitle = "RapidR Studio"
  Mcp.ServerVersion = Lang.Version
  ' (the recipe-first workflow and a short language card: rapidr-studio's
  ' guide.rs — small models start from a working program and read the hints)
  Mcp.Instructions = McpTools.ServerInstructions
  McpTimer.Interval = 250
  McpTimer.Enabled = 0
  McpTimer.OnTimer = McpTickTimer
  ' (the desktop's only: a web page can't listen for AI apps — WebMCP,
  ' later, is the browser's way)
  IF Proj.FileManager = "" THEN
    McpOn = 0
    EXIT SUB
  END IF
  Dock.AddPane(McpPanel, TR$("AI Apps"), "tab:outputbox", "plug")
  McpPanel.OnResize = McpLayOutPane
  Dock.HidePane("mcppanel")
  ' (Output stays the group's front tab, as SetUpDock left it)
  Dock.ShowPane("outputbox")
  McpPage.Parent = SettingsDialog
  SettingsTabs.AddTabs TR$("AI Apps")
  McpFillModes McpPageModeBox
  McpModeFollow
  FOR k = 0 TO 3
    McpRowImg(k).Parent = McpPage
    McpRowImg(k).Width = 16: McpRowImg(k).Height = 16
    McpRowImg(k).AccessibleHidden = 1
    McpRowHeadL(k).Parent = McpPage
    McpRowHeadL(k).AutoSize = 0
    McpRowHeadL(k).Caption = McpRowHead(k)
    McpRowHeadL(k).AccessibleHidden = 1
    McpRowBodyL(k).Parent = McpPage
    McpRowBodyL(k).WordWrap = 1: McpRowBodyL(k).AutoSize = 0
    McpRowBodyL(k).Caption = McpRowBody(k)
    McpRowBodyL(k).AccessibleName = McpRowHead(k) + ": " + McpRowBody(k)
  NEXT
  McpSeeFoot.Caption = McpFootText
  ' (the activity list's menu: the Output pane's words)
  McpLogItem(0).Caption = TR$("&Copy") + CHR$(9) + "Ctrl+C"
  McpLogItem(1).Caption = TR$("Copy &All")
  McpLogItem(2).Caption = TR$("C&lear Activity...")
  FOR k = 0 TO 2
    McpLogItem(k).Tag = k
    McpLogItem(k).OnClick = McpLogMenuPicked
    McpLogMenu.AddItems McpLogItem(k)
  NEXT
  McpLogMenu.OnPopup = McpLogMenuOpening
  McpLog.PopupMenu = McpLogMenu
  McpReaskTimer.Interval = 400
  McpReaskTimer.Enabled = 0
  McpReaskTimer.OnTimer = McpReaskTick
  McpShowState
  ' (School Lock: AI apps locked — nothing started, nothing asked; the
  ' unlock starts them as the user had them: schoollock.inc)
  IF McpOn AND LockOffApps = 0 THEN
    IF McpConsentOk THEN
      McpStart
    ELSEIF McpConfirmed THEN
      ' (confirmed an earlier question: asked once more, once Studio shows)
      McpReaskTimer.Enabled = 1
    ELSE
      McpOn = 0
    END IF
  END IF
END SUB

' What AI apps can see and do (docs/legal/review-2026-10-29-mcp.md, 4b):
' the rows of the first-time question and of Settings > AI Apps — K's
' icon, head and words — and what follows them.
FUNCTION McpRowIcon(K AS INTEGER) AS STRING
  SELECT CASE K
    CASE 0: Result = "visible"
    CASE 1: Result = "ai-tool"
    CASE 2: Result = "lock"
    CASE ELSE: Result = "info"
  END SELECT
END FUNCTION

FUNCTION McpRowHead(K AS INTEGER) AS STRING
  SELECT CASE K
    CASE 0: Result = TR$("It can see")
    CASE 1: Result = TR$("It can")
    CASE 2: Result = TR$("It never sees")
    CASE ELSE: Result = TR$("Keep in mind")
  END SELECT
END FUNCTION

FUNCTION McpRowBody(K AS INTEGER) AS STRING
  SELECT CASE K
    CASE 0: Result = TR$("The files of your open project — your code, forms and data files — and nothing else on your computer. It also gets pictures of Studio's window and of your running program's windows, and a written list of what they show, as you see them.")
    CASE 1: Result = TR$("Check code, and read what your program prints and RAI's chat. It can also change forms and code, open other projects, run, debug, look at, click and type in your program, and ask RAI (which may cost money), each only as your permission mode allows; the default asks you first.")
    CASE 2: Result = TR$("Your AI provider keys or passwords: your keychain keeps them.")
    CASE ELSE: Result = TR$("RapidR sends none of this anywhere, but the AI app may pass what it reads to the company that makes it, under that company's terms. A program can show private data, yours or other people's, and clicking in it acts as you: a real Send, Pay or Delete button does it for real, so use test data. Many AI apps are for adults only: if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust.")
  END SELECT
END FUNCTION

FUNCTION McpFootText AS STRING
  Result = TR$("Share only what you may share: schoolwork, an employer's code or someone else's licensed code may not be yours to share. Turn this off, or Reset Token, to disconnect every AI app. Studio keeps a list of what they did on this computer only; Clear Activity removes it.")
END FUNCTION

' (the manual's page, in the browser)
FUNCTION McpManualUrl AS STRING
  Result = "https://github.com/iBobX/RapidR/blob/main/docs/manual/ai-apps.md"
END FUNCTION

SUB McpLearnClick
  OpenWebLink McpManualUrl
END SUB

SUB McpStart
  IF Mcp.Listening THEN EXIT SUB
  ' (School Lock: AI apps can't connect while it is on — schoollock.inc)
  IF LockOffApps THEN
    OutputLine TR$("AI apps can't start: School Lock is on (Tools > School Lock...).")
    McpShowState
    EXIT SUB
  END IF
  McpToken = McpTokenNow
  IF McpToken = "" THEN EXIT SUB
  Mcp.Host = "127.0.0.1"
  Mcp.Port = McpPort
  Mcp.Token = McpToken
  McpGrants = ""
  McpSyncTools
  IF Mcp.Start = 0 THEN
    OutputLine TR$("AI apps can't connect: {error}. Pick another port in Tools > Settings > AI Apps.", "error", Mcp.Error)
    SetState RunState, TR$("AI apps: port {port} is taken", "port", McpPort)
  ELSE
    OutputLine TR$("AI apps can connect at {url} (this computer only; the header is in Tools > Settings > AI Apps).", "url", McpUrl)
    McpTimer.Enabled = 1
  END IF
  McpShowState
END SUB

SUB McpStopServer
  DIM k AS INTEGER
  ' (what waits is denied: its AI app is told)
  WHILE McpWaitCount > 0
    McpDecide 0
  WEND
  IF McpRunCall > 0 THEN Mcp.Reply McpRunCall, "The server stopped.", 1
  IF McpRaiCall > 0 THEN Mcp.Reply McpRaiCall, "The server stopped.", 1
  McpRunCall = 0: McpRaiCall = 0
  IF Mcp.Listening THEN
    Mcp.Disconnect ""
    Mcp.Stop
    McpTools.AuditCall "Studio", "stop", "", "the server stopped"
  END IF
  McpTimer.Enabled = 0
  McpGrants = ""
  McpShowState
END SUB

' (Studio closing)
SUB McpShutdown
  McpStopServer
END SUB

' Stop, in the pane: the server off, every AI app disconnected (Settings
' turns it on again).
SUB McpStopClick
  IF Mcp.Listening = 0 THEN
    ShowSettingsPage 2
    EXIT SUB
  END IF
  McpOn = 0
  McpStopServer
  McpSaveSettings
  OutputLine TR$("AI apps: stopped. Every AI app was disconnected.")
END SUB

' (the tools: the assistant's — those that change files only when changes
' aren't off —, then Studio's own)
SUB McpSyncTools
  DIM names AS STRING, n AS INTEGER, k AS INTEGER, t AS STRING
  Mcp.ClearTools
  ' (the same tools in every mode — a call the mode refuses is said so, in
  ' plain words)
  McpTools.Permission = "propose"
  Mcp.AddTool "studio_status", "What RapidR Studio has open now: the project and its files, the documents open (unsaved ones marked), whether the program runs, its errors, RAI, and what you may do. Call it first.", "", "read"
  names = McpTools.ToolNames
  n = TALLY(names, CHR$(10)) + 1
  FOR k = 1 TO n
    t = FIELD$(names, CHR$(10), k)
    IF t <> "" AND t <> "layout" THEN Mcp.AddTool t, McpTools.ToolDescription(t), McpTools.ToolSchema(t), McpTools.ToolKind(t)
  NEXT
  Mcp.AddTool "list_documents", "The documents open in Studio's editor: each file (relative to the project), whether it is the active one, its view (Code or Design) and whether it has unsaved changes.", "", "read"
  Mcp.AddTool "get_problems", "The program's errors and warnings as Studio's compiler finds them (its main file and what it includes), with file, line and column.", "", "read"
  Mcp.AddTool "read_output", "What the running (or last) program printed in Studio's Output, since your last read_output; All = true for everything kept. Also says whether it still runs.", "[All AS BOOLEAN]", "read"
  Mcp.AddTool "get_rai_steps", "What RAI, Studio's own assistant, is doing or did in its last answer: busy or not, its status, and its steps (each tool it used).", "", "read"
  Mcp.AddTool "get_rai_transcript", "RAI's chat as the user sees it, as Markdown: its last Chars characters (8000 by default).", "[Chars AS INTEGER]", "read"
  Mcp.AddTool "get_rai_audit", "RAI's audit log: its last Count lines (30 by default) — each request to its provider, each tool, each change it proposed and what the user decided.", "[Count AS INTEGER]", "read"
  ' (mcpapp.inc: the running program's eyes and hands — only once the user
  ' confirmed the question that says so; an AI app can't even see them
  ' before)
  IF McpConsentOk THEN McpAppTools
  Mcp.AddTool "layout_report", "The program's forms as the source makes them: each form's client area, each component's place and size (Left, Top, Width, Height, or aligned), then what looks wrong — a control cut off by its form, two on top of each other, a caption too long for its button.", "", "read"
  Mcp.AddTool "capture", "A picture (PNG) of RapidR Studio's window as the user sees it now: the code, the form designer, the panes. Use it to see a form you changed in the designer.", "", "read"
  Mcp.AddTool "open_project", "Opens an existing project (.rrproj) or program (.rr, .bas) in Studio: a path inside the user's Projects folder, RapidR's examples (examples/gui/hello_form.rr) or a recent project; anything else asks the user. Never discards unsaved changes. The open project IS the user's program: to build or change it use write_file and propose_edit on its files — don't open or create another project for the user's task.", "path AS STRING", "change"
  Mcp.AddTool "set_property", "Sets a component's property in the form designer, as the user would in the Properties pane: Component (its name, rapidr_list gives them), Property (Caption, Left, Width, Color …) and Value as the code writes it (" + CHR$(34) + "OK" + CHR$(34) + " for text, 120, &HFF0000, True). One undo step in its file.", "Component AS STRING, Property AS STRING, Value AS STRING", "change"
  Mcp.AddTool "debug", "The debugger, on the program run_program started with Debug = true: Action breakpoint (File and Line: set one; Remove = true takes it away), continue, step_over, step_into, step_out, pause, or state. Waits up to Wait seconds (5 by default) for the program to stop, then says where it is, the call stack and the variables.", "Action AS STRING, [File AS STRING], [Line AS INTEGER], [Remove AS BOOLEAN], [Wait AS INTEGER]", "change"
  Mcp.AddTool "run_program", "Runs the open project's program in Studio (saving its files first, as Run does) and waits up to Wait seconds (3 by default, 60 at most) or until it ends; says how it went and what it printed. Action: start (default), stop, restart or status. Debug = true runs it with the debugger (it stops at the user's breakpoints).", "[Action AS STRING], [Wait AS INTEGER], [Debug AS BOOLEAN]", "change"
  Mcp.AddTool "ask_rai", "Asks RAI, RapidR Studio's own assistant, as the user would in its panel (the user sees it), and waits for its answer (Wait = false: at once; then get_rai_steps). RAI may propose changes the user reviews. It uses the user's AI provider, which may cost money.", "Prompt AS STRING, [Wait AS BOOLEAN]", "network"
END SUB

' The open project, as McpTools sees it: its files, the editors' texts.
SUB McpSyncProject
  DIM i AS INTEGER, all AS STRING, d AS INTEGER
  McpTools.ClearFiles
  IF ProjectOpen = 0 THEN
    McpTools.ProjectFolder = ""
    McpTools.ProjectFiles = ""
    McpTools.MainFile = ""
    McpTools.ProjectName = ""
    EXIT SUB
  END IF
  FOR i = 0 TO Proj.FileCount - 1
    all = all + Proj.File(i) + CHR$(10)
  NEXT
  McpTools.ProjectFolder = Proj.Folder
  McpTools.ProjectFiles = all
  McpTools.MainFile = Proj.MainFile
  McpTools.ProjectName = Proj.Name
  McpTools.CompatMode = Proj.CompatMode
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN McpTools.SetFile DocFile(d), CodeDoc(d).Text
  NEXT
END SUB

' ---- the pane and the status bar ------------------------------------------------------

SUB McpFonts
  McpHead.Font = UIFont
  McpHead.Font.Bold = 1
  McpHead.Font.Color = TC("page.text")
  McpSub.Font = DimFont
  McpSub.Font.Color = TC("page.dim")
  McpAskText.Font = UIFont
  McpAskText.Font.Bold = 1
  McpAskText.Font.Color = TC("page.text")
  ' (a question waiting: a card, its mark in the warning colour)
  McpAsk.Color = TC("card")
  McpAskMark.Color = TC("status.warning.border")
  McpLog.Font = UIFont
  IF ThemeName = "classic" THEN McpLog.ItemHeight = 16 ELSE McpLog.ItemHeight = 22
  McpStopBtn.Font = UIFont
  McpModeBox.Font = UIFont
  ' (Allow everything's chip: status.warning — a soft fill on the header's
  ' face, the warning icon, its words in the strong colour)
  McpBadge.Color = TC("face")
  McpChip.Color = TC("face")
  McpChip.AccessibleHidden = 1
  McpChipIcon.AccessibleHidden = 1
  McpBadgeText.AccessibleHidden = 1
  McpBadgeText.Font = UIFont
  McpBadgeText.Font.Color = TC("status.warning.text")
  McpChipBmp.LoadIcon "warning", 16
  McpChipIcon.BMPHandle = McpChipBmp
  ' (the question's lines are measured in the chrome font: AISetLinesHigh's
  ' bitmap, as the Settings dialog sets it)
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  McpOnceBtn.Font = UIFont
  McpSessionBtn.Font = UIFont
  McpDenyBtn.Font = UIFont
  McpReviewBtn.Font = UIFont
  McpLayOutPane
  McpChip.Repaint
END SUB

' The pane's controls for its width (docs/design-guidelines.md, "Pane
' anatomy"): the header — the title and its line at the left, centred
' together; at the right Stop, the mode's menu and Allow everything's chip,
' all UiCompactH tall on the header's centre line, 8 apart, 12 from the
' edge —; the question strip under it, its buttons compact too.
SUB McpLayOutPane
  DIM w AS INTEGER, ch AS INTEGER, hh AS INTEGER, y AS INTEGER, x AS INTEGER, bw AS INTEGER, rx AS INTEGER, th AS INTEGER, ah AS INTEGER
  ch = UiCompactH
  hh = UiPaneHeadH
  w = McpPanel.ClientWidth
  IF w < 200 THEN w = 600
  IF McpTop.Height <> hh THEN McpTop.Height = hh
  y = (hh - ch) \ 2
  x = w - 12 - 88
  McpStopBtn.Left = x: McpStopBtn.Top = y: McpStopBtn.Width = 88: McpStopBtn.Height = ch
  x = x - 8 - 180
  McpModeBox.Left = x: McpModeBox.Top = y: McpModeBox.Width = 180: McpModeBox.Height = ch
  ' (the chip: 8, the icon, 6, the words, 12)
  bw = 8 + 16 + 6 + McpBadgeText.Width + 12
  x = x - 8 - bw
  McpBadge.Left = x: McpBadge.Top = y: McpBadge.Width = bw: McpBadge.Height = ch
  McpChip.Left = 0: McpChip.Top = 0: McpChip.Width = bw: McpChip.Height = ch
  McpChipIcon.Left = 8: McpChipIcon.Top = (ch - 16) \ 2
  McpBadgeText.Left = 30: McpBadgeText.Top = (ch - McpBadgeText.Height) \ 2
  IF McpBadge.Visible THEN rx = McpBadge.Left ELSE rx = McpModeBox.Left
  ' (the title and its line, one block on the header's centre)
  th = McpHead.Height + 2 + McpSub.Height
  McpHead.Left = 12: McpHead.Top = (hh - th) \ 2: McpHead.Width = rx - 24
  McpSub.Left = 12: McpSub.Top = McpHead.Top + McpHead.Height + 2: McpSub.Width = rx - 24
  ' (the question: its words — as many lines as they take, up to three
  ' (measured plain: bold is wider, hence the tenth less) —, then the
  ' buttons; the strip as tall as that)
  McpAskText.Width = w - 28
  th = AISetLinesHigh(McpAskText.Caption, (w - 28) * 9 \ 10)
  IF th < 18 THEN th = 18
  IF th > 54 THEN th = 54
  McpAskText.Top = 10: McpAskText.Height = th
  y = 10 + th + 8
  McpOnceBtn.Left = 16: McpOnceBtn.Top = y: McpOnceBtn.Width = 100: McpOnceBtn.Height = ch
  McpSessionBtn.Left = 124: McpSessionBtn.Top = y: McpSessionBtn.Width = 172: McpSessionBtn.Height = ch
  McpDenyBtn.Left = 304: McpDenyBtn.Top = y: McpDenyBtn.Width = 72: McpDenyBtn.Height = ch
  McpReviewBtn.Left = 384: McpReviewBtn.Top = y: McpReviewBtn.Width = 88: McpReviewBtn.Height = ch
  ah = y + ch + 12
  IF McpAsk.Height <> ah THEN McpAsk.Height = ah
  McpAskMark.Height = ah
  ' (the activity under the header and the question: a whole number of
  ' rows tall — a list box scrolled to its end shows no half row at its
  ' top then; what's left under it is the pane's)
  y = McpTop.Height
  IF McpAsk.Visible THEN y = y + McpAsk.Height
  th = McpPanel.ClientHeight - y - 4
  IF th < McpLog.ItemHeight THEN th = McpLog.ItemHeight
  McpLog.Left = 0: McpLog.Top = y: McpLog.Width = w
  McpLog.Height = (th \ McpLog.ItemHeight) * McpLog.ItemHeight + 4
  IF McpLog.ItemCount > 0 AND McpLog.ItemIndex = McpLog.ItemCount - 1 THEN McpLogEnd
END SUB

' (the first line of the pane and the status bar's panel: on or off, who is in)
SUB McpShowState
  DIM head AS STRING, n AS INTEGER, names AS STRING, k AS INTEGER, line AS STRING
  IF Mcp.Listening THEN
    n = Mcp.ClientCount
    FOR k = 1 TO n
      line = FIELD$(Mcp.Clients, CHR$(10), k)
      IF names <> "" THEN names = names + ", "
      names = names + FIELD$(line, CHR$(9), 1)
    NEXT
    IF n = 0 THEN
      head = TR$("AI apps: on, none connected")
      McpSub.Caption = TR$("Waiting at {url} (this computer only)", "url", McpUrl)
    ELSE
      head = TR$("AI apps: {names}", "names", names)
      IF McpLastWhat <> "" THEN McpSub.Caption = McpLastWhat ELSE McpSub.Caption = TRN$("{n} call", "{n} calls", McpCalls)
    END IF
    McpStopBtn.Caption = TR$("&Stop")
  ELSE
    head = TR$("AI apps: off")
    IF LockOffApps THEN
      McpSub.Caption = TR$("School Lock is on: AI apps can't connect.")
    ELSE
      McpSub.Caption = TR$("Tools > Settings > AI Apps turns them on.")
    END IF
    McpStopBtn.Caption = TR$("&Turn On...")
  END IF
  McpHead.Caption = head
  McpStatusShow
END SUB

SUB McpStatusShow
  DIM t AS STRING
  IF Mcp.Listening = 0 AND McpStatusPanel = 0 THEN EXIT SUB
  IF McpStatusPanel = 0 THEN
    Status.AddPanels ""
    McpStatusPanel = 1
    LayOutStatus
  END IF
  IF Mcp.Listening = 0 THEN
    t = TR$("AI apps: off")
  ELSEIF McpMode = "all" THEN
    t = TR$("AI apps: allow everything")
  ELSEIF McpWaitCount > 0 THEN
    t = TR$("AI apps: waiting for you")
  ELSEIF Mcp.ClientCount = 0 THEN
    t = TR$("AI apps: on")
  ELSE
    t = TR$("AI apps: {what}", "what", McpLastWhat)
    IF McpLastWhat = "" THEN t = TR$("AI apps: connected")
  END IF
  Status.Panel(6).Caption = FitMiddle(t, 196)
END SUB

' (chrome.inc's LayOutStatus: the room AI apps' panel takes, 0 when it isn't there)
FUNCTION McpStatusWidth AS INTEGER
  IF McpStatusPanel THEN Result = 220 ELSE Result = 0
END FUNCTION

' A line in the pane (and the audit log): when, which AI app, what, how.
SUB McpLogLine(Client AS STRING, Tool AS STRING, What AS STRING, Target AS STRING, Outcome AS STRING)
  DIM line AS STRING
  line = LEFT$(TIME$, 8) + "  " + Client + "  " + What
  IF Outcome <> "" THEN line = line + "  — " + McpOutcomeShown(Outcome)
  McpLog.AddItems line
  IF McpLog.ItemCount > 500 THEN McpLog.DelItems 0
  McpLogEnd
  McpTools.AuditCall Client, Tool, Target, Outcome
  McpLastWhat = Client + ": " + What
  ' (a new project took the mode of the one before: said now, after the
  ' call that opened it — the pane's line and the status bar)
  IF McpCarrySaid <> "" THEN
    McpLog.AddItems LEFT$(TIME$, 8) + "  " + McpCarrySaid
    McpLogEnd
    McpLastWhat = McpCarrySaid
    Application.Announce McpCarrySaid
    McpCarrySaid = ""
  END IF
  McpShowState
END SUB

' How the pane says an outcome. McpLogLine's Outcome stays English — it goes to
' the audit log as it is — and the pane shows these words in Studio's language.
FUNCTION McpOutcomeShown(Outcome AS STRING) AS STRING
  DIM why AS STRING
  SELECT CASE Outcome
    CASE "ok": Result = TR$("ok")
    CASE "error": Result = TR$("error")
    CASE "applied": Result = TR$("applied")
    CASE "applied (you allowed it)": Result = TR$("applied (you allowed it)")
    CASE "denied by you": Result = TR$("denied by you")
    CASE "refused": Result = TR$("refused")
    CASE "couldn't run": Result = TR$("couldn't run")
    CASE "started": Result = TR$("started")
    CASE "sent": Result = TR$("sent")
    CASE "RAI didn't start": Result = TR$("RAI didn't start")
    CASE ELSE
      IF LEFT$(Outcome, 9) = "refused: " THEN
        ' (Read only, or the reason the tool gave)
        why = MID$(Outcome, 10)
        IF why = "Read only" THEN why = McpModeName("read")
        Result = TR$("refused: {why}", "why", why)
      ELSEIF LEFT$(Outcome, 9) = "refused (" AND RIGHT$(Outcome, 6) = " open)" THEN
        Result = TR$("refused ({dialog} open)", "dialog", McpDialogShown(MID$(Outcome, 10, LEN(Outcome) - 15)))
      ELSE
        Result = Outcome
      END IF
  END SELECT
END FUNCTION

' (McpDialogOpen's name of the window that is open, in Studio's language, for the pane)
FUNCTION McpDialogShown(Dialog AS STRING) AS STRING
  SELECT CASE Dialog
    CASE "Settings": Result = TR$("Settings")
    CASE "Set Up RAI": Result = TR$("Set Up RAI")
    CASE "a question": Result = TR$("a question")
    CASE "a review window": Result = TR$("a review window")
    CASE "a dialog": Result = TR$("a dialog")
    CASE "a value's window": Result = TR$("a value's window")
    CASE ELSE: Result = Dialog
  END SELECT
END FUNCTION

' (shown the first time an AI app works, as the Output's neighbour — never
' taking the keyboard from the user)
SUB McpPaneShow
  IF Dock.PaneState("mcppanel") = "hidden" THEN Dock.ShowPane("mcppanel")
  McpLayOutPane
END SUB

' (the activity's newest line picked and in view: the list is a whole
' number of rows tall — McpLayOutPane —, so no half row shows at its top)
SUB McpLogEnd
  IF McpLog.ItemCount = 0 THEN EXIT SUB
  McpLog.ItemIndex = McpLog.ItemCount - 1
END SUB

SUB McpClientIn(Client AS STRING)
  McpPaneShow
  McpLog.AddItems LEFT$(TIME$, 8) + "  " + TR$("{client} connected", "client", Client)
  McpTools.AuditCall Client, "connect", "", "ok"
  Application.Announce TR$("{client} connected to Studio", "client", Client)
  McpShowState
END SUB

SUB McpClientOut(Client AS STRING)
  McpLog.AddItems LEFT$(TIME$, 8) + "  " + TR$("{client} left", "client", Client)
  McpTools.AuditCall Client, "disconnect", "", "ok"
  McpShowState
END SUB

' ---- the calls ----------------------------------------------------------------------

' An AI app's call: its kind's choice — allowed, asked, never — then done.
SUB McpToolCalled(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER)
  DIM kind AS STRING, perm AS STRING, client AS STRING, what AS STRING
  client = Mcp.ClientName
  McpCalls = McpCalls + 1
  ' (a crash report's last calls: the tool's name only — crash.inc)
  Proj.CrashCall "MCP: " + Tool
  McpPaneShow
  kind = McpKindOf(Tool)
  ' (the open project's mode)
  McpModeFollow
  perm = McpDecision(kind)
  McpSyncProject
  what = McpTools.ToolActivity(Tool, Arguments)
  IF perm = "refuse" THEN
    Mcp.Reply CallId, "Not allowed: RapidR Studio lets AI apps only look at this project now, and not at the running program (its user chose Read only in the AI Apps pane), so nothing can be changed, run, looked at in the program or used. Tell the user what you would do instead; they can choose another mode there.", 1
    McpLogLine client, Tool, what, "", "refused: Read only"
    EXIT SUB
  END IF
  ' (a change to files is made first — nothing is written — so the user
  ' decides on what it really is; a run, RAI, a property and a project to
  ' open are asked first)
  IF kind = "change" AND Tool <> "open_project" AND Tool <> "set_property" THEN
    McpProposeThen Tool, Arguments, CallId, client, what, perm
    EXIT SUB
  END IF
  IF Tool = "open_project" THEN
    ' (a path that can't be opened is said at once: the user isn't asked)
    what = McpOpenCheck(Mcp.Argument("Path"))
    IF what <> "" THEN
      Mcp.Reply CallId, what, 1
      McpLogLine client, Tool, TR$("Opening a project"), "", "refused"
      EXIT SUB
    END IF
    what = TR$("Opening {path}", "path", McpShownPath(McpFullPath(Mcp.Argument("Path"))))
    ' (outside the Projects folder, the examples and the recent list: asked
    ' in every mode)
    IF McpOpenAllowed(Mcp.Argument("Path")) = 0 THEN perm = "ask"
  END IF
  IF perm = "allow" THEN
    McpRun Tool, Arguments, CallId, client, what
  ELSE
    McpWait Tool, Arguments, CallId, client, kind, what
  END IF
END SUB

' (a change: proposed — its errors back to the AI app at once —, then
' applied as the user's choice says)
SUB McpProposeThen(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING, Perm AS STRING)
  DIM answer AS STRING
  answer = McpTools.CallTool(Tool, Arguments)
  IF McpTools.CallFailed THEN
    Mcp.Reply CallId, answer, 1
    McpLogLine Client, Tool, What, McpTools.CallTarget, "refused: " + McpTools.CallNote
    EXIT SUB
  END IF
  IF Perm = "allow" THEN
    Mcp.Reply CallId, McpApply(answer), 0
    McpLogLine Client, Tool, What, McpTools.CallTarget, "applied"
  ELSE
    McpWait Tool, answer, CallId, Client, "change", What
  END IF
END SUB

' (a call waits for the user: shown in the pane, said, its AI app told)
SUB McpWait(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, Kind AS STRING, What AS STRING)
  DIM n AS INTEGER
  IF McpWaitCount >= MCPWAITS THEN
    Mcp.Reply CallId, "Too many calls are waiting for the user: wait for their answers first.", 1
    EXIT SUB
  END IF
  n = McpWaitCount
  McpWaitCall(n) = CallId
  McpWaitTool(n) = Tool
  McpWaitArgs(n) = Arguments
  McpWaitKind(n) = Kind
  McpWaitClient(n) = Client
  McpWaitWhat(n) = What
  McpWaitCount = n + 1
  Mcp.Progress CallId, "Waiting for the user to allow it in RapidR Studio."
  McpLog.AddItems LEFT$(TIME$, 8) + "  " + Client + "  " + What + "  — " + TR$("waiting for you")
  McpLogEnd
  McpLastWhat = TR$("{client}: {what} (waiting for you)", "client", Client, "what", What)
  McpShowAsk
  ' (in front, where the user sees it — the keyboard stays where it is)
  Dock.ShowPane("mcppanel")
  Application.Announce TR$("{client} asks: {what}. Allow it in the AI Apps pane.", "client", Client, "what", What), 1
END SUB

' (the first call waiting, in the pane, with its buttons)
SUB McpShowAsk
  DIM t AS STRING, k AS INTEGER, files AS STRING, info AS STRING
  IF McpWaitCount = 0 THEN
    McpAsk.Visible = 0
    McpLayOutPane
    McpShowState
    EXIT SUB
  END IF
  SELECT CASE McpWaitKind(0)
    CASE "change"
      IF McpWaitTool(0) = "open_project" THEN
        t = TR$("{client} wants to open {what}.", "client", McpWaitClient(0), "what", McpWaitWhat(0))
      ELSE
        FOR k = 0 TO McpTools.ProposalCount - 1
          info = McpTools.ProposalInfo(k)
          IF FIELD$(info, CHR$(9), 6) = "" THEN
            IF files <> "" THEN files = files + ", "
            files = files + FIELD$(info, CHR$(9), 2)
          END IF
        NEXT
        t = TR$("{client} wants to change {files}: {summary}", "client", McpWaitClient(0), "files", files, "summary", FIELD$(McpTools.ProposalInfo(0), CHR$(9), 3))
      END IF
    CASE "run": t = TR$("{client} wants to run your program ({what}).", "client", McpWaitClient(0), "what", McpWaitWhat(0))
    ' (mcpapp.inc: what Allow Once covers, and what it may see)
    CASE "drive": t = TR$("{client} wants to look at or use your running program: {what}. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data.", "client", McpWaitClient(0), "what", McpWaitWhat(0))
    CASE "rai": t = TR$("{client} wants to ask RAI: {prompt}", "client", McpWaitClient(0), "prompt", LEFT$(Mcp.Argument("Prompt", McpWaitCall(0)), 160))
    CASE ELSE: t = McpWaitClient(0) + ": " + McpWaitWhat(0)
  END SELECT
  IF McpWaitCount > 1 THEN t = TR$("{question} ({n} more waiting)", "question", t, "n", McpWaitCount - 1)
  McpAskText.Caption = t
  McpReviewBtn.Visible = Flag(McpWaitKind(0) = "change" AND McpWaitTool(0) <> "open_project")
  ' (under the pane's first line: placed there before it shows)
  McpAsk.Top = McpTop.Top + McpTop.Height
  McpAsk.Visible = 1
  McpLayOutPane
  McpShowState
END SUB

SUB McpOnceClick
  McpDecide 1
END SUB

SUB McpSessionClick
  McpDecide 2
END SUB

SUB McpDenyClick
  McpDecide 0
END SUB

' The user's answer to the first call waiting: 0 deny, 1 allow once, 2
' allow its kind for this session.
SUB McpDecide(Choice AS INTEGER)
  DIM tool AS STRING, args AS STRING, callId AS INTEGER, client AS STRING, kind AS STRING, what AS STRING, k AS INTEGER
  IF McpWaitCount = 0 THEN EXIT SUB
  tool = McpWaitTool(0): args = McpWaitArgs(0): callId = McpWaitCall(0)
  client = McpWaitClient(0): kind = McpWaitKind(0): what = McpWaitWhat(0)
  FOR k = 0 TO McpWaitCount - 2
    McpWaitCall(k) = McpWaitCall(k + 1)
    McpWaitTool(k) = McpWaitTool(k + 1)
    McpWaitArgs(k) = McpWaitArgs(k + 1)
    McpWaitKind(k) = McpWaitKind(k + 1)
    McpWaitClient(k) = McpWaitClient(k + 1)
    McpWaitWhat(k) = McpWaitWhat(k + 1)
  NEXT
  McpWaitCount = McpWaitCount - 1
  IF Choice = 2 AND INSTR(McpGrants, "|" + kind + "|") = 0 THEN McpGrants = McpGrants + "|" + kind + "|"
  ' (mcpapp.inc: the running program allowed — until this run ends)
  IF Choice > 0 AND kind = "drive" THEN McpAppRunAllowed = RunStarted
  IF Choice = 0 THEN
    IF kind = "change" AND tool <> "open_project" THEN McpRejectAll
    Mcp.Reply callId, "The user didn't allow this in RapidR Studio. Ask them what they want instead.", 1
    McpLogLine client, tool, what, "", "denied by you"
  ELSEIF kind = "change" AND tool <> "open_project" THEN
    Mcp.Reply callId, McpApply(args), 0
    McpLogLine client, tool, what, McpTools.CallTarget, "applied (you allowed it)"
  ELSE
    McpRun tool, args, callId, client, what
  END IF
  McpShowAsk
END SUB

' The changes waiting in McpTools, applied as the assistant's are: each file
' changed in its editor as one undo step (opened when it wasn't; saved by
' the user, as any change), a new file written and added to the project, a
' new program created and opened. What the AI app is told.
FUNCTION McpApply(Said AS STRING) AS STRING
  DIM i AS INTEGER, info AS STRING, kind AS STRING, path AS STRING, full AS STRING, d AS INTEGER
  DIM newText AS STRING, patch AS STRING, done AS STRING, f AS RFileStream, made AS STRING, problems AS STRING, dirty AS STRING, before AS STRING, beforeKey AS STRING
  FOR i = 0 TO McpTools.ProposalCount - 1
    info = McpTools.ProposalInfo(i)
    kind = FIELD$(info, CHR$(9), 1)
    path = FIELD$(info, CHR$(9), 2)
    IF FIELD$(info, CHR$(9), 6) <> "" THEN
      ' (decided already)
    ELSEIF kind = "project" THEN
      IF made = "" THEN
        dirty = McpDirtyFiles
        IF dirty <> "" THEN
          ' (opening the new program would ask the user to save these: found by tools/ai_eval's
          ' outside-AI-app runs, where Studio then answered nothing, to any call, for good)
          problems = problems + "The new program wasn't made: Studio has unsaved changes (" + dirty + "), and opening another program would ask the user to save them. Put the program in the open project with write_file or propose_edit, or ask the user to save or close them first. "
          made = "!"
        ELSE
          made = McpTools.CreateProject(DefaultFolder)
          IF made = "" OR LEFT$(made, 6) = "error:" THEN
            problems = problems + "The new program couldn't be created: " + MID$(made, 8) + ". "
            made = "!"
          END IF
        END IF
      END IF
      ' (a new program that wasn't made is dropped, not kept for the next change: kept, whatever
      ' change came next made it — and opened it over that change's unsaved text, where Studio
      ' waited on a Save dialog for good: found again by the 2026-10-29 runs, quiz-game)
      IF made = "!" THEN McpTools.Decide i, "failed"
    ELSE
      full = ProjectPath(path)
      newText = McpTools.ProposalNew(i)
      IF kind = "create" AND FILEEXISTS(full) = 0 THEN
        IF FolderOf(full) <> "" AND DIREXISTS(FolderOf(full)) = 0 THEN MKDIR FolderOf(full)
        f.Open(full, 65535)
        f.WriteStr(newText, LEN(newText))
        f.Close
        IF Proj.AddFile(full) THEN
          IF Proj.Kind = "project" THEN SaveProjectFile
        END IF
        FillProjectTree
        OpenDocument full
        McpTools.Decide i, "applied"
        done = done + path + " "
      ELSE
        d = DocOf(full)
        IF d < 0 THEN
          OpenDocument full
          d = DocOf(full)
        END IF
        IF d < 0 THEN
          problems = problems + path + " couldn't be opened to change it. "
          McpTools.Decide i, "failed"
        ELSEIF AIMatches(CodeDoc(d).Text, McpTools.ProposalOld(i)) = 0 THEN
          problems = problems + path + " changed since you read it: read it again and propose the change again. "
          McpTools.Decide i, "stale"
        ELSE
          patch = McpTools.Patch(CodeDoc(d).Text, newText)
          IF patch <> "" AND CodeDoc(d).ApplyPatches(patch, 0) = 0 THEN
            problems = problems + path + "'s editor refused the change. "
            McpTools.Decide i, "failed"
          ELSE
            ProjectFileChanged d
            McpTools.Decide i, "applied"
            done = done + path + " "
          END IF
        END IF
      END IF
    END IF
  NEXT
  McpTools.ClearDecided
  IF made <> "" AND made <> "!" THEN
    McpTools.ClearProposals
    ' (the files this very call changed are unsaved now: opening over them would ask the user
    ' to save them, and Studio would answer nothing until they did)
    dirty = McpDirtyFiles
    IF dirty <> "" THEN
      Result = "The new program was written (" + McpShownPath(made) + ") but not opened: Studio has unsaved changes (" + dirty + "), and opening it would ask the user to save them. Open it with open_project once they are saved."
      EXIT FUNCTION
    END IF
    IF ProjectOpen THEN before = McpMode ELSE before = ""
    beforeKey = ProjectFile
    OpenProject made
    McpCarryMode before, beforeKey
    Result = "Created and opened the new program " + McpTools.CreatedTitle + " (" + FileTitle(made) + "). " + Said
    EXIT FUNCTION
  END IF
  IF done <> "" THEN AnalyzeActive
  ' (one clear sentence about the change: "Changed main.rr in Studio's editor
  ' (not on disk yet …)" — not that and "Proposed … they decide" together)
  IF problems <> "" AND done = "" THEN
    Result = "Not changed: " + problems
  ELSEIF problems <> "" THEN
    Result = "Changed " + RTRIM$(done) + " in Studio's editor (not on disk yet: Run saves it). Not changed: " + problems
  ELSE
    Result = McpTools.AppliedText(Said, RTRIM$(done))
  END IF
END FUNCTION

' (denied: every change waiting is set aside, said nowhere else)
SUB McpRejectAll
  DIM i AS INTEGER
  FOR i = 0 TO McpTools.ProposalCount - 1
    McpTools.Decide i, "rejected"
  NEXT
  McpTools.ClearProposals
END SUB

' ---- reviewing the change (Review...) --------------------------------------------------

SUB McpReviewClick
  DIM i AS INTEGER, info AS STRING
  IF McpWaitCount = 0 THEN EXIT SUB
  McpRevList.Clear
  McpRevCount = 0
  FOR i = 0 TO McpTools.ProposalCount - 1
    info = McpTools.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 6) = "" AND McpRevCount < 32 THEN
      McpRevIndex(McpRevCount) = i
      McpRevCount = McpRevCount + 1
      McpRevList.AddItems FIELD$(info, CHR$(9), 2)
    END IF
  NEXT
  IF McpRevCount = 0 THEN EXIT SUB
  McpRevAt = 0
  McpRevList.ItemIndex = 0
  McpRevPicked
  McpRevApplyBtn.Width = 96: McpRevApplyBtn.Height = 30: McpRevApplyBtn.Top = 13
  McpRevRejectBtn.Width = 96: McpRevRejectBtn.Height = 30: McpRevRejectBtn.Top = 13
  McpRevApplyBtn.Left = McpRevFoot.ClientWidth - 212
  McpRevRejectBtn.Left = McpRevFoot.ClientWidth - 108
  McpRevDecided = -1
  McpReview.Center
  McpReview.ShowModal
  IF McpRevDecided = 1 THEN McpDecide 1
  IF McpRevDecided = 0 THEN McpDecide 0
END SUB

SUB McpRevPicked
  DIM i AS INTEGER, info AS STRING
  IF McpRevList.ItemIndex < 0 THEN EXIT SUB
  i = McpRevIndex(McpRevList.ItemIndex)
  info = McpTools.ProposalInfo(i)
  McpRevHead.Caption = "  " + FIELD$(info, CHR$(9), 2) + ": " + FIELD$(info, CHR$(9), 3)
  McpRevDiff.Language = FIELD$(info, CHR$(9), 2)
  McpRevDiff.LeftText = McpTools.ProposalOld(i)
  McpRevDiff.RightText = McpTools.ProposalNew(i)
END SUB

SUB McpRevApply
  McpRevDecided = 1
  McpReview.Close
END SUB

SUB McpRevReject
  McpRevDecided = 0
  McpReview.Close
END SUB

' ---- the tools Studio adds ----------------------------------------------------------------

' A call allowed: run now (or later: run_program and ask_rai answer when
' they are done).
SUB McpRun(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM answer AS STRING, failed AS INTEGER
  SELECT CASE Tool
    CASE "studio_status": answer = McpStatusText
    CASE "list_documents": answer = McpDocumentsText
    CASE "get_problems"
      answer = McpTools.CallTool("check_code", "{}")
      failed = McpTools.CallFailed
    CASE "read_output": answer = McpReadOutput(Flag(LCASE$(Mcp.Argument("All", CallId)) = "true"))
    CASE "get_rai_steps": answer = McpRaiStepsText
    CASE "get_rai_transcript": answer = McpRaiTranscript(VAL(Mcp.Argument("Chars", CallId)))
    CASE "get_rai_audit": answer = McpRaiAudit(VAL(Mcp.Argument("Count", CallId)))
    CASE "open_project"
      answer = McpOpenProject(Mcp.Argument("Path", CallId))
      failed = Flag(LEFT$(answer, 7) = "Can't o")
    CASE "run_program"
      McpRunProgram CallId, Client, What
      EXIT SUB
    CASE "debug"
      McpDebug CallId, Client, What
      EXIT SUB
    CASE "layout_report"
      McpSyncProject
      answer = McpTools.LayoutReport
    CASE "capture"
      McpCapture CallId, Client
      EXIT SUB
    ' (mcpapp.inc: the program answers through Session's OnApp)
    CASE "app_layout", "app_screenshot", "app_click", "app_type", "app_key", "app_drag", "app_scroll", "app_wait"
      McpAppRun Tool, CallId, Client, What
      EXIT SUB
    CASE "set_property"
      answer = McpSetProperty(Mcp.Argument("Component", CallId), Mcp.Argument("Property", CallId), Mcp.Argument("Value", CallId))
      failed = Flag(LEFT$(answer, 4) = "Can'")
    CASE "ask_rai"
      McpAskRai CallId, Client, What
      EXIT SUB
    CASE ELSE
      ' (the assistant's tools)
      answer = McpTools.CallTool(Tool, Arguments)
      failed = McpTools.CallFailed
      Mcp.Reply CallId, answer, failed
      IF failed THEN McpLogLine Client, Tool, What, McpTools.CallTarget, "error" ELSE McpLogLine Client, Tool, What, McpTools.CallTarget, "ok"
      EXIT SUB
  END SELECT
  Mcp.Reply CallId, answer, failed
  IF failed THEN McpLogLine Client, Tool, What, "", "error" ELSE McpLogLine Client, Tool, What, "", "ok"
END SUB

' (~ for the home folder: an AI app needs no user name)
FUNCTION McpShownPath(Path AS STRING) AS STRING
  DIM home AS STRING
  home = ENVIRON$("HOME")
  IF home = "" THEN home = ENVIRON$("USERPROFILE")
  home = REPLACESUBSTR$(home, "\", "/")
  IF home <> "" AND LEFT$(REPLACESUBSTR$(Path, "\", "/"), LEN(home)) = home THEN Result = "~" + MID$(REPLACESUBSTR$(Path, "\", "/"), LEN(home) + 1) ELSE Result = Path
END FUNCTION


FUNCTION McpStatusText AS STRING
  DIM t AS STRING, nl AS STRING, i AS INTEGER, f AS STRING, d AS INTEGER, docs AS STRING, n AS INTEGER, info AS STRING
  nl = CHR$(10)
  t = "RapidR Studio " + Lang.Version + nl
  IF ProjectOpen THEN
    FOR i = 0 TO Proj.FileCount - 1
      IF f <> "" THEN f = f + ", "
      f = f + Proj.File(i)
    NEXT
    t = t + "Project: " + ProjectName + " (" + McpShownPath(Proj.Folder) + "); main file " + Proj.MainFile + "; files: " + f + nl
  ELSE
    t = t + "Project: none open (open_project opens one; examples/gui/hello_form.rr is one of RapidR's examples)" + nl
  END IF
  t = t + "Documents: " + McpDocumentsText + nl
  SELECT CASE RunState
    CASE "running": t = t + "Program: running" + nl
    CASE "paused": t = t + "Program: paused (debugger) at " + FileTitle(PausedFile) + ":" + Num(PausedLine) + nl
    CASE "error": t = t + "Program: not running (" + StateText + ")" + nl
    CASE ELSE: t = t + "Program: not running" + nl
  END SELECT
  IF ProjectOpen THEN t = t + "Errors: " + Num(ProgramErrors) + " (get_problems lists them)" + nl
  IF AIReady THEN
    IF AI.Busy THEN t = t + "RAI: answering (" + AI.Activity + ")" + nl ELSE t = t + "RAI: ready (" + AI.Provider + ", " + AI.Model + ")" + nl
  ELSE
    t = t + "RAI: not set up (the user sets it up in Studio)" + nl
  END IF
  FOR i = 0 TO McpTools.ProposalCount - 1
    info = McpTools.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 6) = "" THEN n = n + 1
  NEXT
  IF n > 0 THEN t = t + "Your changes waiting for the user: " + Num(n) + nl
  ' (why capture says no for now)
  IF McpDialogOpen <> "" THEN t = t + "Open in Studio: " + McpDialogOpen + " (no picture of Studio until it closes)" + nl
  t = t + "Permission: " + McpModeNameEn(McpMode) + " — " + McpModeNoteEn(McpMode)
  IF McpGrants <> "" AND McpMode = "ask" THEN t = t + " Allowed for this session: " + REPLACESUBSTR$(MID$(McpGrants, 2, LEN(McpGrants) - 2), "||", ", ") + "."
  ' (what to do next: an empty project says "call find_example first")
  t = t + nl + McpTools.StatusNext
  Result = t
END FUNCTION

FUNCTION McpRel(Path AS STRING) AS STRING
  DIM p AS STRING, f AS STRING
  p = REPLACESUBSTR$(Path, "\", "/")
  f = REPLACESUBSTR$(Proj.Folder, "\", "/")
  IF f <> "" AND LEFT$(p, LEN(f) + 1) = f + "/" THEN Result = MID$(p, LEN(f) + 2) ELSE Result = McpShownPath(p)
END FUNCTION

FUNCTION McpDocumentsText AS STRING
  DIM d AS INTEGER, t AS STRING, one AS STRING
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      one = McpRel(DocFile(d))
      IF d = ActiveDoc THEN one = one + " (active"
      IF d <> ActiveDoc THEN one = one + " ("
      IF DocHasForm(d) AND DocViewNow(d) = "design" THEN one = one + ", Design view" ELSE one = one + ", Code view"
      IF DocDirty(d) THEN one = one + ", unsaved changes"
      one = REPLACESUBSTR$(one, "(, ", "(") + ")"
      IF t <> "" THEN t = t + "; "
      t = t + one
    END IF
  NEXT
  IF t = "" THEN t = "none open"
  Result = t
END FUNCTION

' ---- open_project ------------------------------------------------------------------------

' (a full path for what an AI app named: absolute, ~/…, examples/…, or
' inside the Projects folder)
FUNCTION McpFullPath(Path AS STRING) AS STRING
  DIM p AS STRING, home AS STRING
  p = REPLACESUBSTR$(LTRIM$(RTRIM$(Path)), "\", "/")
  home = REPLACESUBSTR$(ENVIRON$("HOME"), "\", "/")
  IF LEFT$(p, 2) = "~/" AND home <> "" THEN p = home + MID$(p, 2)
  IF LEFT$(p, 9) = "examples/" THEN p = HomeDir + p
  IF LEFT$(p, 1) <> "/" AND MID$(p, 2, 1) <> ":" THEN p = DefaultFolder + "/" + p
  Result = REPLACESUBSTR$(p, "//", "/")
END FUNCTION

' (inside a folder, for real: no .. step out of it)
FUNCTION McpInside(Path AS STRING, Folder AS STRING) AS INTEGER
  DIM f AS STRING
  f = REPLACESUBSTR$(Folder, "\", "/")
  IF RIGHT$(f, 1) <> "/" THEN f = f + "/"
  Result = Flag(f <> "/" AND LEFT$(Path, LEN(f)) = f AND INSTR(Path, "/../") = 0 AND RIGHT$(Path, 3) <> "/..")
END FUNCTION

' Whether open_project may open Path without asking: inside the Projects
' folder, RapidR's examples, or on the recent list.
FUNCTION McpOpenAllowed(Path AS STRING) AS INTEGER
  DIM p AS STRING
  p = McpFullPath(Path)
  Result = Flag(McpInside(p, DefaultFolder) OR McpInside(p, HomeDir + "examples") OR INSTR(CHR$(10) + RecentList + CHR$(10), CHR$(10) + p + CHR$(10)) > 0)
END FUNCTION

' Why open_project can't open Path ("" when it can).
FUNCTION McpOpenCheck(Path AS STRING) AS STRING
  DIM p AS STRING, ext AS STRING
  p = McpFullPath(Path)
  ext = LCASE$(FileExt(p))
  IF ext <> "rrproj" AND ext <> "rr" AND ext <> "bas" AND ext <> "rqw" AND ext <> "rqb" THEN
    Result = "Can't open " + Path + ": open_project opens a project (.rrproj) or a program (.rr, .bas)."
  ELSEIF INSTR(p + "/", "/../") > 0 THEN
    Result = "Can't open " + Path + ": give the path without .. in it."
  ELSEIF FILEEXISTS(p) = 0 THEN
    Result = "Can't open " + Path + ": there is no such file (" + McpShownPath(p) + ")."
  ELSE
    Result = ""
  END IF
END FUNCTION

' The files with unsaved changes in Studio's editor, "a.rr b.rr", "" when there are none:
' opening another project would ask the user to save them, a question nobody answers while an
' AI app waits (and every call after it waits too).
FUNCTION McpDirtyFiles AS STRING
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN Result = Result + McpRel(DocFile(d)) + " "
  NEXT
  Result = RTRIM$(Result)
END FUNCTION

FUNCTION McpOpenProject(Path AS STRING) AS STRING
  DIM p AS STRING, dirty AS STRING, before AS STRING, beforeKey AS STRING
  p = McpFullPath(Path)
  Result = McpOpenCheck(Path)
  IF Result <> "" THEN EXIT FUNCTION
  dirty = McpDirtyFiles
  IF dirty <> "" THEN
    Result = "Can't open " + Path + ": Studio has unsaved changes (" + dirty + "). Ask the user to save or close them first: open_project never discards them."
    EXIT FUNCTION
  END IF
  IF ProjectOpen THEN before = McpMode ELSE before = ""
  beforeKey = ProjectFile
  OpenProject p
  IF ProjectOpen = 0 THEN
    Result = "Can't open " + Path + ": " + StateText
    EXIT FUNCTION
  END IF
  McpCarryMode before, beforeKey
  McpSyncProject
  Result = "Opened. " + McpStatusText
END FUNCTION

' ---- run_program and read_output -----------------------------------------------------------

' (debug.inc's ProgramOutput: the program's output, kept for read_output)
SUB McpProgramOutput(Text AS STRING)
  IF Mcp.Listening = 0 THEN EXIT SUB
  McpOutput = McpOutput + Text
  IF LEN(McpOutput) > MCP_OUTPUT_KEPT THEN
    McpOutputDropped = McpOutputDropped + LEN(McpOutput) - MCP_OUTPUT_KEPT
    McpOutput = RIGHT$(McpOutput, MCP_OUTPUT_KEPT)
  END IF
END SUB

FUNCTION McpRunWords AS STRING
  SELECT CASE RunState
    CASE "running": Result = "The program is running."
    CASE "paused": Result = "The program is paused in the debugger at " + FileTitle(PausedFile) + ":" + Num(PausedLine) + "."
    CASE "error": Result = "The program isn't running: " + StateText + "."
    CASE ELSE: Result = "The program isn't running."
  END SELECT
END FUNCTION

FUNCTION McpOutputFrom(At AS LONG) AS STRING
  DIM from AS LONG
  from = At - McpOutputDropped
  IF from < 0 THEN from = 0
  Result = MID$(McpOutput, from + 1)
END FUNCTION

FUNCTION McpReadOutput(All AS INTEGER) AS STRING
  DIM t AS STRING
  IF All THEN t = McpOutputFrom(0) ELSE t = McpOutputFrom(McpOutputRead)
  McpOutputRead = McpOutputDropped + LEN(McpOutput)
  IF t = "" THEN t = "(no new output)"
  Result = McpRunWords + CHR$(10) + "Output:" + CHR$(10) + t
END FUNCTION

SUB McpRunProgram(CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM action AS STRING, w AS DOUBLE
  action = LCASE$(Mcp.Argument("Action", CallId))
  IF action = "" THEN action = "start"
  w = VAL(Mcp.Argument("Wait", CallId))
  IF Mcp.Argument("Wait", CallId) = "" THEN w = 3
  IF w < 0 THEN w = 0
  IF w > 60 THEN w = 60
  SELECT CASE action
    CASE "status"
      Mcp.Reply CallId, McpReadOutput(0)
      McpLogLine Client, "run_program", TR$("Program status"), "", "ok"
      EXIT SUB
    CASE "stop"
      IF RunState = "running" OR RunState = "paused" THEN RunCommand "run.stop"
      Mcp.Reply CallId, "Stopped. " + McpReadOutput(0)
      McpLogLine Client, "run_program", TR$("Stopped the program"), "", "ok"
      EXIT SUB
    CASE "start", "restart"
      IF ProjectOpen = 0 THEN
        Mcp.Reply CallId, "No project is open: open_project first.", 1
        EXIT SUB
      END IF
      IF action = "restart" AND (RunState = "running" OR RunState = "paused") THEN RunCommand "run.stop"
      IF RunState = "running" OR RunState = "paused" THEN
        Mcp.Reply CallId, "The program runs already: run_program with Action stop (or restart) first. " + McpReadOutput(0), 1
        EXIT SUB
      END IF
      McpOutput = "": McpOutputDropped = 0: McpOutputRead = 0
      IF LCASE$(Mcp.Argument("Debug", CallId)) = "true" THEN StartProgram 1 ELSE StartProgram 0
      IF RunState <> "running" AND RunState <> "paused" THEN
        Mcp.Reply CallId, "Studio couldn't run it: " + StateText + CHR$(10) + McpTools.CallTool("check_code", "{}"), 1
        McpLogLine Client, "run_program", What, "", "couldn't run"
        EXIT SUB
      END IF
      McpRunCall = CallId
      McpRunUntil = TIMER + w
      McpRunAction = action
      McpLogLine Client, "run_program", TR$("Running the program"), "", "started"
    CASE ELSE
      Mcp.Reply CallId, "Action is start, stop, restart or status.", 1
  END SELECT
END SUB

' ---- debug -----------------------------------------------------------------------------

SUB McpDebug(CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM action AS STRING, f AS STRING, full AS STRING, d AS INTEGER, ln AS INTEGER, i AS INTEGER, w AS DOUBLE
  action = LCASE$(Mcp.Argument("Action", CallId))
  w = VAL(Mcp.Argument("Wait", CallId))
  IF Mcp.Argument("Wait", CallId) = "" THEN w = 5
  IF w < 0 THEN w = 0
  IF w > 60 THEN w = 60
  SELECT CASE action
    CASE "breakpoint"
      f = Mcp.Argument("File", CallId)
      IF f = "" THEN f = Proj.MainFile
      ln = VAL(Mcp.Argument("Line", CallId))
      full = ProjectPath(f)
      IF ProjectOpen = 0 OR InProject(full) = 0 OR ln < 1 THEN
        Mcp.Reply CallId, "Can't: give File (one of the project's files) and Line (from 1).", 1
        EXIT SUB
      END IF
      d = DocOf(full)
      IF d < 0 THEN
        OpenDocument full
        d = DocOf(full)
      END IF
      IF d < 0 THEN
        Mcp.Reply CallId, "Can't open " + f + ".", 1
        EXIT SUB
      END IF
      i = FindBreakpoint(DocFile(d), ln)
      IF (LCASE$(Mcp.Argument("Remove", CallId)) = "true") = (i >= 0) THEN ToggleBreakpointAt d, ln
      Mcp.Reply CallId, StateText + ". " + McpDebugState
      McpLogLine Client, "debug", TR$(StateText), McpRel(full), "ok"
      EXIT SUB
    CASE "state"
      Mcp.Reply CallId, McpDebugState
      McpLogLine Client, "debug", TR$("Debugger state"), "", "ok"
      EXIT SUB
    CASE "continue": IF RunState = "paused" THEN RunCommand "run.continue"
    CASE "step_over": RunCommand "debug.stepOver"
    CASE "step_into": RunCommand "debug.stepInto"
    CASE "step_out": RunCommand "debug.stepOut"
    CASE "pause": RunCommand "run.pause"
    CASE ELSE
      Mcp.Reply CallId, "Action is breakpoint, continue, step_over, step_into, step_out, pause or state.", 1
      EXIT SUB
  END SELECT
  McpLogLine Client, "debug", TR$("Debugger: {action}", "action", action), "", "ok"
  McpDebugCall = CallId
  McpDebugUntil = TIMER + w
  McpDebugStops = LEN(StopLog)
  McpDebugSettle = 0
END SUB

' Where the program is, its call stack and its variables, in words.
FUNCTION McpDebugState AS STRING
  DIM t AS STRING, nl AS STRING, k AS INTEGER, bps AS STRING
  nl = CHR$(10)
  t = McpRunWords
  IF RunState = "paused" THEN
    t = t + nl + "Call stack:"
    FOR k = 0 TO FrameCount - 1
      t = t + nl + "  " + FrameName(k) + "  " + FileTitle(FrameFile(k)) + ":" + Num(FrameLine(k))
    NEXT
    t = t + nl + "Variables:"
    FOR k = 0 TO VarsTree.ItemCount - 1
      IF k > 60 THEN EXIT FOR
      t = t + nl + "  " + VarsTree.Item(k).Text
    NEXT
  END IF
  FOR k = 0 TO BpCount - 1
    IF bps <> "" THEN bps = bps + ", "
    bps = bps + McpRel(BpFile(k)) + ":" + Num(BpLine(k))
  NEXT
  IF bps = "" THEN bps = "none"
  t = t + nl + "Breakpoints: " + bps
  Result = t
END FUNCTION

' ---- capture and set_property ------------------------------------------------------------

' (the name of a Studio dialog that is open, "" when none is: Settings —
' its token and keys —, Set Up RAI — its key —, the questions, the review
' windows, the value editors)
FUNCTION McpDialogOpen AS STRING
  Result = ""
  IF SettingsDialog.Visible THEN Result = "Settings": EXIT FUNCTION
  IF AISetup.Visible THEN Result = "Set Up RAI": EXIT FUNCTION
  IF ConsentDialog.Visible OR SavePrompt.Visible OR FailNote.Visible THEN Result = "a question": EXIT FUNCTION
  IF McpReview.Visible OR AIReview.Visible OR AINew.Visible OR AISent.Visible THEN Result = "a review window": EXIT FUNCTION
  IF AppDialog.Visible OR NewDialog.Visible THEN Result = "a dialog": EXIT FUNCTION
  IF TextViewer.Visible OR ValueEditor.Visible THEN Result = "a value's window": EXIT FUNCTION
END FUNCTION

' A picture of Studio's window, as the user sees it now.
SUB McpCapture(CallId AS INTEGER, Client AS STRING)
  DIM f AS STRING
  ' (never while a dialog is open — Settings, a question, a review, Set Up
  ' RAI: one may show the token or a key — docs/legal/review-2026-10-29-mcp.md,
  ' addendum B; the picture is Studio's own window, drawn by Studio, never
  ' the screen around it)
  IF McpDialogOpen <> "" THEN
    Mcp.Reply CallId, "No picture now: " + McpDialogOpen + " is open in Studio, and it may show a key or the token. Ask again after the user closes it.", 1
    McpLogLine Client, "capture", TR$("A picture of Studio"), "", "refused (" + McpDialogOpen + " open)"
    EXIT SUB
  END IF
  f = ENVIRON$("TMPDIR")
  IF f = "" THEN f = ENVIRON$("TEMP")
  IF f = "" THEN f = "/tmp"
  IF RIGHT$(f, 1) = "/" OR RIGHT$(f, 1) = "\" THEN f = LEFT$(f, LEN(f) - 1)
  f = f + "/rapidr-studio-capture-" + Num(CallId) + ".png"
  IF Studio.SaveImage(f) = 0 THEN
    Mcp.Reply CallId, "Studio's window couldn't be captured.", 1
    McpLogLine Client, "capture", TR$("A picture of Studio"), "", "error"
    EXIT SUB
  END IF
  Mcp.ReplyImage CallId, f, "RapidR Studio now (" + StateText + ")."
  KILL f
  McpLogLine Client, "capture", TR$("A picture of Studio"), "", "ok"
END SUB

' A component's property set in its form's designer (opened when it isn't),
' as the Properties pane would: one undo step in its file.
FUNCTION McpSetProperty(Component AS STRING, Prop AS STRING, Value AS STRING) AS STRING
  DIM d AS INTEGER, i AS INTEGER, found AS INTEGER, listing AS STRING, file AS STRING, k AS INTEGER
  found = -1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocHasForm(d) AND found < 0 THEN
      FOR i = 0 TO DesignDoc(d).Count - 1
        IF LCASE$(DesignDoc(d).GetName(i)) = LCASE$(Component) THEN found = d: k = i
      NEXT
    END IF
  NEXT
  IF found < 0 THEN
    ' (its file, from the source: opened, then looked for again)
    McpSyncProject
    listing = McpTools.CallTool("rapidr_list", "{}")
    IF McpTools.CallFailed = 0 THEN
      McpJ.Parse listing
      i = 0
      WHILE McpJ.Has("content.objects." + Num(i) + ".id") AND file = ""
        IF LCASE$(McpJ.Get("content.objects." + Num(i) + ".id")) = LCASE$(Component) THEN file = McpJ.Get("content.objects." + Num(i) + ".file")
        i = i + 1
      WEND
      IF file <> "" THEN
        OpenDocument ProjectPath(file)
        d = DocOf(ProjectPath(file))
        IF d >= 0 THEN
          FOR i = 0 TO DesignDoc(d).Count - 1
            IF LCASE$(DesignDoc(d).GetName(i)) = LCASE$(Component) THEN found = d: k = i
          NEXT
        END IF
      END IF
    END IF
  END IF
  IF found < 0 THEN
    Result = "Can't find a component named " + Component + " in the project's forms (rapidr_list lists them)."
    EXIT FUNCTION
  END IF
  DesignDoc(found).SetProp k, Prop, Value
  ProjectFileChanged found
  Result = Component + "." + Prop + " is now " + DesignDoc(found).GetProp(k, Prop) + " (in " + McpRel(DocFile(found)) + ": Edit > Undo takes it back; Run saves it)."
END FUNCTION

' ---- ask_rai and RAI's state ------------------------------------------------------------

SUB McpAskRai(CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM prompt AS STRING
  prompt = Mcp.Argument("Prompt", CallId)
  IF AIReady = 0 THEN
    Mcp.Reply CallId, "RAI isn't set up in this Studio: the user sets it up (Tools > Set Up RAI).", 1
    EXIT SUB
  END IF
  IF AI.Busy THEN
    Mcp.Reply CallId, "RAI is answering already: wait for it (get_rai_steps), then ask again.", 1
    EXIT SUB
  END IF
  IF LTRIM$(RTRIM$(prompt)) = "" THEN
    Mcp.Reply CallId, "Give the Prompt to ask RAI.", 1
    EXIT SUB
  END IF
  Dock.ShowPane("aipanel")
  AISendNow prompt, 0
  IF AI.Busy = 0 THEN
    Mcp.Reply CallId, "RAI didn't start: " + AI.Error + " " + AI.Status, 1
    McpLogLine Client, "ask_rai", What, "", "RAI didn't start"
    EXIT SUB
  END IF
  McpLogLine Client, "ask_rai", TR$("Asking RAI: {prompt}", "prompt", LEFT$(prompt, 60)), "", "sent"
  IF LCASE$(Mcp.Argument("Wait", CallId)) = "false" THEN
    Mcp.Reply CallId, "Asked: RAI is answering. get_rai_steps says how it goes; get_rai_transcript has its answer."
    EXIT SUB
  END IF
  McpRaiCall = CallId
  McpRaiUntil = TIMER + 600
  McpRaiSaid = ""
END SUB

FUNCTION McpRaiStepsText AS STRING
  DIM t AS STRING, nl AS STRING
  nl = CHR$(10)
  IF AIReady = 0 THEN
    Result = "RAI isn't set up in this Studio."
    EXIT FUNCTION
  END IF
  IF AI.Busy THEN t = "RAI: answering — " + AI.Activity ELSE t = "RAI: idle"
  t = t + nl + "Status: " + AI.Status
  IF AI.Error <> "" THEN t = t + nl + "Error: " + AI.Error
  t = t + nl + "Steps:" + nl + AI.Steps
  IF AITools.ProposalCount > 0 THEN t = t + nl + "Changes RAI proposed, waiting for the user's review: " + Num(AITools.ProposalCount)
  Result = t
END FUNCTION

FUNCTION McpRaiTranscript(Chars AS INTEGER) AS STRING
  DIM t AS STRING
  IF Chars <= 0 THEN Chars = 8000
  t = AI.Transcript
  IF LEN(t) > Chars THEN t = "…" + RIGHT$(t, Chars)
  IF t = "" THEN t = "(RAI's chat is empty)"
  Result = t
END FUNCTION

FUNCTION McpRaiAudit(Count AS INTEGER) AS STRING
  IF Count <= 0 THEN Count = 30
  IF Count > 500 THEN Count = 500
  Result = "time, what, target, decision or result (tab-separated):" + CHR$(10) + AI.AuditLog(Count)
END FUNCTION

' ---- the timer: what waits for the program or RAI ---------------------------------------

SUB McpTickTimer(Sender AS RTimer)
  DIM t AS STRING
  ' (another project open: its own mode)
  McpModeFollow
  IF McpRunCall > 0 THEN
    IF RunState <> "running" OR TIMER >= McpRunUntil OR TIMER < McpRunUntil - 120 THEN
      t = McpReadOutput(1)
      IF RunState = "running" THEN t = t + CHR$(10) + "(It still runs: read_output for more, run_program Action stop to end it.)"
      IF RunState = "paused" THEN t = t + CHR$(10) + McpDebugState
      Mcp.Reply McpRunCall, t, Flag(RunState = "error")
      McpRunCall = 0
    END IF
  END IF
  IF McpDebugCall > 0 THEN
    ' (stopped again — a breakpoint, a step's end — or ended: a moment for
    ' the variables, then said)
    IF McpDebugSettle = 0 AND (LEN(StopLog) <> McpDebugStops OR (RunState <> "running" AND RunState <> "paused") OR TIMER >= McpDebugUntil) THEN McpDebugSettle = TIMER + 0.6
    IF McpDebugSettle > 0 AND TIMER >= McpDebugSettle THEN
      Mcp.Reply McpDebugCall, McpDebugState
      McpDebugCall = 0
    END IF
  END IF
  IF McpRaiCall > 0 THEN
    IF AI.Busy THEN
      IF AI.Activity <> McpRaiSaid THEN
        McpRaiSaid = AI.Activity
        Mcp.Progress McpRaiCall, "RAI: " + McpRaiSaid
      END IF
      IF TIMER >= McpRaiUntil THEN
        Mcp.Reply McpRaiCall, "RAI is still answering after 10 minutes: get_rai_steps says how it goes.", 1
        McpRaiCall = 0
      END IF
    ELSE
      Mcp.Reply McpRaiCall, McpRaiStepsText + CHR$(10) + CHR$(10) + "Answer:" + CHR$(10) + AI.Answer
      McpRaiCall = 0
    END IF
  END IF
  McpStatusShow
END SUB

' ---- Settings > AI Apps ---------------------------------------------------------------

SUB McpLayOutPage(Pad AS INTEGER, W AS INTEGER, Top AS INTEGER, H AS INTEGER, Row AS INTEGER, Fh AS INTEGER, Bh AS INTEGER)
  DIM y AS INTEGER, lx AS INTEGER, fx AS INTEGER, fw AS INTEGER, nh AS INTEGER, k AS INTEGER, pw AS INTEGER
  McpPage.Left = 0: McpPage.Top = Top: McpPage.Width = W + 2 * Pad: McpPage.Height = H
  lx = Pad: fx = Pad + 112: fw = W - 112
  y = Pad - 4
  McpOnBox.Left = lx: McpOnBox.Top = y: McpOnBox.Width = W: McpOnBox.Height = 22
  McpOnBox.Font.Bold = 1
  y = y + 26
  nh = McpOnNoteHigh(W)
  McpOnNote.Left = lx: McpOnNote.Top = y: McpOnNote.Width = W: McpOnNote.Height = nh
  McpOnNote.Font.Color = TC("page.dim")
  y = y + nh + 10
  McpPortLabel.Left = lx: McpPortLabel.Top = y + 5: McpPortLabel.Width = 108
  McpPortEdit.Left = fx: McpPortEdit.Top = y: McpPortEdit.Width = 80: McpPortEdit.Height = Fh
  y = y + Row
  McpAddrLabel.Left = lx: McpAddrLabel.Top = y + 5: McpAddrLabel.Width = 108
  McpAddrEdit.Left = fx: McpAddrEdit.Top = y: McpAddrEdit.Width = fw - 148: McpAddrEdit.Height = Fh
  McpCopyAddrBtn.Left = fx + fw - 140: McpCopyAddrBtn.Top = y - (Bh - Fh) \ 2: McpCopyAddrBtn.Width = 140: McpCopyAddrBtn.Height = Bh
  y = y + Row
  McpHeaderLabel.Left = lx: McpHeaderLabel.Top = y + 5: McpHeaderLabel.Width = 108
  McpHeaderEdit.Left = fx: McpHeaderEdit.Top = y: McpHeaderEdit.Width = fw - 148: McpHeaderEdit.Height = Fh
  McpCopyHeaderBtn.Left = fx + fw - 140: McpCopyHeaderBtn.Top = y - (Bh - Fh) \ 2: McpCopyHeaderBtn.Width = 140: McpCopyHeaderBtn.Height = Bh
  y = y + Row
  McpShowTokenBtn.Left = fx: McpShowTokenBtn.Top = y: McpShowTokenBtn.Width = 80: McpShowTokenBtn.Height = Bh
  McpResetBtn.Left = fx + 88: McpResetBtn.Top = y: McpResetBtn.Width = 132: McpResetBtn.Height = Bh
  y = y + Bh + 16
  ' (the permission mode: a label, its menu beside it, what it means under)
  ' (160 wide, wider when its words in Studio's language are: bold, a tenth more)
  pw = AISetMeasure.TextWidth(REPLACESUBSTR$(McpPermHead.Caption, "&", "")) * 11 \ 10 + 8
  IF pw < 160 THEN pw = 160
  IF pw > W - 148 THEN pw = W - 148
  McpPermHead.Left = lx: McpPermHead.Top = y + 5: McpPermHead.Width = pw
  McpPermHead.Font.Bold = 1
  McpPageModeBox.Left = lx + pw + 8: McpPageModeBox.Top = y: McpPageModeBox.Width = 220
  IF McpPageModeBox.Left + 220 > lx + W THEN McpPageModeBox.Width = lx + W - McpPageModeBox.Left
  y = y + Row
  ' (as tall as the longest mode's words, so nothing moves — or is cut —
  ' when another mode is picked)
  nh = McpModeNoteHigh(W)
  McpModeNoteLabel.Left = lx: McpModeNoteLabel.Top = y: McpModeNoteLabel.Width = W: McpModeNoteLabel.Height = nh
  McpModeNoteColour
  y = y + nh + 10
  ' (what AI apps can see and do: the rows — each says its own head —, the
  ' footer and Learn more)
  FOR k = 0 TO 3
    McpRowHeadL(k).Font.Bold = 1
    McpRowHeadL(k).Font.Color = TC("page.text")
    McpRowBodyL(k).Font.Color = TC("page.dim")
    McpRowBmp(k).LoadIcon McpRowIcon(k), 16
    McpRowImg(k).BMPHandle = McpRowBmp(k)
    y = y + InfoRowLayOut(McpRowImg(k), McpRowHeadL(k), McpRowBodyL(k), lx, y, McpHeadW, W) + 4
  NEXT
  nh = AISetLinesHigh(McpSeeFoot.Caption, W)
  McpSeeFoot.Left = lx: McpSeeFoot.Top = y: McpSeeFoot.Width = W: McpSeeFoot.Height = nh
  McpSeeFoot.Font.Color = TC("page.dim")
  y = y + nh + 8
  ' (Clear Activity...; Learn more at the right of its row — a link's
  ' text sits 4 inside its rectangle)
  McpClearBtn.Left = lx: McpClearBtn.Top = y: McpClearBtn.Width = 160: McpClearBtn.Height = Bh
  McpLearnLink.Font = LinkFont
  McpLearnLink.Font.Color = TC("page.link")
  McpLearnLink.Font.Underline = 1
  McpLearnLink.AutoSize = 0
  McpLearnLink.Width = AISetMeasure.TextWidth(McpLearnLink.Caption) + 10: McpLearnLink.Height = 24
  McpLearnLink.Left = lx + W - McpLearnLink.Width + 4: McpLearnLink.Top = y + (Bh - 24) \ 2 + 3
END SUB

' (the switch's note: as tall as its lines, and room for School Lock's
' strip, which takes its place while AI apps are locked — schoollock.inc)
FUNCTION McpOnNoteHigh(W AS INTEGER) AS INTEGER
  Result = AISetLinesHigh(McpOnNote.Caption, W)
  IF LockOffApps AND Result < LockStripH + 4 THEN Result = LockStripH + 4
END FUNCTION

' (how tall the page needs to be, for the dialog: settings.inc)
FUNCTION McpPageHigh(W AS INTEGER, Row AS INTEGER, Bh AS INTEGER) AS INTEGER
  Result = 26 + McpOnNoteHigh(W) + 10 + 3 * Row + Bh + 16 + Row + McpModeNoteHigh(W) + 10 + McpRowsHigh(W) + Bh + 20
END FUNCTION

' (the rows' column of heads: 104 wide, wider when a head in Studio's language
' needs it — bold, a tenth more than the plain font measures)
FUNCTION McpHeadW AS INTEGER
  DIM k AS INTEGER, w AS INTEGER
  Result = 104
  FOR k = 0 TO 3
    w = AISetMeasure.TextWidth(McpRowHead(k)) * 11 \ 10 + 16
    IF w > Result THEN Result = w
  NEXT
END FUNCTION

' (what AI apps can see and do, as the page lays it out: the rows, the
' footer)
FUNCTION McpRowsHigh(W AS INTEGER) AS INTEGER
  DIM k AS INTEGER, h AS INTEGER, bh AS INTEGER
  h = 0
  FOR k = 0 TO 3
    bh = AISetLinesHigh(McpRowBody(k), W - 24 - McpHeadW)
    IF bh < 18 THEN bh = 18
    h = h + bh + 4
  NEXT
  Result = h + AISetLinesHigh(McpFootText, W) + 8
END FUNCTION

' (the mode note's height: its longest mode's, wrapped at W)
FUNCTION McpModeNoteHigh(W AS INTEGER) AS INTEGER
  DIM k AS INTEGER, h AS INTEGER
  Result = 0
  FOR k = 0 TO 3
    h = AISetLinesHigh(McpModeNote(McpModeAt(k)), W)
    IF h > Result THEN Result = h
  NEXT
END FUNCTION

' (the mode note's colour: Allow everything's in the warning's text colour,
' the others secondary — docs/design-guidelines.md, "Colour")
SUB McpModeNoteColour
  IF McpModeAt(McpPageModeBox.ItemIndex) = "all" THEN
    McpModeNoteLabel.Font.Color = TC("status.warning.text")
  ELSE
    McpModeNoteLabel.Font.Color = TC("page.dim")
  END IF
END SUB


' (ShowSettingsPage: the page as the settings are)
SUB McpSettingsFill
  McpOnBox.Checked = McpOn
  McpPortEdit.Text = Num(McpPort)
  McpAddrEdit.Text = McpUrl
  McpHeaderEdit.PasswordChar = "*"
  McpShowTokenBtn.Caption = TR$("Sho&w")
  IF McpToken = "" AND (McpOn OR McpTestToken <> "") THEN McpToken = McpTokenNow
  IF McpToken <> "" THEN McpHeaderEdit.Text = "Authorization: Bearer " + McpToken ELSE McpHeaderEdit.Text = TR$("(turn AI apps on: a token is made)")
  ' (the open project's mode)
  McpModeFollow
  McpPageModeBox.ItemIndex = McpModeIndex(McpMode)
  IF ProjectOpen THEN McpPermHead.Caption = TR$("In {project}, AI apps may &do:", "project", ProjectName) ELSE McpPermHead.Caption = TR$("What AI apps may &do:")
  McpModeNoteLabel.Caption = McpModeNote(McpMode)
  McpModeNoteColour
END SUB

' (Settings' menu: what the mode chosen means, shown at once)
SUB McpPageModePicked
  McpModeNoteLabel.Caption = McpModeNote(McpModeAt(McpPageModeBox.ItemIndex))
  McpModeNoteColour
END SUB

' (settings.inc's SettingsTabChanged)
SUB McpSettingsTab(Index AS INTEGER)
  McpPage.Visible = Flag(Index = 2)
END SUB

' Turned on: the first time, what it means is said, Cancel the default.
SUB McpSetOnClick
  IF McpOnBox.Checked = 0 OR McpConsentOk THEN EXIT SUB
  IF McpConsent = 0 THEN
    McpOnBox.Checked = 0
    EXIT SUB
  END IF
  McpConfirmed = 1
  McpConsentVersion = MCP_CONSENT
END SUB

' The first-time question (consent.inc): what AI apps can see and do, in
' rows; Turn On or Cancel — Cancel the default, Escape too. 1: Turn On.
FUNCTION McpConsent AS INTEGER
  DIM k AS INTEGER
  ConsentBegin TR$("Let AI apps use RapidR Studio?"), TR$("With this on, an AI app on this computer that has your token can work with your open project through Studio, and you see everything it does in the AI Apps pane.")
  FOR k = 0 TO 3
    ConsentRow McpRowIcon(k), McpRowHead(k), McpRowBody(k)
  NEXT
  ConsentYes.AccessibleDescription = TR$("AI apps that have the token can connect to RapidR Studio")
  ConsentNo.AccessibleDescription = TR$("AI apps stay off")
  Result = ConsentAsk(McpFootText, McpManualUrl, TR$("&Turn On"), TR$("Cancel"))
END FUNCTION

' (OK in Settings: the choices taken; the server started or stopped)
SUB McpSettingsTake
  DIM port AS INTEGER, was AS INTEGER
  port = VAL(McpPortEdit.Text)
  IF port < 1024 OR port > 65535 THEN port = MCP_DEFAULT_PORT
  was = McpOn
  ' (the project's mode: Allow everything confirmed when newly chosen)
  IF McpModeAt(McpPageModeBox.ItemIndex) = "all" AND McpMode <> "all" THEN
    IF McpConfirmAll THEN McpSetMode "all"
  ELSE
    McpSetMode McpModeAt(McpPageModeBox.ItemIndex)
  END IF
  McpOn = Flag(McpOnBox.Checked)
  IF McpOn AND McpConsentOk = 0 THEN McpOn = 0
  IF port <> McpPort AND Mcp.Listening THEN McpStopServer
  McpPort = port
  IF McpOn THEN
    IF Mcp.Listening THEN McpSyncTools ELSE McpStart
  ELSEIF Mcp.Listening THEN
    McpStopServer
    OutputLine TR$("AI apps: off. Every AI app was disconnected.")
  END IF
  McpSaveSettings
  McpShowState
END SUB

SUB McpCopyAddressClick
  Clipboard.Text = McpAddrEdit.Text
  Application.Announce TR$("Address copied")
END SUB

SUB McpCopyHeaderClick
  IF McpToken = "" THEN EXIT SUB
  Clipboard.Text = "Authorization: Bearer " + McpToken
  Application.Announce TR$("Header copied")
END SUB

SUB McpShowTokenClick
  IF McpHeaderEdit.PasswordChar = "" THEN
    McpHeaderEdit.PasswordChar = "*"
    McpShowTokenBtn.Caption = TR$("Sho&w")
  ELSE
    McpHeaderEdit.PasswordChar = ""
    McpShowTokenBtn.Caption = TR$("&Hide")
  END IF
END SUB

' Reset Token: a new token at once — every AI app with the old one is out.
SUB McpResetTokenClick
  DIM t AS STRING
  IF McpTestToken <> "" THEN EXIT SUB
  t = Mcp.NewToken
  IF t = "" THEN EXIT SUB
  IF McpSecrets.Save("AI apps", "token", t) = 0 THEN OutputLine TR$("AI apps: the new token couldn't be kept in the keychain ({error}).", "error", McpSecrets.Error)
  McpToken = t
  McpHeaderEdit.Text = "Authorization: Bearer " + t
  IF Mcp.Listening THEN
    McpStopServer
    McpStart
  END IF
  McpTools.AuditCall "Studio", "reset token", "", "every AI app disconnected"
  Application.Announce TR$("A new token: AI apps need the new header")
END SUB

' (the activity list's menu: Copy when a line is picked, Copy All when
' there is one)
SUB McpLogMenuOpening
  McpLogItem(0).Enabled = Flag(McpLog.ItemIndex >= 0)
  McpLogItem(1).Enabled = Flag(McpLog.ItemCount > 0)
END SUB

SUB McpLogMenuPicked(Sender AS RMenuItem)
  SELECT CASE Sender.Tag
    CASE 0: McpLogCopy 0
    CASE 1: McpLogCopy 1
    CASE 2: McpClearLogClick
  END SELECT
END SUB

' The picked line of the activity (or every line, All) on the clipboard as
' plain text: "time  client  action — result", one a line.
SUB McpLogCopy(All AS INTEGER)
  DIM t AS STRING, k AS INTEGER
  IF All THEN
    FOR k = 0 TO McpLog.ItemCount - 1
      IF k > 0 THEN t = t + CHR$(10)
      t = t + McpLog.Item(k)
    NEXT
  ELSEIF McpLog.ItemIndex >= 0 THEN
    t = McpLog.Item(McpLog.ItemIndex)
  END IF
  IF t <> "" THEN Clipboard.Text = t
END SUB

' (Edit > Copy — Ctrl+C, ⌘C — while the AI Apps pane has the focus: the
' picked line; shell.inc's RunCommand asks first)
FUNCTION McpPaneCommand(Id AS STRING) AS INTEGER
  IF LCASE$(Dock.ActivePane) <> "mcppanel" THEN EXIT FUNCTION
  IF Id = "edit.copy" THEN
    McpLogCopy 0
    Result = 1
  END IF
END FUNCTION

SUB McpClearLogClick
  ' (a small confirmation: the question, one sentence, Clear or Cancel —
  ' consent.inc, no rows)
  ConsentBegin TR$("Clear the AI apps' activity?"), TR$("The list of what AI apps did is removed from this computer ({location}).", "location", McpTools.AuditLocation)
  ConsentNo.AccessibleDescription = TR$("The list stays")
  ConsentYes.AccessibleDescription = TR$("Removes the list of what AI apps did")
  IF ConsentAsk("", "", TR$("C&lear"), TR$("Cancel")) = 0 THEN EXIT SUB
  IF McpTools.ClearAudit THEN McpLog.Clear
END SUB

' (Studio's language changed) What was made once, in the language before: the
' pane's title, Settings' tab, the mode menus, the page's rows (their words and
' the names read aloud), the activity list's menu, and the lines that say a
' state. The captions and names written as TR$ texts are translated again by
' Studio itself. Settings is laid out again after this.
SUB McpLanguageChanged
  DIM k AS INTEGER, was AS INTEGER
  ' (the desktop's only: the web has no AI Apps pane)
  IF Proj.FileManager = "" THEN EXIT SUB
  Dock.PaneTitle("mcppanel", TR$("AI Apps"))
  SettingsTabs.Tab(2) = TR$("AI Apps")
  ' (the pane's menu: McpModeFilling keeps the refill from being a choice)
  McpModeFilling = 1
  McpFillModes McpModeBox
  McpModeBox.ItemIndex = McpModeIndex(McpMode)
  McpModeFilling = 0
  ' (Settings' menu: what is picked in it kept)
  was = McpPageModeBox.ItemIndex
  McpFillModes McpPageModeBox
  IF was >= 0 THEN McpPageModeBox.ItemIndex = was
  FOR k = 0 TO 3
    McpRowHeadL(k).Caption = McpRowHead(k)
    McpRowBodyL(k).Caption = McpRowBody(k)
    McpRowBodyL(k).AccessibleName = McpRowHead(k) + ": " + McpRowBody(k)
  NEXT
  McpSeeFoot.Caption = McpFootText
  McpLogItem(0).Caption = TR$("&Copy") + CHR$(9) + "Ctrl+C"
  McpLogItem(1).Caption = TR$("Copy &All")
  McpLogItem(2).Caption = TR$("C&lear Activity...")
  IF ProjectOpen THEN McpPermHead.Caption = TR$("In {project}, AI apps may &do:", "project", ProjectName) ELSE McpPermHead.Caption = TR$("What AI apps may &do:")
  IF McpPageModeBox.ItemIndex >= 0 THEN McpModeNoteLabel.Caption = McpModeNote(McpModeAt(McpPageModeBox.ItemIndex))
  IF McpToken = "" THEN McpHeaderEdit.Text = TR$("(turn AI apps on: a token is made)")
  IF McpWaitCount > 0 THEN McpShowAsk
  McpShowState
  McpLayOutPane
END SUB

' ---- commands and test steps ------------------------------------------------------------

FUNCTION McpCommand(Id AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Id
    CASE "mcp.settings": ShowSettingsPage 2
    CASE "view.aiApps"
      McpPaneShow
      Dock.FocusPane("mcppanel")
      McpShowState
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (a test step mcp:<what>[=<value>]: only where the tests' token stands in
' for the keychain's)
SUB McpStep(Stp AS STRING)
  DIM what AS STRING, arg AS STRING, k AS INTEGER
  IF McpTestToken = "" THEN EXIT SUB
  k = INSTR(Stp, "=")
  IF k > 0 THEN
    what = LEFT$(Stp, k - 1): arg = MID$(Stp, k + 1)
  ELSE
    what = Stp
  END IF
  SELECT CASE what
    CASE "on"
      IF arg <> "" THEN McpPort = VAL(arg)
      McpOn = 1: McpConfirmed = 1: McpConsentVersion = MCP_CONSENT
      McpStart
    CASE "off"
      McpOn = 0
      McpStopServer
    CASE "consent"
      ' (the first-time question, as turning the switch on asks it; the
      ' answer in the switch: mcponbox.checked)
      k = McpConsent
      McpConfirmed = k
      IF k THEN McpConsentVersion = MCP_CONSENT
      McpOnBox.Checked = k
    CASE "allow"
      FOR k = 1 TO TALLY(arg, "|") + 1
        IF INSTR(McpGrants, "|" + FIELD$(arg, "|", k) + "|") = 0 THEN McpGrants = McpGrants + "|" + FIELD$(arg, "|", k) + "|"
      NEXT
    CASE "mode"
      McpModeFollow
      McpSetMode McpModeAt(McpModeIndex(arg))
    CASE "decide"
      IF arg = "deny" THEN McpDecide 0
      IF arg = "once" THEN McpDecide 1
      IF arg = "session" THEN McpDecide 2
    CASE "settings": ShowSettingsPage 2
    CASE "pane": k = McpCommand("view.aiApps")
  END SELECT
END SUB
