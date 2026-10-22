' ai.inc: the AI assistant (docs/ide-ai.md §5, docs/manual/ai-assistant.md).
'
' The assistant is an RAI — the component users' programs have, the same
' code — with RapidR Studio's project attached as tools (an
' RAIProjectTools: the files, the forms, the registry, the compiler, the
' changes it proposes). Both are DIM'd in decl.inc. Here: its panel, docked
' with Properties (View > AI Assistant, Ctrl+I); the review of what it
' proposes (RDiffView: each file applied as one undo step in its editor); a
' new program created and opened at once with its banner — Run It, Undo,
' What Did It Do? — (docs/ai-ux-plan.md), or previewed first when Review
' first is on; "What was sent"; the token meter. Its settings: Tools > Settings > AI (settings.inc). And an RAI on a
' form, live in the designer: its Model list fetched once it has a key, the
' key typed in the inspector kept in the keychain, never in the code.
'
' Hidden until it is turned on AND a provider, a key and a model are
' chosen: no AI pane, no AI button, no AI command, nothing sent. Then
' nothing is sent until Send; the panel says what Send shares. Send while
' it works queues the message ("Queued: …", × takes it back): it goes when
' the answer ends; typing never stops an answer — Stop does, and gives the
' queued message back to the box (docs/preview4-plan.md R3).
'
' Preview 4's R1–R6 (docs/preview4-plan.md): the chats are kept per project
' in Studio's data folder and listed (Chats: switch, rename, delete with
' Undo, search; aichats.inc); a sent question can be edited, and the last
' answer retried — what followed moves to a saved copy; the status line
' says what RAI does now (Reading main.rr, Searching the manual…) with
' its seconds, and when the model has been silent for 30; a context meter
' with Compact, / commands, the reasoning effort and "Thought for N s"; one
' model list with ↻ (the provider is chosen in Settings, or by Set up RAI:
' aisetup.inc).

DECLARE SUB AIUpdate
DECLARE SUB AIDone(Status AS STRING)
DECLARE SUB AIModelsIn(Count AS INTEGER)
DECLARE SUB AIErrorSaid(Message AS STRING)
DECLARE SUB AISend
DECLARE SUB AISendNow(T AS STRING, FromBox AS INTEGER)
DECLARE SUB AIShowQueue
DECLARE SUB AIQueueCancelClick
DECLARE SUB AISendQueued(Status AS STRING)
DECLARE SUB AIStopClick
DECLARE SUB AINewChat
DECLARE SUB AIInputKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIModelPicked
DECLARE SUB AIReviewClick
DECLARE SUB AISentLinkClick
DECLARE SUB AIRevPicked
DECLARE SUB AIRevHunk(Index AS INTEGER, Accepted AS INTEGER)
DECLARE SUB AIRevAcceptAll
DECLARE SUB AIRevRejectAll
DECLARE SUB AIRevApplyClick
DECLARE SUB AIRevCancel
DECLARE SUB AINewPicked
DECLARE SUB AINewBrowse
DECLARE SUB AINewCreate
DECLARE SUB AINewCancel
DECLARE SUB AISentClose
DECLARE SUB AIKeyNeeded
DECLARE SUB AIChatLink(Url AS STRING)
DECLARE SUB AIBannerRunClick
DECLARE SUB AIBannerRunIn
DECLARE SUB AIBannerRunOut
DECLARE SUB AIBannerFixClick
DECLARE SUB AIBannerUndoClick
DECLARE SUB AIBannerWhatClick
DECLARE SUB AIBannerCloseClick
DECLARE SUB LayOutAIBanner
DECLARE SUB AIProgressShow
DECLARE SUB AIHideBanner
DECLARE SUB AICreateNow
DECLARE SUB AIShowReviewBar
DECLARE FUNCTION AIUnsavedAsked AS INTEGER
DECLARE SUB AIOpenMade(Made AS STRING, Errs AS INTEGER)
DECLARE SUB AIShowBanner(Errs AS INTEGER)
DECLARE SUB AISetBanner(Errs AS INTEGER)
DECLARE SUB AIRecheck
DECLARE SUB AIUndoProject
DECLARE SUB AIForgetRecent(Path AS STRING)
DECLARE FUNCTION AskUndoAnyway(Question AS STRING, Detail AS STRING) AS INTEGER
DECLARE SUB InspectorEditorRequest(Prop AS STRING)
DECLARE SUB DesignAIModelsIn(Count AS INTEGER)
DECLARE SUB DesignAIError(Message AS STRING)
DECLARE FUNCTION DesignAIProviderName AS STRING
DECLARE FUNCTION DesignAIKey AS STRING
DECLARE SUB DesignAIKeyNote
DECLARE SUB AIRefreshClick
DECLARE SUB AIEffortPicked
DECLARE SUB AICompactClick
DECLARE SUB AIClockTick(Sender AS RTimer)
DECLARE SUB AIInputChanged
DECLARE SUB AISlashPicked
DECLARE SUB AIShowSlash
DECLARE FUNCTION AIRunSlash(T AS STRING) AS INTEGER
DECLARE SUB AIShowContext
DECLARE SUB AIUndoLast
DECLARE SUB AIEditCancelClick
DECLARE SUB AIFillEffort
DECLARE SUB AISetEffort(E AS STRING)
DECLARE FUNCTION AIThousands(N AS INTEGER) AS STRING
DECLARE SUB AISlashComplete
DECLARE FUNCTION AIMatches(A AS STRING, B AS STRING) AS INTEGER
' (aichats.inc: the chats kept, edit and retry)
DECLARE SUB SetUpAIChats
DECLARE SUB LayOutAIChats
DECLARE SUB AIShowChatTitle
DECLARE SUB AIChatsFonts
DECLARE SUB AISaveChat
DECLARE SUB AIChatsClick
DECLARE SUB AIShowChats(Show AS INTEGER)
DECLARE SUB AIStartNewChat
DECLARE SUB AILoadProjectChat
DECLARE SUB AIEditQuestion(Q AS INTEGER)
DECLARE SUB AIRetry
DECLARE FUNCTION AISendEdited(T AS STRING) AS INTEGER
DECLARE FUNCTION AIChatStep(Verb AS STRING, Arg AS STRING) AS INTEGER
' (aisetup.inc: Set up RAI)
DECLARE SUB SetUpAISetup
DECLARE FUNCTION AISetupStep(Verb AS STRING, Arg AS STRING) AS INTEGER

CONST AIMAXPROPS = 32

' (the review: each proposal's text as the user's decisions left it, and
' whether they looked at it)
DIM AIRevResult(AIMAXPROPS) AS STRING
DIM AIRevSeen(AIMAXPROPS) AS INTEGER
DIM AIRevAt AS INTEGER
' (the panel's lists being filled: their OnChange isn't the user's)
DIM AIFilling AS INTEGER

' ---- the panel ---------------------------------------------------------------------
DIM AIPanel AS RPanel
DIM AITop AS RPanel
DIM AIModelBox AS RComboBox
DIM AINewBtn AS RButton
DIM AIChat AS RMarkdownView
DIM AIReviewBar AS RPanel
DIM AIReviewBtn AS RButton
DIM AIBottom AS RPanel
DIM AIInput AS RMemo
DIM AISendBtn AS RButton
DIM AIStopBtn AS RButton
DIM AINote AS RLabel
DIM AIMeter AS RLabel
' (measures the meter's text: how many lines it wraps to, AIMeterHigh)
DIM AIMeterMeasure AS RBitmap
DIM AISentLink AS RLabel
DIM AIStatus AS RStatusBar
' (while an answer runs, the status is its progress checklist: what it
' said last, not said again)
DIM AIProgressSaid AS STRING
' (a message sent while it works: it waits here, shown above the box, and
' goes when the answer ends)
DIM AIQueueBar AS RPanel
DIM AIQueueText AS RLabel
DIM AIQueueCancel AS RButton
DIM AIQueued AS STRING
' (R6: one model list, ↻ fetches it again; the effort beside it)
DIM AIRefreshBtn AS RButton
DIM AIEffortBox AS RComboBox
' (Low/Medium/High, kept per user: "" — the model's own way)
DIM AIEffort AS STRING
' (R1: the chats — the Chats button, the chat's title beside it)
DIM AIChatsBtn AS RButton
DIM AIChatTitle AS RLabel
' (R4: the seconds the answer has taken, beside Stop — a label a screen
' reader reads when asked, never announced each second; the status says
' once when the model has been silent for 30)
DIM AIElapsed AS RLabel
DIM AIClock AS RTimer
DIM AIQuietSaid AS INTEGER
' (seconds of silence before it is said: 30; the tests', less)
DIM AIQuietAfter AS INTEGER
' (R5: the context meter and Compact)
DIM AIContextBar AS RProgressBar
DIM AIContextText AS RLabel
DIM AICompactBtn AS RButton
' (R5: the / commands, listed while one is typed)
DIM AISlashBar AS RPanel
DIM AISlashList AS RListBox
DIM AISlashShown AS STRING
' (R2: editing a sent question — the bar that says what Send will do)
DIM AIEditBar AS RPanel
DIM AIEditText AS RLabel
DIM AIEditCancel AS RButton
DIM AIEditing AS INTEGER
' (/undo: the files the last review changed, as they were and became)
DIM AIUndoDoc(AIMAXPROPS) AS INTEGER
DIM AIUndoBefore(AIMAXPROPS) AS STRING
DIM AIUndoAfter(AIMAXPROPS) AS STRING
DIM AIUndoCount AS INTEGER
' ---- R1: the chats kept (aichats.inc) ----
' (the chat shown: its id — "" until it is first kept —, the project it
' belongs to — its project file, "" none, CHR$(1) before the first —, its
' title)
DIM AIChatId AS STRING
DIM AIChatProject AS STRING
DIM AIChatName AS STRING
' (the assistant made a new program, or undid one: the chat moves with
' the project, it isn't replaced)
DIM AIRehoming AS INTEGER
' (the list: the ids and titles it shows, a line each)
DIM AIChatIds AS STRING
DIM AIChatTitles AS STRING
' (a chat deleted, kept for Undo: what it was, and its project)
DIM AIChatDeleted AS STRING
DIM AIChatDeletedProject AS STRING
DIM AIChatDeletedTitle AS STRING

DIM AIChatsPanel AS RPanel
DIM AIChatsHead AS RPanel
DIM AIChatSearch AS REdit
DIM AIChatsFoot AS RPanel
DIM AIChatsOpenBtn AS RButton
DIM AIChatsRenameBtn AS RButton
DIM AIChatsDeleteBtn AS RButton
DIM AIChatSearchLabel AS RLabel
DIM AIChatList AS RListBox
DIM AIChatsNote AS RLabel
DIM AIRenameBar AS RPanel
DIM AIRenameEdit AS REdit
DIM AIRenameOk AS RButton
DIM AIRenameCancel AS RButton
DIM AIChatUndoBar AS RPanel
DIM AIChatUndoText AS RLabel
DIM AIChatUndoBtn AS RButton


' ---- a new program, straight into the IDE (docs/ai-ux-plan.md §1) ---------------------
' The banner across the top, under the tool bar: what the assistant made,
' Run it (the keys go there), Undo, What did it do?; with errors left, Ask
' it to fix too. Studio's own panel, in the look's colours.
DIM AIBanner AS RPanel
DIM AIBannerMark AS RPanel
DIM AIBannerText AS RLabel
DIM AIBannerRun AS RButton
DIM AIBannerFix AS RButton
DIM AIBannerUndo AS RButton
DIM AIBannerWhat AS RButton
DIM AIBannerClose AS RButton
DIM AIBannerFont AS RFont
' (the program the banner is about: its folder, its project file, its
' errors; the project open before it, opened again by Undo)
DIM AIBannerFolder AS STRING
DIM AICreatedFile AS STRING
DIM AIBannerErrors AS INTEGER
DIM AIPrevProject AS STRING
' (the banner's Run it takes the keys once the editor's opening is over)
DIM AIBannerFocusTurns AS INTEGER
' (tests: a file of the user's added to the new program's folder)
DIM AITouch AS RFileStream

' ---- the review of the changes it proposes -----------------------------------------
CREATE AIReview AS RForm
  Caption = "Review the Assistant's Changes"
  Width = 1000: Height = 660
  CREATE AIRevFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE AIRevInfo AS RLabel
      Left = 16: Top = 18: Width = 380
      AccessibleName = "What will be applied"
    END CREATE
    CREATE AIRevAll AS RButton
      Caption = "Accept &All"
      AccessibleName = "Accept every change of every file"
      OnClick = AIRevAcceptAll
    END CREATE
    CREATE AIRevNone AS RButton
      Caption = "&Reject All"
      AccessibleName = "Reject every change of every file"
      OnClick = AIRevRejectAll
    END CREATE
    CREATE AIRevApplyBtn AS RButton
      Caption = "A&pply"
      AccessibleName = "Apply the accepted changes"
      Default = 1
      OnClick = AIRevApplyClick
    END CREATE
    CREATE AIRevCancelBtn AS RButton
      Caption = "Cancel"
      Cancel = 1
      OnClick = AIRevCancel
    END CREATE
  END CREATE
  ' (what the change does, above the files and the diff)
  CREATE AIRevTop AS RPanel
    Align = 1
    Height = 44
    BevelOuter = 0
    CREATE AIRevHead AS RLabel
      Left = 16: Top = 13: Width = 960
      AccessibleName = "What the change does"
    END CREATE
  END CREATE
  CREATE AIRevList AS RListBox
    Align = 3
    Width = 220
    AccessibleName = "Files the assistant changes"
    OnClick = AIRevPicked
  END CREATE
  CREATE AIRevDiff AS RDiffView
    Align = 5
    Mode = "split"
    AccessibleName = "The change: the file as it is on the left, as proposed on the right"
    OnHunkChange = AIRevHunk
  END CREATE
END CREATE

' ---- a new program's preview --------------------------------------------------------
CREATE AINew AS RForm
  Caption = "A New Program from the Assistant"
  Width = 1040: Height = 700
  CREATE AINewHead AS RPanel
    Align = 1
    Height = 64
    BevelOuter = 0
    CREATE AINewTitle AS RLabel
      Left = 16: Top = 10: Width = 900
    END CREATE
    CREATE AINewWhat AS RLabel
      Left = 16: Top = 34: Width = 900
    END CREATE
  END CREATE
  CREATE AINewFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE AINewFolderLabel AS RLabel
      Caption = "Create &in:"
      Left = 16: Top = 19: Width = 70
    END CREATE
    CREATE AINewFolder AS REdit
      Left = 88: Top = 14: Width = 460
      AccessibleName = "The folder the new project goes in"
    END CREATE
    CREATE AINewBrowseBtn AS RButton
      Caption = "&Browse..."
      AccessibleName = "Choose the folder the new project goes in"
      OnClick = AINewBrowse
    END CREATE
    CREATE AINewCreateBtn AS RButton
      Caption = "&Create and Open"
      Default = 1
      OnClick = AINewCreate
    END CREATE
    CREATE AINewCancelBtn AS RButton
      Caption = "Cancel"
      Cancel = 1
      OnClick = AINewCancel
    END CREATE
  END CREATE
  CREATE AINewFiles AS RListBox
    Align = 3
    Width = 150
    AccessibleName = "The new program's files"
    OnClick = AINewPicked
  END CREATE
  CREATE AINewForm AS RDesignSurface
    Align = 4
    Width = 400
    ShowGrid = 0
    AccessibleName = "The form, as it will look"
  END CREATE
  CREATE AINewCode AS RCodeEditor
    Align = 5
    ReadOnly = 1
    AccessibleName = "The file's code"
  END CREATE
END CREATE

' ---- what was sent ---------------------------------------------------------------------
CREATE AISent AS RForm
  Caption = "What Was Sent"
  Width = 860: Height = 620
  CREATE AISentFoot AS RPanel
    Align = 2
    Height = 58
    BevelOuter = 0
    CREATE AISentWhere AS RLabel
      Left = 16: Top = 10: Width = 680: Height = 40
      WordWrap = 1: AutoSize = 0
    END CREATE
    CREATE AISentOk AS RButton
      Caption = "Close"
      Default = 1: Cancel = 1
      OnClick = AISentClose
    END CREATE
  END CREATE
  CREATE AISentText AS RCodeEditor
    Align = 5
    ReadOnly = 1
    Language = "plaintext"
    AccessibleName = "The last request, as sent (your key isn't in it)"
  END CREATE
END CREATE

' ---- set-up ------------------------------------------------------------------------

SUB SetUpAI
  DIM i AS INTEGER, all AS STRING, line AS STRING
  AI.OnUpdate = AIUpdate
  AI.OnDone = AIDone
  AI.OnModelsLoaded = AIModelsIn
  AI.OnError = AIErrorSaid
  AI.OnKeyNeeded = AIKeyNeeded
  ' (every request, tool call and decision in the local audit log)
  AI.Audit = 1
  ' (the answers in the language of the user's latest message, whatever
  ' the files, the tools or a long chat's summary are written in)
  AI.ReplyLanguage = "auto"
  ' (the project as the assistant's tools: on demand, never all at once)
  AI.AttachComponent AITools, "project"
  ' (Tools > Settings > AI's own: its lists and errors on its tab)
  SetAI.OnModelsLoaded = AISetModelsIn
  SetAI.OnError = AISetError
  SetAI.OnDone = AISetDone
  SetAI.Audit = 1
  ' (an RAI on a form: its models listed in the designer)
  DesignAI.OnModelsLoaded = DesignAIModelsIn
  DesignAI.OnError = DesignAIError
  Inspector.OnEditorRequest = InspectorEditorRequest
  Inspector.OnSelect = InspectorRowSelected
  ' (the providers, as the component lists them: the settings' list and
  ' Set up RAI's)
  all = AI.Providers
  AIProvCount = 0
  FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    line = FIELD$(all, CHR$(10), i)
    IF line <> "" AND AIProvCount < 8 THEN
      AIProvIds(AIProvCount) = FIELD$(line, CHR$(9), 1)
      AIProvNames(AIProvCount) = FIELD$(line, CHR$(9), 2)
      AIProvCount = AIProvCount + 1
    END IF
  NEXT
  ' (R2: Edit under each question, Retry under the last answer)
  AI.Editable = 1

  AIPanel.Parent = Studio
  AIPanel.Width = 360: AIPanel.Height = 520
  AIPanel.BevelOuter = 0
  AIPanel.AccessibleName = "AI Assistant"

  ' (the top strip: the chats and this one's title, a new chat; the model
  ' — the provider is Settings' — with ↻, and the effort)
  AITop.Parent = AIPanel
  AITop.Align = 1
  AITop.Height = 70
  AITop.BevelOuter = 0
  AIChatsBtn.Parent = AITop
  AIChatsBtn.Caption = "Chats"
  AIChatsBtn.AccessibleName = "Chats"
  AIChatsBtn.AccessibleDescription = "This project's chats: switch, rename, delete, search"
  AIChatsBtn.OnClick = AIChatsClick
  AIChatTitle.Parent = AITop
  AIChatTitle.AutoSize = 0: AIChatTitle.WordWrap = 0
  AIChatTitle.AccessibleName = "This chat"
  AIModelBox.Parent = AITop
  AIModelBox.AccessibleName = "Model"
  AIModelBox.OnChange = AIModelPicked
  AIRefreshBtn.Parent = AITop
  AIRefreshBtn.Caption = "↻"
  AIRefreshBtn.AccessibleName = "Refresh the model list"
  AIRefreshBtn.Hint = "Ask the provider for its models again"
  AIRefreshBtn.ShowHint = 1
  AIRefreshBtn.OnClick = AIRefreshClick
  AIEffortBox.Parent = AITop
  AIEffortBox.Style = 2
  AIEffortBox.AddItems "Effort: Auto", "Effort: Low", "Effort: Medium", "Effort: High"
  AIEffortBox.AccessibleName = "Reasoning effort"
  AIEffortBox.AccessibleDescription = "How hard the model thinks before it answers: more takes longer and costs more"
  AIEffortBox.OnChange = AIEffortPicked
  AINewBtn.Parent = AITop
  AINewBtn.Caption = "New Chat"
  AINewBtn.AccessibleName = "Start a new chat"
  AINewBtn.AccessibleDescription = "This chat is kept: Chats lists it"
  AINewBtn.OnClick = AINewChat

  ' (the bottom: the status sentence a screen reader announces, the box
  ' to ask in, Send, Stop, and what Send shares)
  AIStatus.Parent = AIPanel
  AIStatus.SimplePanel = 1
  AIStatus.AccessibleName = "Assistant status"
  AIBottom.Parent = AIPanel
  AIBottom.Align = 2
  AIBottom.Height = 204
  AIBottom.BevelOuter = 0
  AIInput.Parent = AIBottom
  AIInput.WordWrap = 1
  AIInput.ScrollBars = 2
  AIInput.AccessibleName = "Ask the assistant"
  AIInput.AccessibleDescription = "Enter sends; Shift+Enter starts a new line"
  AIInput.OnKeyDown = AIInputKey
  AISendBtn.Parent = AIBottom
  AISendBtn.Caption = "Send"
  AISendBtn.AccessibleName = "Send to the assistant"
  AISendBtn.OnClick = AISend
  AIStopBtn.Parent = AIBottom
  AIStopBtn.Caption = "Stop"
  AIStopBtn.AccessibleName = "Stop the answer"
  AIStopBtn.OnClick = AIStopClick
  AIStopBtn.Enabled = 0
  AINote.Parent = AIBottom
  AINote.WordWrap = 1: AINote.AutoSize = 0
  AINote.AccessibleName = "What Send shares"
  ' (the session's tokens, and their cost when the prices are known)
  AIMeter.Parent = AIBottom
  AIMeter.WordWrap = 1: AIMeter.AutoSize = 0
  AIMeter.AccessibleName = "Tokens used"
  AISentLink.Parent = AIBottom
  AISentLink.Caption = "What was sent"
  AISentLink.Cursor = -21
  AISentLink.AccessibleName = "What was sent to the assistant"
  AISentLink.OnClick = AISentLinkClick
  ' (R4: the answer's seconds, right of Stop while it works)
  AIElapsed.Parent = AIBottom
  AIElapsed.AutoSize = 0: AIElapsed.WordWrap = 0
  AIElapsed.Alignment = 1
  AIElapsed.AccessibleName = "Time the answer has taken"
  AIClock.Interval = 1000
  AIQuietAfter = 30
  AIClock.Enabled = 0
  AIClock.OnTimer = AIClockTick
  ' (R5: how full the conversation is, against where it is compacted by
  ' itself; Compact does it now)
  AIContextBar.Parent = AIBottom
  AIContextBar.Min = 0: AIContextBar.Max = 100
  AIContextBar.ShowText = 0
  AIContextBar.AccessibleName = "Context used"
  AIContextText.Parent = AIBottom
  AIContextText.AutoSize = 0: AIContextText.WordWrap = 0
  AIContextText.AccessibleName = "Context"
  AICompactBtn.Parent = AIBottom
  AICompactBtn.Caption = "Compact"
  AICompactBtn.AccessibleName = "Compact the chat"
  AICompactBtn.AccessibleDescription = "Summarises this chat now, so each question sends less (/compact)"
  AICompactBtn.OnClick = AICompactClick
  AIInput.OnChange = AIInputChanged

  ' (R5: the / commands while one is typed, just above the box)
  AISlashBar.Parent = AIPanel
  AISlashBar.Align = 2
  AISlashBar.Height = 132
  AISlashBar.BevelOuter = 0
  AISlashBar.Visible = 0
  AISlashBar.AccessibleName = "Commands"
  AISlashList.Parent = AISlashBar
  AISlashList.Align = 5
  AISlashList.AccessibleName = "Commands: Up and Down choose, Tab completes, Enter runs"
  AISlashList.OnClick = AISlashPicked
  AISlashList.OnDblClick = AISlashPicked

  ' (R2: a sent question being edited — what Send will do, and Cancel)
  AIEditBar.Parent = AIPanel
  AIEditBar.Align = 2
  AIEditBar.Height = 72
  AIEditBar.BevelOuter = 0
  AIEditBar.Visible = 0
  AIEditBar.AccessibleName = "Editing a sent question"
  AIEditText.Parent = AIEditBar
  AIEditText.AutoSize = 0: AIEditText.WordWrap = 1
  AIEditCancel.Parent = AIEditBar
  AIEditCancel.Caption = "Cancel"
  AIEditCancel.AccessibleName = "Stop editing the question"
  AIEditCancel.OnClick = AIEditCancelClick

  ' (a message sent while the assistant works: "Queued: …" just above the
  ' box, × takes it back; hidden while there is none)
  AIQueueBar.Parent = AIPanel
  AIQueueBar.Align = 2
  AIQueueBar.Height = 32
  AIQueueBar.BevelOuter = 0
  AIQueueBar.Visible = 0
  AIQueueBar.AccessibleName = "Queued message"
  AIQueueText.Parent = AIQueueBar
  AIQueueText.AutoSize = 0: AIQueueText.WordWrap = 0
  AIQueueCancel.Parent = AIQueueBar
  AIQueueCancel.Caption = "×"
  AIQueueCancel.AccessibleName = "Cancel the queued message"
  AIQueueCancel.AccessibleDescription = "It won't be sent"
  AIQueueCancel.OnClick = AIQueueCancelClick

  ' (the proposals' bar: shown when the answer proposes changes — one
  ' button that says what it opens; the status says the rest, once)
  AIReviewBar.Parent = AIPanel
  AIReviewBar.Align = 2
  AIReviewBar.Height = 46
  AIReviewBar.BevelOuter = 0
  AIReviewBar.Visible = 0
  AIReviewBtn.Parent = AIReviewBar
  AIReviewBtn.Caption = "Review the Changes..."
  AIReviewBtn.AccessibleName = "Review the proposed changes"
  AIReviewBtn.OnClick = AIReviewClick

  ' (the conversation; a card's links — Open, Undo, Review — come here)
  AIChat.Parent = AIPanel
  AIChat.Align = 5
  AIChat.FollowEnd = 1
  AIChat.AccessibleName = "Conversation with the assistant"
  AIChat.OnLinkClick = AIChatLink

  ' (the new program's banner, across Studio under the tool bar: hidden
  ' until the assistant makes one)
  AIBanner.Parent = Studio
  AIBanner.Top = 60
  AIBanner.Align = 1
  AIBanner.Height = 56
  AIBanner.BevelOuter = 0
  AIBanner.Visible = 0
  AIBanner.AccessibleName = "The assistant's new program"
  AIBannerMark.Parent = AIBanner
  AIBannerMark.BevelOuter = 0
  AIBannerMark.Left = 0: AIBannerMark.Top = 0: AIBannerMark.Width = 6
  AIBannerText.Parent = AIBanner
  AIBannerText.WordWrap = 1: AIBannerText.AutoSize = 0
  AIBannerRun.Parent = AIBanner
  AIBannerRun.AccessibleName = "Run it"
  AIBannerRun.AccessibleDescription = "Runs the new program (F5). Nothing runs until you press it."
  AIBannerRun.OnClick = AIBannerRunClick
  ' (the accent, and Enter, only while it has the keys: Enter in the
  ' designer or a list never runs the program)
  AIBannerRun.OnEnter = AIBannerRunIn
  AIBannerRun.OnExit = AIBannerRunOut
  AIBannerFix.Parent = AIBanner
  AIBannerFix.Caption = "Ask It to Fix"
  AIBannerFix.AccessibleName = "Ask the assistant to fix the errors"
  AIBannerFix.OnClick = AIBannerFixClick
  AIBannerUndo.Parent = AIBanner
  AIBannerUndo.AccessibleName = "Undo"
  AIBannerUndo.AccessibleDescription = "Closes the new program and removes the files the assistant wrote"
  AIBannerUndo.OnClick = AIBannerUndoClick
  AIBannerWhat.Parent = AIBanner
  AIBannerWhat.Caption = "What Did It Do?"
  AIBannerWhat.AccessibleName = "What did it do?"
  AIBannerWhat.AccessibleDescription = "Shows what the assistant did, in the Assistant pane"
  AIBannerWhat.OnClick = AIBannerWhatClick
  AIBannerClose.Parent = AIBanner
  AIBannerClose.Caption = "×"
  AIBannerClose.AccessibleName = "Close the banner"
  AIBannerClose.OnClick = AIBannerCloseClick
  AIBanner.OnResize = LayOutAIBanner

  ' (R1: the chats list, in the conversation's place while shown;
  ' Set up RAI's window)
  SetUpAIChats
  SetUpAISetup
  ' (the chat kept last when no project is open: shown again)
  AILoadProjectChat

  AIPanel.OnResize = LayOutAIPanel
  ' (Tools > Settings: its General and AI pages)
  SettingsTabs.AddTabs "General", "AI"
  Dock.AddPane(AIPanel, "Assistant", "tab:inspector", "ai")
  AIApplySettings
  LayOutAIPanel
  IF AIReady = 0 THEN Dock.HidePane("aipanel")
END SUB

' How tall the meter is for its text wrapped at spaces to W pixels in its
' font, as a WordWrap label wraps it: a line when it fits, more when it
' doesn't (with cached tokens and prices it is longer than the pane is
' wide: "· about $…" is never left out of a label one line high).
FUNCTION AIMeterHigh(W AS INTEGER) AS INTEGER
  DIM lines AS INTEGER, cur AS STRING, wd AS STRING, i AS INTEGER, n AS INTEGER, t AS STRING
  AIMeterMeasure.Font.Name = DimFont.Name: AIMeterMeasure.Font.Size = DimFont.Size
  t = LTRIM$(RTRIM$(AIMeter.Caption))
  lines = 1
  IF t <> "" THEN
    n = TALLY(t, " ") + 1
    FOR i = 1 TO n
      wd = FIELD$(t, " ", i)
      IF cur = "" THEN
        cur = wd
      ELSEIF AIMeterMeasure.TextWidth(cur + " " + wd) > W - 2 THEN
        lines = lines + 1
        cur = wd
      ELSE
        cur = cur + " " + wd
      END IF
    NEXT
  END IF
  ' (one line is 18 pixels, as it always was; each more adds a line's pitch)
  Result = 18 + (lines - 1) * (AIMeterMeasure.TextHeight(" ") + 2)
END FUNCTION

' The meter shows the session's usage (its cost too once prices are
' given); the panel is laid out again when the text needs more lines.
SUB AIShowUsage
  AIMeter.Caption = AI.Usage
  IF AIMeter.Width > 0 THEN
    IF AIMeterHigh(AIMeter.Width) <> AIMeter.Height THEN LayOutAIPanel
  END IF
END SUB

' The panel's controls placed for its width (the docks resize it).
SUB LayOutAIPanel
  DIM w AS INTEGER, bh AS INTEGER, y AS INTEGER, mh AS INTEGER, ew AS INTEGER
  w = AIPanel.ClientWidth
  IF w < 200 THEN w = 200
  IF ThemeName = "classic" THEN bh = 23 ELSE bh = 28
  ' (the chats and this one's title above, New Chat at the right; the
  ' model below with ↻ and the effort: the docks keep the panel narrow)
  AIChatsBtn.Left = 6: AIChatsBtn.Top = 4: AIChatsBtn.Width = 64: AIChatsBtn.Height = bh
  AINewBtn.Left = w - 86: AINewBtn.Top = 4: AINewBtn.Width = 80: AINewBtn.Height = bh
  AIChatTitle.Left = 78: AIChatTitle.Top = 4 + (bh - 18) \ 2: AIChatTitle.Width = w - 86 - 6 - 78: AIChatTitle.Height = 18
  ew = 118
  IF w < 300 THEN ew = 96
  AIEffortBox.Left = w - 6 - ew: AIEffortBox.Top = 38: AIEffortBox.Width = ew
  AIRefreshBtn.Left = w - 6 - ew - 4 - 30: AIRefreshBtn.Top = 37: AIRefreshBtn.Width = 30: AIRefreshBtn.Height = bh
  AIModelBox.Left = 6: AIModelBox.Top = 38: AIModelBox.Width = w - 12 - ew - 4 - 30 - 4
  ' (the box; Send and Stop, the answer's seconds at the right; the
  ' context with Compact; the meter, two lines when narrow; "What was
  ' sent" beside the buttons when it fits, else under the meter; what
  ' Send shares)
  AIInput.Left = 6: AIInput.Top = 6: AIInput.Width = w - 12: AIInput.Height = 64
  AISendBtn.Left = 6: AISendBtn.Top = 76: AISendBtn.Width = 74: AISendBtn.Height = bh
  AIStopBtn.Left = 86: AIStopBtn.Top = 76: AIStopBtn.Width = 74: AIStopBtn.Height = bh
  AIElapsed.Left = w - 62: AIElapsed.Top = 76 + (bh - 16) \ 2: AIElapsed.Width = 56: AIElapsed.Height = 18
  y = 82 + bh
  AICompactBtn.Left = w - 6 - 84: AICompactBtn.Top = y: AICompactBtn.Width = 84: AICompactBtn.Height = bh
  AIContextBar.Left = 8: AIContextBar.Top = y + (bh - 8) \ 2: AIContextBar.Width = 56: AIContextBar.Height = 8
  AIContextText.Left = 70: AIContextText.Top = y + (bh - 18) \ 2: AIContextText.Width = w - 6 - 84 - 6 - 70: AIContextText.Height = 18
  y = y + bh + 6
  mh = AIMeterHigh(w - 16)
  AIMeter.Left = 8: AIMeter.Top = y: AIMeter.Width = w - 16: AIMeter.Height = mh
  y = y + mh + 4
  IF w - 176 - 60 >= 96 THEN
    AISentLink.Left = 170: AISentLink.Top = 76 + (bh - 16) \ 2: AISentLink.Width = w - 176 - 60
  ELSE
    AISentLink.Left = 8: AISentLink.Top = y: AISentLink.Width = w - 16
    y = y + 22
  END IF
  AIEditText.Left = 8: AIEditText.Top = 6: AIEditText.Width = w - 16 - 84: AIEditText.Height = 60
  AIEditCancel.Left = w - 6 - 76: AIEditCancel.Top = (72 - bh) \ 2: AIEditCancel.Width = 76: AIEditCancel.Height = bh
  IF ThemeName = "classic" THEN AISlashList.ItemHeight = 16 ELSE AISlashList.ItemHeight = 22
  LayOutAIChats
  AIShowChatTitle
  AINote.Left = 8: AINote.Top = y: AINote.Width = w - 16: AINote.Height = 64
  IF AIBottom.Height <> y + 68 THEN AIBottom.Height = y + 68
  AIReviewBtn.Left = 6: AIReviewBtn.Top = 9: AIReviewBtn.Width = w - 12: AIReviewBtn.Height = bh
  AIQueueCancel.Left = w - 34: AIQueueCancel.Top = (32 - 26) \ 2: AIQueueCancel.Width = 28: AIQueueCancel.Height = 26
  AIQueueText.Left = 8: AIQueueText.Top = 8: AIQueueText.Width = w - 48: AIQueueText.Height = 18
  AIShowQueue
  AIChat.EmptyText = AIEmptyText
END SUB

SUB AIBannerFonts
  AIBannerFont.Name = UIFont.Name
  AIBannerFont.Size = UIFont.Size + 3
  AIBannerFont.Bold = 1
  AIBanner.Color = TC("card")
  AIBannerMark.Color = TC("accent")
  AIBannerText.Font = AIBannerFont
  AIBannerText.Font.Color = TC("page.text")
  AIBannerRun.Font = UIFont
  AIBannerRun.Font.Bold = 1
  AIBannerFix.Font = UIFont
  AIBannerUndo.Font = UIFont
  AIBannerWhat.Font = UIFont
  AIBannerClose.Font = UIFont
  LayOutAIBanner
END SUB

SUB AIPanelFonts
  AIChat.Font = UIFont
  AIInput.Font = UIFont
  AINote.Font = DimFont
  AINote.Font.Color = TC("page.dim")
  AIMeter.Font = DimFont
  AIMeter.Font.Color = TC("page.dim")
  AISentLink.Font = DimFont
  AISentLink.Font.Color = TC("page.link")
  AIStatus.Font = StatusFont
  AIReviewBtn.Font = UIFont
  AIReviewBtn.Font.Bold = 1
  AIQueueText.Font = UIFont
  AIQueueText.Font.Color = TC("page.text")
  AIQueueCancel.Font = UIFont
  AIChatsBtn.Font = UIFont
  AIChatTitle.Font = UIFont
  AIChatTitle.Font.Bold = 1
  AIChatTitle.Font.Color = TC("page.text")
  AIRefreshBtn.Font = UIFont
  AIEffortBox.Font = UIFont
  AIElapsed.Font = DimFont
  AIElapsed.Font.Color = TC("page.dim")
  AIContextText.Font = DimFont
  AIContextText.Font.Color = TC("page.dim")
  AICompactBtn.Font = UIFont
  AISlashList.Font = UIFont
  AIEditText.Font = UIFont
  AIEditText.Font.Color = TC("page.text")
  AIEditCancel.Font = UIFont
  AIChatsFonts
  AIBannerFonts
  AIRevHead.Font = UIFont
  AINewTitle.Font = SubFont
  AINewWhat.Font = UIFont
  AINewCode.Font = MonoFont
  AISentText.Font = MonoFont
END SUB

' What the empty conversation says: how to start, or what is missing.
FUNCTION AIEmptyText AS STRING
  IF AI.Provider = "" THEN
    Result = "RAI needs a place to answer from: Tools > Set up RAI walks you through it (free on this computer, or a cloud key)."
  ELSEIF AI.NeedsKey AND AI.HasKey = 0 THEN
    Result = "Add your API key (Tools > Set up RAI, or Tools > Settings > AI), then ask about this project or for a new program."
  ELSEIF AI.Model = "" THEN
    Result = "Choose a model in the list above (↻ asks the provider for its models)."
  ELSE
    Result = "Ask about this project, or for a new program: " + CHR$(34) + "Add a Clear button" + CHR$(34) + ", " + CHR$(34) + "Build me a tip calculator" + CHR$(34) + ". A new program opens in Studio, ready to run; changes to your files wait for you to review them. Type / for commands."
  END IF
END FUNCTION

' (the provider's name, as the lists show it)
FUNCTION AIProviderName AS STRING
  DIM k AS INTEGER
  Result = "the provider"
  FOR k = 0 TO AIProvCount - 1
    IF AIProvIds(k) = AI.Provider THEN Result = AIProvNames(k)
  NEXT
  IF AI.Provider = "compatible" THEN Result = "the server at " + AI.BaseURL
END FUNCTION

' The settings into the component and the panel.
SUB AIApplySettings
  DIM k AS INTEGER
  AI.Provider = AIProvider
  ' (the address is a compatible server's, the workspace Anthropic's: a
  ' cloud provider's own address is never replaced by one typed for another)
  IF AIProvider = "compatible" THEN AI.BaseURL = AIBase ELSE AI.BaseURL = ""
  IF AIProvider = "anthropic" THEN AI.WorkspaceID = AIWorkspace ELSE AI.WorkspaceID = ""
  ' (Studio's assistant writes whole programs: 4096 a reply unless the
  ' user set another limit — RAI's own default is smaller)
  IF AIMaxTokens > 0 THEN AI.MaxTokens = AIMaxTokens ELSE AI.MaxTokens = 4096
  AI.PriceIn = AIPriceIn
  AI.PriceOut = AIPriceOut
  AITools.Permission = AIPermission
  AITools.ReviewFirst = AIReviewFirst
  ' (the key: Studio's keychain entry for the provider, else its usual
  ' environment variable — never in the settings)
  AI.APIKeySource = AIKeySourceOf(AIProvider)
  AI.Model = AIModelOf(AIProvider)
  AI.Effort = AIEffort
  AIFilling = 1
  AIFillModels
  AIFillEffort
  AIFilling = 0
  AINote.Caption = "Send shares your message, this chat and what the assistant reads of this project with " + AIProviderName + ": nothing else, nothing before."
  AIChat.EmptyText = AIEmptyText
  AIEnableSend
END SUB

' The model list: the ones fetched (by the assistant, or by Settings for
' the same provider and address), else the one chosen.
SUB AIFillModels
  DIM all AS STRING, i AS INTEGER, line AS STRING, k AS INTEGER
  AIModelBox.Clear
  all = AI.Models
  IF all = "" AND SetAI.Provider = AI.Provider AND SetAI.BaseURL = AI.BaseURL THEN all = SetAI.Models
  k = -1
  IF all <> "" THEN
    FOR i = 1 TO TALLY(all, CHR$(10)) + 1
      line = FIELD$(FIELD$(all, CHR$(10), i), CHR$(9), 1)
      IF line <> "" THEN
        AIModelBox.AddItems AIModelItem(FIELD$(all, CHR$(10), i))
        IF line = AI.Model THEN k = AIModelBox.ItemCount - 1
      END IF
    NEXT
  END IF
  IF k < 0 AND AI.Model <> "" THEN
    AIModelBox.AddItems AI.Model
    k = AIModelBox.ItemCount - 1
  END IF
  AIModelBox.ItemIndex = k
  AIModelBox.Text = AI.Model
  ' (who answers: said, not a row of its own — Settings changes it)
  AIModelBox.AccessibleDescription = "A model of " + AIProviderName + ". The provider is chosen in Tools > Settings > AI, or Set up RAI."
  AIModelBox.Hint = AIProviderName + "'s models (the provider: Tools > Settings > AI)"
  AIModelBox.ShowHint = 1
END SUB

' The effort list as the setting is, offered where the provider and model
' take one (DeepSeek's models choose for themselves).
SUB AIFillEffort
  SELECT CASE AIEffort
    CASE "low": AIEffortBox.ItemIndex = 1
    CASE "medium": AIEffortBox.ItemIndex = 2
    CASE "high": AIEffortBox.ItemIndex = 3
    CASE ELSE: AIEffortBox.ItemIndex = 0
  END SELECT
  AIEffortBox.Enabled = Flag(AI.EffortSupported <> 0)
  IF AI.EffortSupported THEN
    AIEffortBox.Hint = "Low, Medium or High: the model thinks more before it answers (slower, costs more). Auto: as it usually does."
  ELSE
    AIEffortBox.Hint = "This provider's models choose how much to think for themselves."
  END IF
  AIEffortBox.ShowHint = 1
END SUB

' (Effort picked: kept, said)
SUB AIEffortPicked
  DIM e AS STRING
  IF AIFilling THEN EXIT SUB
  SELECT CASE AIEffortBox.ItemIndex
    CASE 1: e = "low"
    CASE 2: e = "medium"
    CASE 3: e = "high"
    CASE ELSE: e = ""
  END SELECT
  AISetEffort e
END SUB

SUB AISetEffort(E AS STRING)
  AIEffort = E
  AI.Effort = E
  SaveSettings
  AIFilling = 1
  AIFillEffort
  AIFilling = 0
  IF E = "" THEN
    AIStatus.SimpleText = "Effort: Auto, as the model usually thinks."
  ELSE
    AIStatus.SimpleText = "Effort: " + UCASE$(LEFT$(E, 1)) + MID$(E, 2) + " — slower, and it costs more."
  END IF
END SUB

' ↻: the provider's models asked for again (a model it no longer lists is
' forgotten, chosen again); the list fills when they come.
SUB AIRefreshClick
  IF AI.Busy THEN
    AIStatus.SimpleText = "RAI is answering: ↻ works once it is done (or Stop)."
    EXIT SUB
  END IF
  IF AI.RefreshModels THEN
    AIRefreshBtn.Enabled = 0
    AIStatus.SimpleText = AI.Status
  ELSE
    AIStatus.SimpleText = AI.Error
  END IF
END SUB

' (a model as the lists show it: its id, and the costlier ones marked —
' nothing hidden)
FUNCTION AIModelItem(Line AS STRING) AS STRING
  Result = FIELD$(Line, CHR$(9), 1)
  IF FIELD$(Line, CHR$(9), 3) <> "" THEN Result = Result + "   " + FIELD$(Line, CHR$(9), 3)
END FUNCTION

' (the model a list's text names: its id, the hint after it left out)
FUNCTION AIModelId(Text AS STRING) AS STRING
  Result = FIELD$(LTRIM$(RTRIM$(Text)), " ", 1)
END FUNCTION

SUB AIEnableSend
  ' (Send while it works queues the message: enabled whenever the
  ' assistant is ready)
  AISendBtn.Enabled = Flag(AIReady <> 0)
  AIStopBtn.Enabled = Flag(AI.Busy <> 0)
  AIRefreshBtn.Enabled = Flag(AI.Busy = 0 AND AI.Provider <> "")
  AICompactBtn.Enabled = Flag(AI.Busy = 0 AND AI.QuestionCount > 0)
  ' (the clock while it works: the seconds, the provider's silence)
  IF AI.Busy THEN
    IF AIClock.Enabled = 0 THEN
      AIQuietSaid = 0
      AIClock.Enabled = 1
    END IF
  ELSE
    AIClock.Enabled = 0
    AIElapsed.Caption = ""
  END IF
  AIShowUsage
  AIShowContext
END SUB

' The context meter: how much the next question sends, against where the
' chat is compacted by itself (SummaryAt, or 70% of the model's window).
SUB AIShowContext
  DIM used AS INTEGER, limit AS INTEGER, pct AS INTEGER, t AS STRING
  used = AI.ContextTokens
  limit = AI.ContextLimit
  IF limit <= 0 THEN limit = 1
  pct = (used * 100) \ limit
  IF pct > 100 THEN pct = 100
  IF AIContextBar.Position <> pct THEN AIContextBar.Position = pct
  IF used <= 0 THEN
    t = "Context: empty"
  ELSE
    t = "Context: " + AIThousands(used) + " of " + AIThousands(limit) + " (" + LTRIM$(STR$(pct)) + "%)"
  END IF
  IF AIContextText.Caption <> t THEN
    AIContextText.Caption = t
    AIContextText.Hint = "About " + AIThousands(used) + " tokens go with the next question. At " + AIThousands(limit) + " the chat is compacted by itself; Compact does it now."
    AIContextText.ShowHint = 1
    AIContextBar.AccessibleDescription = t
  END IF
END SUB

' (1234567 → "1,234,567")
FUNCTION AIThousands(N AS INTEGER) AS STRING
  DIM t AS STRING, acc AS STRING, i AS INTEGER, k AS INTEGER
  t = LTRIM$(STR$(N))
  FOR i = LEN(t) TO 1 STEP -1
    acc = MID$(t, i, 1) + acc
    k = k + 1
    IF k MOD 3 = 0 AND i > 1 THEN acc = "," + acc
  NEXT
  Result = acc
END FUNCTION

' (every second while it works: its seconds beside Stop; past 30 s with no
' word from the provider, the status says so once — wait, or Stop)
SUB AIClockTick(Sender AS RTimer)
  DIM q AS INTEGER, e AS INTEGER
  IF AI.Busy = 0 THEN
    AIClock.Enabled = 0
    AIElapsed.Caption = ""
    EXIT SUB
  END IF
  e = AI.Elapsed
  ' (an answer's seconds; nothing while only the model list is asked for)
  IF AI.Activity = "" AND e = 0 THEN AIElapsed.Caption = "" ELSE AIElapsed.Caption = LTRIM$(STR$(e)) + " s"
  q = AI.Quiet
  IF q >= AIQuietAfter THEN
    IF AIQuietSaid = 0 THEN
      AIQuietSaid = 1
      AIStatus.SimpleText = "No answer from the model for " + LTRIM$(STR$(q)) + " s: wait, or Stop."
      AIProgressSaid = AIStatus.SimpleText
    END IF
  ELSEIF AIQuietSaid THEN
    ' (it speaks again: the status says what it does)
    AIQuietSaid = 0
    AIProgressSaid = ""
    AIProgressShow
  END IF
END SUB

SUB AICompactClick
  IF AI.Compact THEN
    AIStatus.SimpleText = "Compacting the chat…"
    AIEnableSend
  ELSE
    AIStatus.SimpleText = AI.Error
  END IF
END SUB

' (Studio's keychain entry for a provider's key, then the provider's usual
' environment variable: where the assistant's key comes from)
FUNCTION AIKeySourceOf(Id AS STRING) AS STRING
  DIM env AS STRING
  Result = ""
  IF Id = "" THEN EXIT FUNCTION
  Result = "keychain:RapidR Studio/" + Id
  env = AIEnvOf(Id)
  IF env <> "" THEN Result = Result + "|env:" + env
END FUNCTION

' (a provider's usual environment variable for its key)
FUNCTION AIEnvOf(Id AS STRING) AS STRING
  SELECT CASE Id
    CASE "anthropic": Result = "ANTHROPIC_API_KEY"
    CASE "openai": Result = "OPENAI_API_KEY"
    CASE "deepseek": Result = "DEEPSEEK_API_KEY"
    CASE ELSE: Result = ""
  END SELECT
END FUNCTION

' Whether the assistant may show itself: turned on, with a provider, a
' key (a local server needs none) and a model the user chose.
FUNCTION AIReady AS INTEGER
  Result = Flag(AIOn <> 0 AND AI.Provider <> "" AND AI.Model <> "" AND (AI.NeedsKey = 0 OR AI.HasKey <> 0))
END FUNCTION

' (Tools > Settings > AI's choices, kept per user: project.inc's settings)
FUNCTION AIModelOf(Id AS STRING) AS STRING
  DIM i AS INTEGER, line AS STRING
  Result = ""
  FOR i = 1 TO TALLY(AIModels, CHR$(10)) + 1
    line = FIELD$(AIModels, CHR$(10), i)
    IF FIELD$(line, CHR$(9), 1) = Id AND Id <> "" THEN Result = FIELD$(line, CHR$(9), 2)
  NEXT
END FUNCTION

SUB AISetModelOf(Id AS STRING, Model AS STRING)
  DIM i AS INTEGER, line AS STRING, all AS STRING
  IF Id = "" THEN EXIT SUB
  FOR i = 1 TO TALLY(AIModels, CHR$(10)) + 1
    line = FIELD$(AIModels, CHR$(10), i)
    IF line <> "" AND FIELD$(line, CHR$(9), 1) <> Id THEN all = all + line + CHR$(10)
  NEXT
  IF Model <> "" THEN all = all + Id + CHR$(9) + Model + CHR$(10)
  AIModels = all
END SUB

SUB LoadAISettings
  IF Reg.ValueExists("AIOn") THEN AIOn = VAL(Reg.ReadString("AIOn"))
  IF Reg.ValueExists("AIProvider") THEN AIProvider = Reg.ReadString("AIProvider")
  IF Reg.ValueExists("AIModels") THEN AIModels = REPLACESUBSTR$(Reg.ReadString("AIModels"), "|", CHR$(10))
  IF Reg.ValueExists("AIModels") THEN AIModels = REPLACESUBSTR$(AIModels, "=", CHR$(9))
  IF Reg.ValueExists("AIBaseURL") THEN AIBase = Reg.ReadString("AIBaseURL")
  IF Reg.ValueExists("AIWorkspace") THEN AIWorkspace = Reg.ReadString("AIWorkspace")
  IF Reg.ValueExists("AIMaxTokens") THEN AIMaxTokens = VAL(Reg.ReadString("AIMaxTokens"))
  IF Reg.ValueExists("AIPermission") THEN AIPermission = Reg.ReadString("AIPermission")
  IF Reg.ValueExists("AIReviewFirst") THEN AIReviewFirst = VAL(Reg.ReadString("AIReviewFirst"))
  IF Reg.ValueExists("AIEffort") THEN AIEffort = Reg.ReadString("AIEffort")
  IF Reg.ValueExists("AIPrices") THEN
    AIPriceIn = VAL(FIELD$(Reg.ReadString("AIPrices"), "/", 1))
    AIPriceOut = VAL(FIELD$(Reg.ReadString("AIPrices"), "/", 2))
  END IF
END SUB

' (never a key: the keychain keeps those)
SUB SaveAISettings
  DIM m AS STRING
  m = REPLACESUBSTR$(AIModels, CHR$(9), "=")
  m = REPLACESUBSTR$(m, CHR$(10), "|")
  Reg.WriteString("AIOn", LTRIM$(STR$(AIOn)))
  Reg.WriteString("AIProvider", AIProvider)
  Reg.WriteString("AIModels", m)
  Reg.WriteString("AIBaseURL", AIBase)
  Reg.WriteString("AIWorkspace", AIWorkspace)
  Reg.WriteString("AIMaxTokens", LTRIM$(STR$(AIMaxTokens)))
  Reg.WriteString("AIPermission", AIPermission)
  Reg.WriteString("AIReviewFirst", LTRIM$(STR$(AIReviewFirst)))
  Reg.WriteString("AIEffort", AIEffort)
  Reg.WriteString("AIPrices", LTRIM$(STR$(AIPriceIn)) + "/" + LTRIM$(STR$(AIPriceOut)))
END SUB

' Turns the assistant on or off; its pane and tool bar button show once
' it is ready too (a provider, a key, a model).
SUB SetAIOn(IsOn AS INTEGER)
  AIOn = Flag(IsOn)
  AIShowOrHide
END SUB

' The Assistant shares Properties' tab group at the right. The default side
' (240 wide) is too narrow for a chat: the first time the pane shows, its
' column opens 380 wide. Only when the column is still the default width:
' one the user dragged stays as it is. (The width is in the layout's text,
' on the line of the column that sits in the layout's row.)
FUNCTION LayoutIndent(Line AS STRING) AS INTEGER
  LayoutIndent = LEN(Line) - LEN(LTRIM$(Line))
END FUNCTION

SUB WidenAIPanel
  DIM Ls(1 TO 200) AS STRING
  DIM lay AS STRING, n AS INTEGER, k AS INTEGER, j AS INTEGER, at AS INTEGER, up AS INTEGER, cur AS INTEGER
  DIM sp AS INTEGER, lead AS STRING, acc AS STRING
  lay = Dock.SaveLayout
  n = TALLY(lay, CHR$(10)) + 1
  IF n > 200 THEN EXIT SUB
  FOR k = 1 TO n
    Ls(k) = FIELD$(lay, CHR$(10), k)
    IF at = 0 AND INSTR(Ls(k), " tabs ") > 0 AND INSTR(Ls(k), " aipanel") > 0 THEN at = k
  NEXT
  IF at = 0 THEN EXIT SUB
  ' (up the layout's lines until the one whose parent is the row: its
  ' number is the width)
  cur = at
  DO
    up = 0
    FOR j = cur - 1 TO 1 STEP -1
      IF LayoutIndent(Ls(j)) < LayoutIndent(Ls(cur)) THEN
        up = j
        EXIT FOR
      END IF
    NEXT
    IF up = 0 THEN EXIT SUB
    IF INSTR(Ls(up), "split row") > 0 THEN EXIT DO
    cur = up
  LOOP
  IF VAL(LTRIM$(Ls(cur))) < 1 OR VAL(LTRIM$(Ls(cur))) > 260 THEN EXIT SUB
  lead = LEFT$(Ls(cur), LayoutIndent(Ls(cur)))
  sp = INSTR(LTRIM$(Ls(cur)), " ")
  Ls(cur) = lead + "380" + MID$(LTRIM$(Ls(cur)), sp)
  FOR k = 1 TO n
    acc = acc + Ls(k)
    IF k < n THEN acc = acc + CHR$(10)
  NEXT
  Dock.LoadLayout(acc)
  ' (the dock sets the panel's width at once; a panel's OnResize doesn't come from that)
  LayOutAIPanel
END SUB

SUB AIShowOrHide
  DIM wasHidden AS INTEGER
  BuildToolBar
  UpdateCommands
  IF AIReady THEN
    wasHidden = (Dock.PaneState("aipanel") = "hidden")
    Dock.ShowPane("aipanel")
    IF wasHidden THEN WidenAIPanel
  ELSE
    AI.Cancel
    Dock.HidePane("aipanel")
  END IF
  ' (Send and Stop as the assistant now is: ready or not, a key saved or
  ' removed in Settings)
  AIEnableSend
  AIChat.EmptyText = AIEmptyText
END SUB

' View > AI Assistant (Ctrl+I): the panel, the keyboard in its box; not
' set up yet, Set up RAI (the provider chosen, a model; Settings > AI has
' every detail).
SUB ShowAIPanel
  IF AIReady = 0 THEN
    ShowAISetup
    EXIT SUB
  END IF
  Dock.FocusPane("aipanel")
  LayOutAIPanel
  AIInput.SetFocus
END SUB

' ---- asking ------------------------------------------------------------------------

' What the assistant sees of the project: its folder and files, the open
' editors' texts (unsaved changes included).
SUB AISyncProject
  DIM i AS INTEGER, all AS STRING, d AS INTEGER
  AITools.ClearFiles
  IF ProjectOpen = 0 THEN
    AITools.ProjectFolder = ""
    AITools.ProjectFiles = ""
    AITools.MainFile = ""
    AITools.ProjectName = ""
    EXIT SUB
  END IF
  FOR i = 0 TO Proj.FileCount - 1
    all = all + Proj.File(i) + CHR$(10)
  NEXT
  AITools.ProjectFolder = Proj.Folder
  AITools.ProjectFiles = all
  AITools.MainFile = Proj.MainFile
  AITools.ProjectName = Proj.Name
  AITools.CompatMode = Proj.CompatMode
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN AITools.SetFile DocFile(d), CodeDoc(d).Text
  NEXT
END SUB

SUB AISend
  DIM t AS STRING
  t = AIInput.Text
  ' (the line break Enter may have left at its end isn't the user's)
  DO WHILE RIGHT$(t, 1) = CHR$(10) OR RIGHT$(t, 1) = CHR$(13)
    t = LEFT$(t, LEN(t) - 1)
  LOOP
  IF LTRIM$(RTRIM$(t)) = "" THEN EXIT SUB
  ' (a / command: Studio's, never sent)
  IF LEFT$(LTRIM$(t), 1) = "/" AND AIEditing = 0 THEN
    IF AIRunSlash(LTRIM$(t)) THEN
      AIInput.Text = ""
      AIShowSlash
    END IF
    EXIT SUB
  END IF
  ' (a sent question edited: what followed it moves to a saved copy, then
  ' the new text is asked — aichats.inc)
  IF AIEditing > 0 THEN
    IF AISendEdited(t) THEN AIInput.Text = ""
    EXIT SUB
  END IF
  ' (while it works: the message waits, shown, and goes when the answer
  ' ends — typing never stops an answer; a second one joins the first)
  IF AI.Busy THEN
    IF AIQueued = "" THEN AIQueued = t ELSE AIQueued = AIQueued + CHR$(10) + CHR$(10) + t
    AIInput.Text = ""
    AIShowQueue
    AIInput.SetFocus
    EXIT SUB
  END IF
  AISendNow t, 1
END SUB

' (a question to the assistant now: typed in the box, or the one queued)
SUB AISendNow(T AS STRING, FromBox AS INTEGER)
  ' (the answer followed again, a thought opened before or not)
  AIChat.FollowEnd = 1
  AISyncProject
  AIReviewBar.Visible = 0
  AIProgressSaid = ""
  IF AI.Ask(T) THEN
    IF FromBox THEN AIInput.Text = ""
  ELSE
    AIStatus.SimpleText = AI.Error
    ' (not sent: the words go back to the box, never lost)
    IF FromBox = 0 THEN
      IF AIInput.Text = "" THEN AIInput.Text = T ELSE AIInput.Text = T + CHR$(10) + AIInput.Text
    END IF
  END IF
  AIChat.Text = AI.Transcript
  AIEnableSend
  AIProgressShow
END SUB

' (the queued message, above the box: its first line, shortened to fit)
SUB AIShowQueue
  DIM first AS STRING, room AS INTEGER, k AS INTEGER
  IF AIQueued = "" THEN
    IF AIQueueBar.Visible THEN AIQueueBar.Visible = 0
    EXIT SUB
  END IF
  first = AIQueued
  k = INSTR(first, CHR$(10))
  IF k > 0 THEN first = LEFT$(first, k - 1) + " …"
  room = (AIQueueText.Width - 60) \ 7
  IF room < 12 THEN room = 12
  IF LEN(first) > room THEN first = LEFT$(first, room - 1) + "…"
  AIQueueText.Caption = "Queued: " + first
  AIQueueText.AccessibleName = "Queued: " + AIQueued
  AIQueueText.Hint = AIQueued
  IF AIQueueBar.Visible = 0 THEN AIQueueBar.Visible = 1
END SUB

' (×: the queued message isn't sent)
SUB AIQueueCancelClick
  AIQueued = ""
  AIShowQueue
  AIStatus.SimpleText = "The queued message won't be sent."
  AIInput.SetFocus
END SUB

' (Enter sends; Shift+Enter is a new line; while the / commands show, Up and
' Down choose one, Tab completes it, Enter runs it, Escape closes them;
' Escape while a question is edited stops editing)
SUB AIInputKey(Key AS INTEGER, Shift AS INTEGER)
  IF AISlashBar.Visible THEN
    IF Key = 38 OR Key = 40 THEN
      IF Key = 38 AND AISlashList.ItemIndex > 0 THEN AISlashList.ItemIndex = AISlashList.ItemIndex - 1
      IF Key = 40 AND AISlashList.ItemIndex < AISlashList.ItemCount - 1 THEN AISlashList.ItemIndex = AISlashList.ItemIndex + 1
      Key = 0
      EXIT SUB
    END IF
    IF Key = 9 AND Shift = 0 THEN
      Key = 0
      AISlashComplete
      EXIT SUB
    END IF
    IF Key = 27 THEN
      Key = 0
      AISlashBar.Visible = 0
      EXIT SUB
    END IF
    IF Key = 13 AND Shift = 0 THEN
      ' (a command not typed whole: the one chosen)
      IF INSTR(AIInput.Text, " ") = 0 AND AISlashList.ItemIndex >= 0 THEN AIInput.Text = FIELD$(AISlashList.Item(AISlashList.ItemIndex), " ", 1)
    END IF
  END IF
  IF Key = 27 AND AIEditing > 0 THEN
    Key = 0
    AIEditCancelClick
    EXIT SUB
  END IF
  IF Key = 13 AND Shift = 0 THEN
    Key = 0
    AISend
  END IF
END SUB

' ---- the / commands (R5) -------------------------------------------------------------
' Typed at the start of the box: listed as they are typed, run on Enter,
' never sent to the model.

' (the commands, a line each: the command, what it does)
FUNCTION AISlashCommands AS STRING
  Result = "/compact" + CHR$(9) + "Summarise this chat now: each question then sends less" + CHR$(10)
  Result = Result + "/new" + CHR$(9) + "Start a new chat (this one is kept in Chats)" + CHR$(10)
  Result = Result + "/chats" + CHR$(9) + "This project's chats: switch, rename, delete, search" + CHR$(10)
  Result = Result + "/model" + CHR$(9) + "Choose the model (/model name picks it)" + CHR$(10)
  Result = Result + "/effort" + CHR$(9) + "How hard it thinks: /effort low, medium, high or auto" + CHR$(10)
  Result = Result + "/undo" + CHR$(9) + "Undo the assistant's last change" + CHR$(10)
  Result = Result + "/help" + CHR$(9) + "What you can type here"
END FUNCTION

' The list under the box follows what is typed: the commands that start
' with it ("/c" → /compact, /chats); none, hidden.
SUB AIInputChanged
  AIShowSlash
END SUB

SUB AIShowSlash
  DIM t AS STRING, all AS STRING, i AS INTEGER, line AS STRING, cw AS STRING, shown AS STRING, n AS INTEGER
  t = LCASE$(LTRIM$(AIInput.Text))
  IF LEFT$(t, 1) <> "/" OR INSTR(t, " ") > 0 OR INSTR(t, CHR$(10)) > 0 OR AIEditing > 0 THEN
    IF AISlashBar.Visible THEN AISlashBar.Visible = 0
    AISlashShown = ""
    EXIT SUB
  END IF
  all = AISlashCommands
  FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    line = FIELD$(all, CHR$(10), i)
    cw = FIELD$(line, CHR$(9), 1)
    IF LEFT$(cw, LEN(t)) = t THEN
      shown = shown + cw + "   " + FIELD$(line, CHR$(9), 2) + CHR$(10)
      n = n + 1
    END IF
  NEXT
  IF n = 0 THEN
    AISlashBar.Visible = 0
    AISlashShown = ""
    EXIT SUB
  END IF
  IF shown <> AISlashShown THEN
    AISlashShown = shown
    AISlashList.Clear
    FOR i = 1 TO n
      AISlashList.AddItems FIELD$(shown, CHR$(10), i)
    NEXT
    AISlashList.ItemIndex = 0
    AISlashBar.Height = n * 22 + 8
  END IF
  IF AISlashBar.Visible = 0 THEN AISlashBar.Visible = 1
END SUB

' (Tab, or a click: the command chosen, in the box)
SUB AISlashComplete
  IF AISlashList.ItemIndex < 0 THEN EXIT SUB
  AIInput.Text = FIELD$(AISlashList.Item(AISlashList.ItemIndex), " ", 1) + " "
  AISlashBar.Visible = 0
  AIInput.SetFocus
END SUB

SUB AISlashPicked
  AISlashComplete
END SUB

' Runs a / command; False when it isn't one (the box keeps it, the status
' says so).
FUNCTION AIRunSlash(T AS STRING) AS INTEGER
  DIM cmd AS STRING, arg AS STRING, k AS INTEGER
  k = INSTR(T, " ")
  IF k > 0 THEN
    cmd = LCASE$(LEFT$(T, k - 1))
    arg = LTRIM$(RTRIM$(MID$(T, k + 1)))
  ELSE
    cmd = LCASE$(RTRIM$(T))
  END IF
  Result = 1
  SELECT CASE cmd
    CASE "/compact": AICompactClick
    CASE "/new": AINewChat
    CASE "/chats": AIShowChats 1
    CASE "/model"
      IF arg = "" THEN
        AIModelBox.SetFocus
        AIStatus.SimpleText = "Choose the model in the list (↻ asks " + AIProviderName + " for its models again)."
      ELSE
        AIModelBox.Text = arg
        AIModelPicked
        AIStatus.SimpleText = "The model is " + AI.Model + " now."
      END IF
    CASE "/effort"
      SELECT CASE LCASE$(arg)
        CASE "low", "medium", "high": AISetEffort LCASE$(arg)
        CASE "auto", "off", "": AISetEffort ""
        CASE ELSE
          AIStatus.SimpleText = "/effort takes low, medium, high or auto."
          Result = 0
      END SELECT
    CASE "/undo": AIUndoLast
    CASE "/help"
      AIStatus.SimpleText = "Commands: /compact, /new, /chats, /model, /effort, /undo, /help. Type / to see them listed."
      AIInput.Text = "/"
      AIShowSlash
      Result = 0
    CASE ELSE
      AIStatus.SimpleText = cmd + " isn't a command: /help lists them. (To send it, start it with a space.)"
      Result = 0
  END SELECT
END FUNCTION

' /undo: what the assistant changed last taken back — a new program it
' made (as the banner's Undo), else the files the last review changed, each
' as one undo step, when its editor still has what the review left.
SUB AIUndoLast
  DIM i AS INTEGER, d AS INTEGER, done AS INTEGER, stale AS STRING, patch AS STRING
  IF AIBanner.Visible AND AITools.CreatedFolder <> "" THEN
    AIUndoProject
    EXIT SUB
  END IF
  IF AIUndoCount = 0 THEN
    AIStatus.SimpleText = "Nothing to undo: the assistant changed nothing since Studio started (or it was undone)."
    EXIT SUB
  END IF
  FOR i = 0 TO AIUndoCount - 1
    d = AIUndoDoc(i)
    IF d >= 0 AND d < DocCount THEN
      IF DocOpen(d) AND AIMatches(CodeDoc(d).Text, AIUndoAfter(i)) THEN
        patch = AITools.Patch(CodeDoc(d).Text, AIUndoBefore(i))
        IF patch <> "" THEN
          IF CodeDoc(d).ApplyPatches(patch, 0) THEN
            ProjectFileChanged d
            done = done + 1
          END IF
        END IF
      ELSE
        IF stale <> "" THEN stale = stale + ", "
        IF DocOpen(d) THEN stale = stale + FileTitle(DocFile(d)) ELSE stale = stale + "a closed file"
      END IF
    END IF
  NEXT
  AIUndoCount = 0
  IF stale <> "" THEN
    AIStatus.SimpleText = "Undone in " + LTRIM$(STR$(done)) + " file(s); " + stale + " changed since: Edit > Undo there."
  ELSEIF done = 1 THEN
    AIStatus.SimpleText = "Undone: the assistant's last change is taken back (Edit > Redo puts it back)."
  ELSE
    AIStatus.SimpleText = "Undone: the assistant's last changes are taken back in " + LTRIM$(STR$(done)) + " files."
  END IF
  IF done > 0 THEN AIRecheck
END SUB

' (Cancel, or Escape: the question isn't edited; the box empties)
SUB AIEditCancelClick
  AIEditing = 0
  AIEditBar.Visible = 0
  AIInput.Text = ""
  AIStatus.SimpleText = "Not edited: the chat is as it was."
  AIInput.SetFocus
END SUB

' (Stop ends the answer; a queued message goes back to the box, unsent)
SUB AIStopClick
  AI.Cancel
  AIEnableSend
END SUB

' New Chat: this one kept (Chats lists it), a new one started.
SUB AINewChat
  AIStartNewChat
  AITools.ClearProposals
  AIReviewBar.Visible = 0
  AIEnableSend
  AIInput.SetFocus
END SUB

SUB AIUpdate
  AIChat.Text = AI.Transcript
  AIEnableSend
  AIProgressShow
END SUB

' The progress checklist while an answer runs, in the one status line (a
' screen reader announces it politely, once a step): the newest things the
' tools did, ✓ each, and what goes on now — "✓ Found an example · ✓ Wrote
' the program · … Checking". The answer done, the status says what it left.
SUB AIProgressShow
  DIM steps AS STRING, n AS INTEGER, i AS INTEGER, txt AS STRING, doing AS STRING, newest AS STRING, stepFrom AS INTEGER
  IF AI.Busy = 0 THEN EXIT SUB
  steps = AI.Steps
  IF steps = "" THEN n = 0 ELSE n = TALLY(steps, CHR$(10)) + 1
  IF n > 0 THEN newest = FIELD$(steps, CHR$(10), n)
  ' (what it does now, as RAI says it — Thinking, Reading sites.rr,
  ' Searching the manual, Writing server.rr, Checking the code —; between
  ' requests, what the last step left)
  IF AI.Activity <> "" THEN
    doing = AI.Activity + "…"
  ELSEIF INSTR(newest, " to fix") > 0 THEN
    doing = "Fixing the errors…"
  ELSEIF n = 0 THEN
    doing = "Thinking…"
  ELSE
    doing = "Working…"
  END IF
  ' (what it does now first — the line is one line, a screen reader reads
  ' it from the start —, then the newest steps done, ✓ each)
  txt = doing
  stepFrom = n - 1
  IF stepFrom < 1 THEN stepFrom = 1
  FOR i = n TO stepFrom STEP -1
    txt = txt + " · ✓ " + FIELD$(steps, CHR$(10), i)
  NEXT
  ' (the provider silent for 30 s: that stays said until it speaks)
  IF AIQuietSaid THEN EXIT SUB
  IF txt <> AIProgressSaid THEN
    AIProgressSaid = txt
    AIStatus.SimpleText = txt
  END IF
END SUB

SUB AIDone(Status AS STRING)
  DIM n AS INTEGER
  AIChat.Text = AI.Transcript
  ' (one sentence, announced: what happened, and what to do now)
  AIStatus.SimpleText = AI.Status
  AIProgressShow
  AIEnableSend
  n = AITools.ProposalCount
  ' (an answer that failed after its tools — a local server short of memory
  ' for the last sentence — still leaves what they made: Robert's walk of
  ' 2026-10-18; Stop leaves it to Preview)
  IF (Status = "ok" OR Status = "error") AND n > 0 THEN
    IF AIProjectProposal AND AIReviewFirst = 0 THEN
      ' (a new program: created and opened at once, its banner on top)
      AICreateNow
    ELSE
      AIShowReviewBar
      ' (R4: what it waits for, said)
      IF AIReviewBar.Visible AND LEFT$(AIStatus.SimpleText, 16) <> "Waiting for you" THEN AIStatus.SimpleText = "Waiting for you — " + AIStatus.SimpleText
    END IF
  END IF
  AIShowUsage
  ' (R1: the chat kept, as it is now)
  AISaveChat
  AISendQueued Status
END SUB

' (the message queued while it worked, once it is done: sent now — or,
' after Stop, given back to the box, unsent)
SUB AISendQueued(Status AS STRING)
  DIM q AS STRING
  IF AIQueued = "" OR AI.Busy <> 0 THEN EXIT SUB
  q = AIQueued
  AIQueued = ""
  AIShowQueue
  IF Status = "cancelled" THEN
    IF AIInput.Text = "" THEN AIInput.Text = q ELSE AIInput.Text = q + CHR$(10) + AIInput.Text
    AIStatus.SimpleText = "Stopped. Your queued message is back in the box, not sent."
  ELSE
    AISendNow q, 0
  END IF
END SUB

' (what waits for the user: a change to review, or — Review first — the
' new program to look at; one button that says which)
SUB AIShowReviewBar
  IF AITools.ProposalCount = 0 THEN EXIT SUB
  IF AIProjectProposal THEN
    AIReviewBtn.Caption = "Preview the New Program..."
    AIReviewBtn.AccessibleName = "Preview the new program before it is created"
  ELSE
    AIReviewBtn.Caption = "Review the Changes..."
    AIReviewBtn.AccessibleName = "Review the proposed changes"
  END IF
  AIReviewBar.Visible = 1
END SUB

' (no key: Tools > Settings > AI asks for one)
SUB AIKeyNeeded
  AIStatus.SimpleText = "There is no key for " + AIProviderName + ": Tools > Settings > AI."
END SUB

SUB AIErrorSaid(Message AS STRING)
  OutputLine "AI: " + Message
  AIStatus.SimpleText = Message
  AIEnableSend
END SUB

SUB AIModelsIn(Count AS INTEGER)
  AIFilling = 1
  ' (nothing is chosen for the user; a model chosen before that the
  ' provider no longer lists is forgotten: chosen again)
  IF AI.Model = "" AND AIModelOf(AI.Provider) <> "" THEN
    OutputLine "AI: " + AIModelOf(AI.Provider) + " is no longer in the provider's list: choose a model again"
    AISetModelOf AI.Provider, ""
  END IF
  AIFillModels
  AIFillEffort
  AIFilling = 0
  AIStatus.SimpleText = AI.Status
  IF AIModelBox.ItemCount > 0 AND AI.Model = "" THEN AIStatus.SimpleText = AI.Status + " Choose one in the list."
  AIEnableSend
  ' (a message sent while the list came: it goes now)
  AISendQueued "ok"
END SUB

SUB AIModelPicked
  IF AIFilling THEN EXIT SUB
  AI.Model = AIModelId(AIModelBox.Text)
  AISetModelOf AI.Provider, AI.Model
  ' (the model chosen is kept, as Settings keeps it)
  SaveSettings
  AIFilling = 1
  AIFillEffort
  AIFilling = 0
  AIEnableSend
  AIShowOrHide
END SUB

SUB AISentLinkClick
  ShowWhatWasSent
END SUB

SUB ShowWhatWasSent
  IF AI.LastRequest = "" THEN
    AISentText.Text = "Nothing has been sent yet."
  ELSE
    AISentText.Text = AI.LastRequest
  END IF
  AISentWhere.Caption = "Your key is never shown or logged. Every request and change is in the audit log on this computer: " + AI.AuditLocation
  AISentOk.Left = AISent.ClientWidth - 104: AISentOk.Top = 14: AISentOk.Width = 88: AISentOk.Height = 30
  AISentWhere.Top = 10: AISentWhere.Height = 40
  AISentWhere.Width = AISent.ClientWidth - 140
  AISent.Center
  AISent.ShowModal
END SUB

SUB AISentClose
  AISent.Close
END SUB

' ---- the review --------------------------------------------------------------------

' Whether the answer proposes a new project (not changes to this one).
FUNCTION AIProjectProposal AS INTEGER
  DIM i AS INTEGER
  Result = 0
  FOR i = 0 TO AITools.ProposalCount - 1
    IF FIELD$(AITools.ProposalInfo(i), CHR$(9), 1) = "project" AND FIELD$(AITools.ProposalInfo(i), CHR$(9), 6) <> "created" THEN Result = 1
  NEXT
END FUNCTION

SUB AIReviewClick
  IF AIProjectProposal THEN ShowAINew ELSE ShowAIReview
END SUB

SUB ShowAIReview
  DIM i AS INTEGER, info AS STRING, n AS INTEGER
  n = AITools.ProposalCount
  IF n = 0 THEN EXIT SUB
  IF n > AIMAXPROPS THEN n = AIMAXPROPS
  AIRevList.Clear
  FOR i = 0 TO n - 1
    info = AITools.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 1) = "create" THEN
      AIRevList.AddItems FIELD$(info, CHR$(9), 2) + " (new)"
    ELSE
      AIRevList.AddItems FIELD$(info, CHR$(9), 2)
    END IF
    AIRevResult(i) = AITools.ProposalOld(i)
    AIRevSeen(i) = 0
  NEXT
  AIRevAt = -1
  AIRevList.ItemIndex = 0
  AIRevShow 0
  AIRevLayOut
  AIReview.Center
  AIReview.ShowModal
END SUB

SUB AIRevLayOut
  DIM w AS INTEGER
  w = AIRevFoot.ClientWidth
  AIRevCancelBtn.Left = w - 104: AIRevCancelBtn.Top = 12: AIRevCancelBtn.Width = 88: AIRevCancelBtn.Height = 30
  AIRevApplyBtn.Left = w - 200: AIRevApplyBtn.Top = 12: AIRevApplyBtn.Width = 88: AIRevApplyBtn.Height = 30
  AIRevNone.Left = w - 316: AIRevNone.Top = 12: AIRevNone.Width = 108: AIRevNone.Height = 30
  AIRevAll.Left = w - 432: AIRevAll.Top = 12: AIRevAll.Width = 108: AIRevAll.Height = 30
  AIRevInfo.Width = w - 460
END SUB

' (the proposal on show: its decisions kept before another is shown)
SUB AIRevKeep
  IF AIRevAt >= 0 THEN
    AIRevResult(AIRevAt) = AIRevDiff.ResultText
  END IF
END SUB

SUB AIRevShow(i AS INTEGER)
  DIM info AS STRING, h AS INTEGER
  AIRevKeep
  AIRevAt = i
  info = AITools.ProposalInfo(i)
  AIRevHead.Caption = FIELD$(info, CHR$(9), 2) + ": " + FIELD$(info, CHR$(9), 3)
  AIRevHead.Width = AIReview.ClientWidth - 32
  AIRevDiff.Language = FIELD$(info, CHR$(9), 2)
  AIRevDiff.LeftText = AITools.ProposalOld(i)
  AIRevDiff.RightText = AITools.ProposalNew(i)
  ' (what was decided before, shown again)
  IF AIRevSeen(i) THEN
    IF AIRevResult(i) = AIRevDiff.RightText THEN AIRevDiff.AcceptAll
  END IF
  AIRevSeen(i) = 1
  AIRevSummary
END SUB

SUB AIRevPicked
  IF AIRevList.ItemIndex >= 0 THEN AIRevShow AIRevList.ItemIndex
END SUB

SUB AIRevHunk(Index AS INTEGER, Accepted AS INTEGER)
  AIRevResult(AIRevAt) = AIRevDiff.ResultText
  AIRevSummary
END SUB

SUB AIRevSummary
  AIRevInfo.Caption = LTRIM$(STR$(AIRevDiff.AcceptedCount)) + " of " + LTRIM$(STR$(AIRevDiff.HunkCount)) + " changes accepted in this file. F7: next change; Enter accepts it, Backspace rejects it."
END SUB

SUB AIRevAcceptAll
  DIM i AS INTEGER
  FOR i = 0 TO AITools.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AITools.ProposalNew(i)
      AIRevSeen(i) = 1
    END IF
  NEXT
  AIRevDiff.AcceptAll
  AIRevSummary
END SUB

SUB AIRevRejectAll
  DIM i AS INTEGER
  FOR i = 0 TO AITools.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AITools.ProposalOld(i)
      AIRevSeen(i) = 1
    END IF
  NEXT
  AIRevDiff.RejectAll
  AIRevSummary
END SUB

SUB AIRevApplyClick
  AIRevKeep
  AIReview.Close
  AIApplyProposals
END SUB

SUB AIRevCancel
  AIReview.Close
END SUB

' Applies what the review left: each file changed in its editor as one
' undo step (opened when it wasn't; saved by the user, as any change); a
' new file written, added to the project and opened. Each decision goes
' to the conversation and the audit log.
SUB AIApplyProposals
  DIM i AS INTEGER, info AS STRING, kind AS STRING, path AS STRING, full AS STRING
  DIM d AS INTEGER, newText AS STRING, patch AS STRING, done AS INTEGER, f AS RFileStream
  DIM before AS STRING, undoN AS INTEGER
  FOR i = 0 TO AITools.ProposalCount - 1
    IF i >= AIMAXPROPS THEN EXIT FOR
    info = AITools.ProposalInfo(i)
    kind = FIELD$(info, CHR$(9), 1)
    path = FIELD$(info, CHR$(9), 2)
    full = ProjectPath(path)
    newText = AIRevResult(i)
    IF newText = AITools.ProposalOld(i) THEN
      AITools.Decide i, "rejected"
    ELSEIF kind = "create" AND FILEEXISTS(full) = 0 THEN
      f.Open(full, 65535)
      f.WriteStr(newText, LEN(newText))
      f.Close
      IF Proj.AddFile(full) THEN
        IF Proj.Kind = "project" THEN SaveProjectFile
      END IF
      FillProjectTree
      OpenDocument full
      AITools.Decide i, "applied"
      done = done + 1
    ELSE
      d = DocOf(full)
      IF d < 0 THEN
        OpenDocument full
        d = DocOf(full)
      END IF
      IF d < 0 THEN
        OutputLine "AI: can't open " + path + " to change it"
      ELSEIF AIMatches(CodeDoc(d).Text, AITools.ProposalOld(i)) = 0 THEN
        ' (the user changed it since the assistant read it: never overwritten)
        OutputLine "AI: " + path + " changed since the assistant read it; its change wasn't applied (ask again)"
        AITools.Decide i, "stale"
      ELSE
        before = CodeDoc(d).Text
        patch = AITools.Patch(CodeDoc(d).Text, newText)
        ' (said only when it really changed: a patch the editor refused
        ' leaves the file as it was, and the chat says so)
        IF patch <> "" AND CodeDoc(d).ApplyPatches(patch, 0) = 0 THEN
          OutputLine "AI: the change to " + path + " couldn't be applied in its editor (ask again)"
          AITools.Decide i, "failed"
        ELSE
          ' (/undo: the file as it was and became)
          IF undoN < AIMAXPROPS THEN
            AIUndoDoc(undoN) = d
            AIUndoBefore(undoN) = before
            AIUndoAfter(undoN) = CodeDoc(d).Text
            undoN = undoN + 1
          END IF
          ProjectFileChanged d
          IF newText = AITools.ProposalNew(i) THEN AITools.Decide i, "applied" ELSE AITools.Decide i, "partly"
          done = done + 1
        END IF
      END IF
    END IF
  NEXT
  ' (what was decided is done with: a review opened again shows only what
  ' still waits, and nothing is said twice)
  AITools.ClearDecided
  AIReviewBar.Visible = 0
  IF undoN > 0 THEN AIUndoCount = undoN
  AIChat.Text = AI.Transcript
  IF done = 0 THEN
    AIStatus.SimpleText = "Nothing was changed: no change was accepted."
  ELSEIF done = 1 THEN
    AIStatus.SimpleText = "1 file changed: Edit > Undo takes it back."
  ELSE
    AIStatus.SimpleText = LTRIM$(STR$(done)) + " files changed: Edit > Undo in each takes it back."
  END IF
  ' (the changes in, is it a program that runs? the banner and the status say)
  IF done > 0 THEN AIRecheck
  ' (the decisions are in the chat: kept)
  AISaveChat
  UpdateCommands
END SUB

' (two texts the same but for their line breaks)
FUNCTION AIMatches(A AS STRING, B AS STRING) AS INTEGER
  Result = Flag(REPLACESUBSTR$(A, CHR$(13), "") = REPLACESUBSTR$(B, CHR$(13), ""))
END FUNCTION

' ---- a new program --------------------------------------------------------------------

SUB ShowAINew
  DIM i AS INTEGER, info AS STRING, mainAt AS INTEGER, name AS STRING
  AINewFiles.Clear
  mainAt = 0
  FOR i = 0 TO AITools.ProposalCount - 1
    info = AITools.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 1) = "project" THEN
      AINewFiles.AddItems FIELD$(info, CHR$(9), 2)
      IF FIELD$(info, CHR$(9), 5) = "1" THEN mainAt = AINewFiles.ItemCount - 1
      name = FIELD$(info, CHR$(9), 4)
      AINewWhat.Caption = FIELD$(info, CHR$(9), 3)
    END IF
  NEXT
  IF AINewFiles.ItemCount = 1 THEN
    AINewTitle.Caption = name + ": 1 file, previewed: nothing is written until you press Create"
  ELSE
    AINewTitle.Caption = name + ": " + LTRIM$(STR$(AINewFiles.ItemCount)) + " files, previewed: nothing is written until you press Create"
  END IF
  IF AINewFolder.Text = "" THEN AINewFolder.Text = DefaultFolder
  AINewFiles.ItemIndex = mainAt
  AINewShow mainAt
  AINewLayOut
  AINew.Center
  AINew.ShowModal
END SUB

SUB AINewLayOut
  DIM w AS INTEGER
  w = AINewFoot.ClientWidth
  AINewCancelBtn.Left = w - 104: AINewCancelBtn.Top = 12: AINewCancelBtn.Width = 88: AINewCancelBtn.Height = 30
  AINewCreateBtn.Left = w - 252: AINewCreateBtn.Top = 12: AINewCreateBtn.Width = 140: AINewCreateBtn.Height = 30
  AINewBrowseBtn.Left = w - 360: AINewBrowseBtn.Top = 12: AINewBrowseBtn.Width = 100: AINewBrowseBtn.Height = 30
  AINewFolder.Width = w - 360 - 8 - AINewFolder.Left
END SUB

' (the k-th file of the new project: its proposal)
FUNCTION AINewProposal(k AS INTEGER) AS INTEGER
  DIM i AS INTEGER, seen AS INTEGER
  Result = -1
  seen = -1
  FOR i = 0 TO AITools.ProposalCount - 1
    IF FIELD$(AITools.ProposalInfo(i), CHR$(9), 1) = "project" THEN
      seen = seen + 1
      IF seen = k THEN
        Result = i
        EXIT FUNCTION
      END IF
    END IF
  NEXT
END FUNCTION

SUB AINewShow(k AS INTEGER)
  DIM i AS INTEGER
  i = AINewProposal(k)
  IF i < 0 THEN EXIT SUB
  AINewCode.Language = FIELD$(AITools.ProposalInfo(i), CHR$(9), 2)
  AINewCode.Text = AITools.ProposalNew(i)
  ' (its form, drawn as the program will draw it)
  AINewForm.Source = AITools.ProposalNew(i)
  AINewForm.Visible = Flag(AINewForm.FormName <> "")
END SUB

SUB AINewPicked
  IF AINewFiles.ItemIndex >= 0 THEN AINewShow AINewFiles.ItemIndex
END SUB

SUB AINewBrowse
  OpenDlg.Caption = "Where the New Program Goes"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN AINewFolder.Text = OpenDlg.FileName
  OpenDlg.PickFolder = 0
END SUB

SUB AINewCreate
  DIM made AS STRING, errs AS INTEGER
  IF AINewFolder.Text = "" THEN AINewFolder.Text = DefaultFolder
  IF DIREXISTS(AINewFolder.Text) = 0 THEN MKDIR AINewFolder.Text
  errs = AITools.ProjectErrors
  IF AIUnsavedAsked = 0 THEN EXIT SUB
  IF ProjectOpen THEN AIPrevProject = ProjectFile ELSE AIPrevProject = ""
  made = AITools.CreateProject(AINewFolder.Text)
  IF made = "" OR LEFT$(made, 6) = "error:" THEN
    SHOWMESSAGE "Can't create the project: " + MID$(made, 8)
    EXIT SUB
  END IF
  AINew.Close
  AIOpenMade made, errs
END SUB

' (the open project's unsaved changes, before another project opens:
' each document asked about, Save / Don't Save / Cancel — 0 on Cancel)
FUNCTION AIUnsavedAsked AS INTEGER
  DIM d AS INTEGER
  Result = 1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN
      IF AskToSave(d) = 0 THEN
        Result = 0
        EXIT FUNCTION
      END IF
    END IF
  NEXT
END FUNCTION

' A new program the assistant made, straight into the IDE (docs/ai-ux-plan.md
' §1): created in the projects folder (Documents/RapidR/Projects when Studio
' is installed) in a folder named from its title, no question asked — the
' user's unsaved changes asked about first, never lost —, opened (its form
' in the designer), the banner on top with Run it. Nothing runs.
SUB AICreateNow
  DIM made AS STRING, errs AS INTEGER
  errs = AITools.ProjectErrors
  IF AIUnsavedAsked = 0 THEN
    ' (kept to create later: Preview… creates it)
    AIShowReviewBar
    AIStatus.SimpleText = "The new program isn't created yet: your changes came first. Preview the New Program creates it."
    EXIT SUB
  END IF
  IF ProjectOpen THEN AIPrevProject = ProjectFile ELSE AIPrevProject = ""
  made = AITools.CreateProject(DefaultFolder)
  IF made = "" OR LEFT$(made, 6) = "error:" THEN
    AIShowReviewBar
    AIStatus.SimpleText = "Can't create the new program in " + DefaultFolder + ": " + MID$(made, 8) + ". Preview the New Program lets you choose another folder."
    EXIT SUB
  END IF
  AIOpenMade made, errs
END SUB

' (the program made: opened, said, its banner shown)
SUB AIOpenMade(Made AS STRING, Errs AS INTEGER)
  DIM langErrs AS INTEGER
  AIReviewBar.Visible = 0
  AICreatedFile = Made
  ' (the chat that made it goes with it: the new project's)
  AIRehoming = 1
  OpenProject Made
  AIRehoming = 0
  AIChat.Text = AI.Transcript
  ' (the assistant's own check and Run's can differ: the larger is said)
  langErrs = ProgramErrors
  IF langErrs > Errs THEN Errs = langErrs
  AIShowBanner Errs
END SUB

' The banner: what the assistant made and what to do with it, Run it
' first (the keys go there, Enter runs); with errors left, Run anyway and
' Ask it to fix.
SUB AIShowBanner(Errs AS INTEGER)
  DIM title AS STRING, errText AS STRING
  title = AITools.CreatedTitle
  AIBannerFolder = AITools.CreatedFolder
  IF Errs = 1 THEN errText = "1 error" ELSE errText = LTRIM$(STR$(Errs)) + " errors"
  IF Errs = 0 THEN
    AIStatus.SimpleText = "Made " + title + ": Run It is at the top."
  ELSE
    AIStatus.SimpleText = "Made " + title + ", " + errText + " left: see the top."
  END IF
  AISetBanner Errs
  ' (the keys to Run it, after the editor's own focus on opening)
  AIBannerFocusTurns = 2
  AIBannerRun.SetFocus
END SUB

' The banner's words for a program with Errs errors: made for you, or still
' with errors (Run anyway, Ask it to fix). Nothing else moves: not the keys,
' not the status.
SUB AISetBanner(Errs AS INTEGER)
  DIM title AS STRING, errText AS STRING
  title = AITools.CreatedTitle
  AIBannerErrors = Errs
  IF Errs = 1 THEN errText = "1 error" ELSE errText = LTRIM$(STR$(Errs)) + " errors"
  IF Errs = 0 THEN
    AIBannerText.Caption = "The assistant made this program for you."
    AIBannerRun.Caption = "▶  Run It"
    AIBannerRun.AccessibleName = "Run it"
  ELSE
    AIBannerText.Caption = "The assistant made this program — it still has " + errText + "."
    AIBannerRun.Caption = "▶  Run Anyway"
    AIBannerRun.AccessibleName = "Run anyway"
  END IF
  AIBannerText.AccessibleName = AIBannerText.Caption
  AIBannerUndo.Caption = "↺  Undo"
  AIBannerFix.Visible = Flag(Errs > 0)
  AIBanner.AccessibleDescription = title + ": " + AIBannerText.Caption
  AIBanner.Visible = 1
  AIBannerRun.Default = 1
  AIBannerFonts
END SUB

' After a turn that made or changed files: the program's errors as Run
' counts them (the language service's — it sees what the assistant's own
' check doesn't: Robert's walk, "Member CLEAR not part of class
' DISPLAYCANVAS" under a banner that said "made for you"). The banner, if
' it is about this program, says what is true; the status says so too.
SUB AIRecheck
  DIM errs AS INTEGER, errText AS STRING, said AS STRING
  IF ProjectOpen = 0 THEN EXIT SUB
  errs = ProgramErrors
  IF AIBanner.Visible AND LCASE$(Proj.Folder) = LCASE$(AIBannerFolder) THEN
    IF errs <> AIBannerErrors THEN AISetBanner errs
  END IF
  IF errs = 1 THEN errText = "1 error" ELSE errText = LTRIM$(STR$(errs)) + " errors"
  said = AIStatus.SimpleText
  IF errs > 0 AND INSTR(said, "error") = 0 THEN
    AIStatus.SimpleText = said + " The program has " + errText + ": Problems lists them."
  END IF
END SUB

SUB AIHideBanner
  AIBanner.Visible = 0
  AIBannerRun.Default = 0
  AIBannerFocusTurns = 0
END SUB

' (editor.inc's focus timer: the banner's Run it takes the keys while a
' new program opens — True when it did)
FUNCTION AIBannerTakesFocus AS INTEGER
  Result = 0
  IF AIBannerFocusTurns <= 0 OR AIBanner.Visible = 0 THEN EXIT FUNCTION
  AIBannerFocusTurns = AIBannerFocusTurns - 1
  AIBannerRun.SetFocus
  Result = 1
END FUNCTION

' (another project opened, or none: the banner was about the new program;
' the changes proposed to the project before wait no more — never applied
' to another project's files)
SUB AIProjectChanged
  ' (R1: the project's own chats — its newest shown; aichats.inc)
  AILoadProjectChat
  IF AITools.ProposalCount > 0 AND AIProjectProposal = 0 THEN
    IF ProjectOpen = 0 OR LCASE$(Proj.Folder) <> LCASE$(AITools.ProjectFolder) THEN
      AITools.ClearProposals
      AIReviewBar.Visible = 0
    END IF
  END IF
  IF AIBanner.Visible = 0 THEN EXIT SUB
  IF ProjectOpen = 0 OR LCASE$(Proj.Folder) <> LCASE$(AIBannerFolder) THEN AIHideBanner
END SUB

SUB LayOutAIBanner
  DIM w AS INTEGER, h AS INTEGER, bh AS INTEGER, x AS INTEGER, y0 AS INTEGER, pad AS INTEGER
  IF ThemeName = "classic" THEN
    bh = 25: h = 44
  ELSE
    bh = 34: h = 56
  END IF
  IF AIBanner.Height <> h THEN AIBanner.Height = h
  w = AIBanner.ClientWidth
  IF w < 400 THEN w = 400
  pad = 12
  y0 = (h - bh) \ 2
  AIBannerMark.Height = h
  ' (from the right: ×, What did it do?, Undo, Ask it to fix, Run it)
  x = w - pad - bh
  AIBannerClose.Left = x: AIBannerClose.Top = y0: AIBannerClose.Width = bh: AIBannerClose.Height = bh
  x = x - 8 - 150
  AIBannerWhat.Left = x: AIBannerWhat.Top = y0: AIBannerWhat.Width = 150: AIBannerWhat.Height = bh
  x = x - 8 - 100
  AIBannerUndo.Left = x: AIBannerUndo.Top = y0: AIBannerUndo.Width = 100: AIBannerUndo.Height = bh
  IF AIBannerFix.Visible THEN
    x = x - 8 - 132
    AIBannerFix.Left = x: AIBannerFix.Top = y0: AIBannerFix.Width = 132: AIBannerFix.Height = bh
  END IF
  x = x - 8 - 136
  AIBannerRun.Left = x: AIBannerRun.Top = y0: AIBannerRun.Width = 136: AIBannerRun.Height = bh
  ' (one line, across from the buttons' middle)
  AIBannerText.Left = 6 + pad + 4: AIBannerText.Top = (h - 26) \ 2 + 1
  AIBannerText.Width = x - 16 - AIBannerText.Left: AIBannerText.Height = 26
END SUB

' Run it: as F5 — the program runs, Studio's Run.
SUB AIBannerRunClick
  RunCommand "run.start"
END SUB

SUB AIBannerRunIn
  IF AIBanner.Visible THEN AIBannerRun.Default = 1
END SUB

SUB AIBannerRunOut
  AIBannerRun.Default = 0
END SUB

' Ask it to fix: the assistant asked, in the chat, to fix what is left.
SUB AIBannerFixClick
  ShowAIPanel
  AIInput.Text = "Please fix the errors that are left."
  AISend
END SUB

SUB AIBannerUndoClick
  AIUndoProject
END SUB

' What did it do?: the Assistant pane, the chat scrolled to the first card
' of the turn that made the program (its cards are level-5 headings, a blank
' line apart: back from the one that says Created over those just before
' it), and the keyboard on the chat — its links, Open and Undo, are Tab stops.
SUB AIBannerWhatClick
  DIM all AS STRING, i AS INTEGER, n AS INTEGER, at AS INTEGER, line AS STRING, before AS STRING, anchor AS STRING
  IF AIReady = 0 THEN EXIT SUB
  Dock.ShowPane("aipanel")
  Dock.FocusPane("aipanel")
  LayOutAIPanel
  all = AIChat.Headings
  n = TALLY(all, CHR$(10)) + 1
  at = 0
  FOR i = 1 TO n
    IF INSTR(FIELD$(FIELD$(all, CHR$(10), i), CHR$(9), 2), "Created") > 0 THEN at = i
  NEXT
  IF at > 0 THEN
    DO WHILE at > 1
      line = FIELD$(all, CHR$(10), at)
      before = FIELD$(all, CHR$(10), at - 1)
      IF VAL(FIELD$(before, CHR$(9), 1)) <> 5 OR VAL(FIELD$(line, CHR$(9), 4)) - VAL(FIELD$(before, CHR$(9), 4)) > 2 THEN EXIT DO
      at = at - 1
    LOOP
    anchor = FIELD$(FIELD$(all, CHR$(10), at), CHR$(9), 3)
  END IF
  IF anchor <> "" THEN AIChat.ScrollTo "#" + anchor
  AIChat.SetFocus
END SUB

SUB AIBannerCloseClick
  AIHideBanner
END SUB

' (a recent project gone: off the list)
SUB AIForgetRecent(Path AS STRING)
  ForgetRecent Path
END SUB

' Undo (the banner's, a card's): the new program closed and the files
' Studio wrote for it removed — only those, the explicit list Studio
' kept, each inside its folder; its folder too when nothing else is in it.
' Changed since (a file edited or added, changes not saved): asked first.
' The project open before comes back.
SUB AIUndoProject
  DIM folder AS STRING, said AS STRING, files AS STRING, i AS INTEGER, f AS STRING, d AS INTEGER, gone AS INTEGER, title AS STRING
  folder = AITools.CreatedFolder
  title = AITools.CreatedTitle
  IF folder = "" THEN
    AIStatus.SimpleText = "There is no new program to undo."
    AIHideBanner
    EXIT SUB
  END IF
  said = AITools.UndoCheck()
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) AND LCASE$(LEFT$(DocFile(d), LEN(folder) + 1)) = LCASE$(folder + "/") THEN
      IF said <> "" THEN said = said + "; "
      said = said + FileTitle(DocFile(d)) + " has changes not saved"
    END IF
  NEXT
  IF said <> "" THEN
    IF AskUndoAnyway("Undo the new program " + title + "?", "It changed since the assistant made it: " + said + ". Undo removes the files the assistant wrote, with your changes to them; files you added stay.") = 0 THEN EXIT SUB
  END IF
  ' (its documents go without another question: Undo was the answer; the
  ' chat stays, and goes back with the project open before)
  AIRehoming = 1
  IF ProjectOpen AND LCASE$(Proj.Folder) = LCASE$(folder) THEN
    FOR d = 0 TO DocCount - 1
      IF DocOpen(d) THEN DocDirty(d) = 0
    NEXT
    UpdateModified
    CloseProject
  END IF
  files = AITools.UndoFiles()
  FOR i = 1 TO TALLY(files, CHR$(10)) + 1
    f = FIELD$(files, CHR$(10), i)
    IF f <> "" THEN
      IF FILEEXISTS(f) THEN KILL f
    END IF
  NEXT
  gone = AITools.UndoDone()
  AIForgetRecent AICreatedFile
  AICreatedFile = ""
  AIHideBanner
  AIChat.Text = AI.Transcript
  IF AIPrevProject <> "" THEN
    IF FILEEXISTS(AIPrevProject) THEN OpenProject AIPrevProject
  END IF
  AIRehoming = 0
  AIPrevProject = ""
  IF gone THEN
    AIStatus.SimpleText = "Undone: " + title + " was removed."
  ELSE
    AIStatus.SimpleText = "Undone: the assistant's files of " + title + " were removed; the files you added stay in " + folder + "."
  END IF
  OutputLine "AI: " + AIStatus.SimpleText
END SUB

' (a card's link in the chat: Open, Undo, Review)
SUB AIChatLink(Url AS STRING)
  DIM verb AS STRING, which AS STRING
  verb = LCASE$(FIELD$(Url, "/", 1))
  which = MID$(Url, LEN(verb) + 2)
  SELECT CASE verb
    ' (R2, R5: RAI's own links — Edit a question, Retry, the thinking)
    CASE "rai:edit": AIEditQuestion VAL(which)
    CASE "rai:retry": AIRetry
    CASE "rai:thought"
      ' (the view no longer follows the end — it does again at the next
      ' question —: the link goes to the top, the thought opening below it)
      IF AI.Busy = 0 THEN AIChat.FollowEnd = 0
      AI.ToggleThought VAL(which)
      AIChat.Text = AI.Transcript
      ' (the link at the top, as much of the thought under it as fits)
      IF AI.Busy = 0 THEN AIChat.ScrollTo Url
    CASE "ai:review"
      IF AITools.ProposalCount > 0 THEN
        AIReviewClick
      ELSE
        AIStatus.SimpleText = "Nothing waits for review: those changes were applied or rejected already."
      END IF
    CASE "ai:open", "ai:undo"
      IF AICreatedFile = "" OR (which <> "" AND LCASE$(RIGHT$(AITools.CreatedFolder, LEN(which) + 1)) <> LCASE$("/" + which)) THEN
        AIStatus.SimpleText = "That program isn't the assistant's last one any more (undone, or another came after): open it with File > Open."
      ELSEIF verb = "ai:open" THEN
        IF LCASE$(Proj.Folder) <> LCASE$(AITools.CreatedFolder) OR ProjectOpen = 0 THEN OpenProject AICreatedFile
      ELSE
        AIUndoProject
      END IF
  END SELECT
END SUB

SUB AINewCancel
  AINew.Close
END SUB

' ---- commands and test steps ---------------------------------------------------------

' The AI commands (the command table's "ai.*"); True when Id was one.
FUNCTION AICommand(Id AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Id
    CASE "ai.assistant": ShowAIPanel
    CASE "ai.newChat": IF AIOn THEN AINewChat
    CASE "ai.review"
      IF AITools.ProposalCount > 0 THEN
        AIReviewClick
      ELSE
        AIStatus.SimpleText = "Nothing waits for review: those changes were applied or rejected already."
      END IF
    CASE "ai.whatWasSent": ShowWhatWasSent
    CASE "ai.settings": ShowSettingsPage 1
    CASE "ai.setup": ShowAISetup
    CASE "ai.chats"
      IF AIReady THEN
        ShowAIPanel
        AIShowChats 1
      END IF
    CASE "ai.compact": IF AIReady THEN AICompactClick
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (tests and the manual's scenes: ai:<what>[=<value>] — the settings set as
' the user sets them, a question asked, the proposals applied or the new
' program created without their dialogs; never a key: the tests give one
' in the provider's environment variable, or use a local server)
SUB AIStep(Stp AS STRING)
  DIM k AS INTEGER, verb AS STRING, arg AS STRING
  k = INSTR(Stp, "=")
  IF k > 0 THEN
    verb = LEFT$(Stp, k - 1)
    arg = MID$(Stp, k + 1)
  ELSE
    verb = Stp
  END IF
  ' (the chats' steps — aichats.inc —, Set up RAI's — aisetup.inc)
  IF AIChatStep(verb, arg) THEN EXIT SUB
  IF AISetupStep(verb, arg) THEN EXIT SUB
  SELECT CASE verb
    CASE "effort": AISetEffort arg
    CASE "quietafter": AIQuietAfter = VAL(arg)
    CASE "compact": AICompactClick
    CASE "refresh": AIRefreshClick
    CASE "slash"
      ' (a / command typed in the box, as the user types it)
      AIInput.Text = arg
      AIShowSlash
    CASE "on": SetAIOn 1
    CASE "ready"
      ' (the AI's commands and pane, once ready)
      AIShowOrHide
    CASE "off": SetAIOn 0
    CASE "provider"
      AIProvider = arg
      AIApplySettings
    CASE "base"
      AIBase = arg
      AIApplySettings
    CASE "model"
      AISetModelOf AIProvider, arg
      AIApplySettings
      AIShowOrHide
    CASE "workspace"
      AIWorkspace = arg
      AIApplySettings
    CASE "permission"
      AIPermission = arg
      AIApplySettings
    CASE "reviewfirst"
      ' (Tools > Settings > AI's "Ask before the assistant creates or
      ' changes files")
      AIReviewFirst = Flag(VAL(arg) <> 0)
      AIApplySettings
    CASE "key"
      ' (tests and the manual's pictures only: a made-up key, never a real
      ' one — kept in memory, never in the keychain: under the tests' memory
      ' store or a browser tab's, kept as Save Key keeps it, so Settings
      ' shows it as a user's; elsewhere only the assistant's for the session)
      AI.Provider = AIProvider
      AI.APIKeySource = AIKeySourceOf(AIProvider)
      ' (where a key would be kept, asked before one is set: the store's
      ' own name, never "the macOS Keychain" for this)
      k = Flag(INSTR(AI.KeyStore, "memory, for this session") > 0 OR INSTR(AI.KeyStore, "browser tab") > 0)
      AI.APIKey = arg
      IF k THEN
        IF AI.RememberKey(0) THEN AI.APIKey = ""
      END IF
      AIApplySettings
      AIShowOrHide
    CASE "fetch": AI.RefreshModels
    CASE "prices"
      AIPriceIn = VAL(FIELD$(arg, "/", 1))
      AIPriceOut = VAL(FIELD$(arg, "/", 2))
      AIApplySettings
    CASE "ask"
      AIInput.Text = arg
      AISend
    CASE "wait"
      ' (the next steps wait for the answer: a real provider takes its time)
      IF AI.Busy THEN DoList = "ai:wait," + DoList
    CASE "designwait"
      ' (the next steps wait for the designer's RAI's model list)
      IF DesignAI.Busy THEN DoList = "ai:designwait," + DoList
    CASE "type": AIInput.Text = arg
    CASE "summaryat"
      ' (tests: the conversation summarised sooner)
      AI.SummaryAt = VAL(arg)
    CASE "stop": AIStopClick
    CASE "new": AINewChat
    CASE "review": AIReviewClick
    CASE "apply"
      ShowAIReviewQuietly
      AIRevAcceptAll
      AIApplyProposals
    CASE "create"
      AINewFolder.Text = arg
      AINewCreate
    CASE "undo": AIUndoProject
    CASE "touch"
      ' (a file of the user's in the new program's folder: Undo asks then)
      IF AITools.CreatedFolder <> "" THEN
        AITouch.Open(AITools.CreatedFolder + "/" + arg, 65535)
        AITouch.WriteStr("mine", 4)
        AITouch.Close
      END IF
    CASE "settings": ShowSettingsPage 1
    CASE "sent": ShowWhatWasSent
    CASE ELSE: OutputLine "(ai:" + Stp + ": not a step)"
  END SELECT
END SUB

' (the review's state made without showing it: ai:apply)
SUB ShowAIReviewQuietly
  DIM i AS INTEGER
  FOR i = 0 TO AITools.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AITools.ProposalOld(i)
      AIRevSeen(i) = 0
    END IF
  NEXT
  AIRevAt = -1
END SUB

' ---- an RAI on a form, live in the designer ------------------------------------------
' Its Provider is offered from a list; once it has a key, its Model's list
' is the provider's own, fetched in the background (DesignAI, an RAI of
' Studio's) — when the key is typed, when the provider or the key's source
' change, when the Model row is selected; "↻ Refresh the list" fetches it
' again. A key typed in the APIKey row is kept in the developer's keychain
' under a name; the code says only where it is (APIKeySource =
' "keychain:NAME|ask"): the key itself is never written into the program.

' (the RAI, provider and base URL the list was fetched for; the list and
' its note, given again when that RAI is shown again)
DIM DesignAIFor AS STRING, DesignAIList AS STRING, DesignAINote AS STRING

' (the RAI the inspector shows, its provider, its base URL)
FUNCTION DesignAIKey AS STRING
  Result = Inspector.Target + "|" + DesignAI.Provider + "|" + DesignAI.BaseURL
END FUNCTION

' (the RAI the inspector shows, if it shows one)
FUNCTION DesignAIShown AS INTEGER
  Result = Flag(UCASE$(Inspector.TargetType) = "RAI" AND INSTR(Inspector.Target, ",") = 0 AND Inspector.Target <> "")
END FUNCTION

' (a value as the inspector shows it: its text)
FUNCTION InspectedText(Prop AS STRING) AS STRING
  Result = Unquote(Inspector.Value(Prop))
END FUNCTION

' (a key source's first keychain name)
FUNCTION KeychainNameOf(Source AS STRING) AS STRING
  DIM i AS INTEGER, part AS STRING
  Result = ""
  FOR i = 1 TO TALLY(Source, "|") + 1
    part = LTRIM$(RTRIM$(FIELD$(Source, "|", i)))
    IF LCASE$(LEFT$(part, 9)) = "keychain:" THEN
      Result = MID$(part, 10)
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

' The designed RAI's provider, base URL and key source into DesignAI.
SUB DesignAITake
  DesignAI.Provider = InspectedText("Provider")
  DesignAI.BaseURL = InspectedText("BaseURL")
  DesignAI.WorkspaceID = InspectedText("WorkspaceID")
  DesignAI.APIKeySource = InspectedText("APIKeySource")
END SUB

' Fetches the designed RAI's models (when it has a provider and a key).
SUB DesignAIModels
  IF DesignAIShown = 0 THEN EXIT SUB
  DesignAITake
  IF DesignAI.Provider = "" THEN
    Inspector.SetNote "Model", "Choose a Provider first: its models are listed here then."
    EXIT SUB
  END IF
  IF DesignAI.NeedsKey AND DesignAI.HasKey = 0 THEN
    Inspector.SetNote "Model", "Type the API key in APIKey (kept in your keychain, never in the code): the provider's models are listed here then."
    EXIT SUB
  END IF
  DesignAIFor = DesignAIKey
  DesignAIList = ""
  Inspector.SetNote "Model", "Loading " + DesignAIProviderName + "'s models…"
  IF DesignAI.RefreshModels = 0 THEN Inspector.SetNote "Model", DesignAI.Error
END SUB

SUB DesignAIModelsIn(Count AS INTEGER)
  DIM i AS INTEGER, list AS STRING, fast AS STRING, line AS STRING
  ' (another RAI shown meanwhile: not its list)
  IF DesignAIShown = 0 OR FIELD$(DesignAIFor, "|", 1) <> Inspector.Target THEN EXIT SUB
  list = "↻ Refresh the list"
  FOR i = 1 TO TALLY(DesignAI.Models, CHR$(10)) + 1
    line = FIELD$(DesignAI.Models, CHR$(10), i)
    IF line <> "" THEN
      list = list + CHR$(10) + FIELD$(line, CHR$(9), 1)
      IF fast = "" AND FIELD$(line, CHR$(9), 3) = "fast · low cost" THEN fast = FIELD$(line, CHR$(9), 1)
    END IF
  NEXT
  IF fast <> "" THEN
    DesignAINote = LTRIM$(STR$(Count)) + " models from " + DesignAIProviderName + ", fast, low-cost ones first: pick one."
  ELSE
    DesignAINote = LTRIM$(STR$(Count)) + " models from " + DesignAIProviderName + ": pick one."
  END IF
  DesignAIList = list
  Inspector.SetChoices "Model", DesignAIList
  Inspector.SetNote "Model", DesignAINote
END SUB

' (the designed RAI's provider, by its name)
FUNCTION DesignAIProviderName AS STRING
  DIM i AS INTEGER, line AS STRING
  Result = DesignAI.Provider
  FOR i = 1 TO TALLY(DesignAI.Providers, CHR$(10)) + 1
    line = FIELD$(DesignAI.Providers, CHR$(10), i)
    IF LCASE$(FIELD$(line, CHR$(9), 1)) = LCASE$(DesignAI.Provider) THEN Result = FIELD$(line, CHR$(9), 2)
  NEXT
END FUNCTION

SUB DesignAIError(Message AS STRING)
  IF DesignAIShown THEN Inspector.SetNote "Model", Message
END SUB

' The inspector changed one of the RAI's rows; True when it was handled
' here (the key: never shown, never in the code).
FUNCTION DesignAIChanged(Prop AS STRING, Value AS STRING) AS INTEGER
  DIM p AS STRING, src AS STRING, nm AS STRING, kept AS INTEGER
  Result = 0
  IF DesignAIShown = 0 THEN EXIT FUNCTION
  p = LCASE$(Prop)
  IF p = "apikey" THEN
    Result = 1
    src = InspectedText("APIKeySource")
    nm = KeychainNameOf(src)
    IF nm = "" THEN nm = "RapidR/" + ProjectName + "/" + Inspector.Target
    DesignAITake
    DesignAI.APIKeySource = "keychain:" + nm
    DesignAI.APIKey = Value
    kept = DesignAI.RememberKey(0)
    DesignAI.APIKey = ""
    IF kept THEN
      SetState RunState, "The key is kept in " + DesignAI.KeyStore + " as " + CHR$(34) + nm + CHR$(34) + ": the program names it, never the key."
      IF KeychainNameOf(src) = "" THEN
        ' (the code names where the key is — and asks the program's user
        ' wherever the keychain doesn't have it); that change lists the
        ' models
        Inspector.SetValue "APIKeySource", "keychain:" + nm + "|ask"
        EXIT FUNCTION
      END IF
    ELSE
      SetState "error", DesignAI.Error
    END IF
    DesignAIKeyNote
    DesignAIModels
  ELSEIF p = "provider" OR p = "baseurl" OR p = "apikeysource" OR p = "workspaceid" THEN
    Inspector.SetChoices "Model", ""
    DesignAIKeyNote
    DesignAIModels
  END IF
END FUNCTION

' (the APIKey row says where the key is kept, never the key)
SUB DesignAIKeyNote
  DIM nm AS STRING
  DesignAITake
  nm = KeychainNameOf(DesignAI.APIKeySource)
  IF DesignAI.HasKey AND nm <> "" THEN
    Inspector.SetNote "APIKey", "A key (" + DesignAI.KeyHint + ") is kept in " + DesignAI.KeyStore + " as " + CHR$(34) + nm + CHR$(34) + ": the code names it, never the key. Type another to replace it."
  ELSEIF DesignAI.HasKey THEN
    Inspector.SetNote "APIKey", "A key (" + DesignAI.KeyHint + ") is found in the " + DesignAI.KeySource + ": the code never holds it."
  ELSE
    Inspector.SetNote "APIKey", ""
  END IF
END SUB

' (↻ Refresh the list, picked in the Model row)
SUB InspectorEditorRequest(Prop AS STRING)
  IF DesignAIShown AND LCASE$(Prop) = "model" THEN DesignAIModels
END SUB

' (the Model row selected: its list, fetched once per RAI and provider;
' the APIKey row: where its key is)
SUB InspectorRowSelected(Prop AS STRING)
  IF DesignAIShown = 0 THEN EXIT SUB
  SELECT CASE LCASE$(Prop)
    CASE "model"
      DesignAITake
      IF DesignAIFor <> DesignAIKey THEN
        DesignAIModels
      ELSEIF DesignAIList <> "" THEN
        Inspector.SetChoices "Model", DesignAIList
        Inspector.SetNote "Model", DesignAINote
      END IF
    CASE "apikey": DesignAIKeyNote
  END SELECT
END SUB
