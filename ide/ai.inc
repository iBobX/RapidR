' ai.inc: the AI assistant (docs/ide-ai.md §5, docs/manual/ai-assistant.md).
'
' RAIAssistant (rapidr-studio's AI, DIM'd in decl.inc) does the work: the
' providers (Anthropic, OpenAI, DeepSeek, any OpenAI-compatible server),
' the key in the OS keychain, the chat streamed, the tools on the project
' (files, forms, the registry, the compiler), the changes it proposes, the
' local audit log. Here: its panel, docked with Properties (View > AI
' Assistant, Ctrl+I); the review of what it proposes (RDiffView: each file
' applied as one undo step in its editor); a new program's preview before
' it is created; "What was sent". Its settings: Tools > Settings > AI
' (settings.inc).
'
' Off until the user turns it on: no AI pane, no AI button, nothing sent.
' Then nothing is sent until Send; the panel says what Send shares.

DECLARE SUB AIUpdate
DECLARE SUB AIDone(Status AS STRING)
DECLARE SUB AIModelsIn(Count AS INTEGER)
DECLARE SUB AIErrorSaid(Message AS STRING)
DECLARE SUB AISend
DECLARE SUB AIStopClick
DECLARE SUB AINewChat
DECLARE SUB AIInputKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIProviderPicked
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
DIM AIReviewText AS RLabel
DIM AIReviewBtn AS RButton
DIM AIBottom AS RPanel
DIM AIInput AS RMemo
DIM AISendBtn AS RButton
DIM AIStopBtn AS RButton
DIM AINote AS RLabel
DIM AISentLink AS RLabel
DIM AIStatus AS RStatusBar

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
  CREATE AIRevList AS RListBox
    Align = 3
    Width = 260
    AccessibleName = "Files the assistant changes"
    OnClick = AIRevPicked
  END CREATE
  CREATE AIRevHead AS RLabel
    Align = 1
    Height = 44
    WordWrap = 1: AutoSize = 0
    AccessibleName = "What the change does"
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
    Width = 200
    AccessibleName = "The new program's files"
    OnClick = AINewPicked
  END CREATE
  CREATE AINewForm AS RDesignSurface
    Align = 4
    Width = 440
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
    Height = 52
    BevelOuter = 0
    CREATE AISentWhere AS RLabel
      Left = 16: Top = 17: Width = 680
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
  AI.OnModels = AIModelsIn
  AI.OnError = AIErrorSaid
  ' (the providers, as the component lists them: one list for the panel
  ' and the settings)
  all = AI.Providers
  AIProvCount = 0
  FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    line = FIELD$(all, CHR$(10), i)
    IF line <> "" AND AIProvCount < 8 THEN
      AIProvIds(AIProvCount) = FIELD$(line, CHR$(9), 1)
      AIProvNames(AIProvCount) = FIELD$(line, CHR$(9), 2)
      AIProviderBox.AddItems FIELD$(line, CHR$(9), 3)
      AIProvCount = AIProvCount + 1
    END IF
  NEXT

  AIPanel.Parent = Studio
  AIPanel.Width = 360: AIPanel.Height = 520
  AIPanel.BevelOuter = 0
  AIPanel.AccessibleName = "AI Assistant"

  ' (the top strip: who answers — switchable here — and a new chat)
  AITop.Parent = AIPanel
  AITop.Align = 1
  AITop.Height = 70
  AITop.BevelOuter = 0
  AIProviderBox.Parent = AITop
  AIProviderBox.Style = 2
  AIProviderBox.AccessibleName = "AI provider"
  AIProviderBox.OnChange = AIProviderPicked
  AIModelBox.Parent = AITop
  AIModelBox.AccessibleName = "Model"
  AIModelBox.OnChange = AIModelPicked
  AINewBtn.Parent = AITop
  AINewBtn.Caption = "New Chat"
  AINewBtn.AccessibleName = "Start a new chat"
  AINewBtn.OnClick = AINewChat

  ' (the bottom: the status sentence a screen reader announces, the box
  ' to ask in, Send, Stop, and what Send shares)
  AIStatus.Parent = AIPanel
  AIStatus.SimplePanel = 1
  AIStatus.AccessibleName = "Assistant status"
  AIBottom.Parent = AIPanel
  AIBottom.Align = 2
  AIBottom.Height = 184
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
  AISentLink.Parent = AIBottom
  AISentLink.Caption = "What was sent"
  AISentLink.Cursor = -21
  AISentLink.AccessibleName = "What was sent to the assistant"
  AISentLink.OnClick = AISentLinkClick

  ' (the proposals' bar: shown when the answer proposes changes)
  AIReviewBar.Parent = AIPanel
  AIReviewBar.Align = 2
  AIReviewBar.Height = 40
  AIReviewBar.BevelOuter = 0
  AIReviewBar.Visible = 0
  AIReviewText.Parent = AIReviewBar
  AIReviewText.Left = 10: AIReviewText.Top = 12
  AIReviewBtn.Parent = AIReviewBar
  AIReviewBtn.Caption = "Review..."
  AIReviewBtn.AccessibleName = "Review the proposed changes"
  AIReviewBtn.OnClick = AIReviewClick

  ' (the conversation)
  AIChat.Parent = AIPanel
  AIChat.Align = 5
  AIChat.FollowEnd = 1
  AIChat.AccessibleName = "Conversation with the assistant"

  AIPanel.OnResize = LayOutAIPanel
  ' (Tools > Settings: its General and AI pages)
  SettingsTabs.AddTabs "General", "AI"
  Dock.AddPane(AIPanel, "Assistant", "tab:inspector", "ai")
  AIApplySettings
  LayOutAIPanel
  IF AIOn = 0 THEN Dock.HidePane("aipanel")
END SUB

' The panel's controls placed for its width (the docks resize it).
SUB LayOutAIPanel
  DIM w AS INTEGER, bh AS INTEGER, pw AS INTEGER
  w = AIPanel.ClientWidth
  IF w < 200 THEN w = 200
  IF ThemeName = "classic" THEN bh = 23 ELSE bh = 28
  ' (who answers above, the model below: the docks keep the panel narrow)
  AIProviderBox.Left = 6: AIProviderBox.Top = 5: AIProviderBox.Width = w - 98
  AINewBtn.Left = w - 86: AINewBtn.Top = 4: AINewBtn.Width = 80: AINewBtn.Height = bh
  AIModelBox.Left = 6: AIModelBox.Top = 38: AIModelBox.Width = w - 12
  AIInput.Left = 6: AIInput.Top = 6: AIInput.Width = w - 12: AIInput.Height = 64
  AISendBtn.Left = w - 160: AISendBtn.Top = 76: AISendBtn.Width = 74: AISendBtn.Height = bh
  AIStopBtn.Left = w - 80: AIStopBtn.Top = 76: AIStopBtn.Width = 74: AIStopBtn.Height = bh
  AISentLink.Left = 8: AISentLink.Top = 76 + (bh - 16) \ 2: AISentLink.Width = w - 176
  AINote.Left = 8: AINote.Top = 82 + bh: AINote.Width = w - 16: AINote.Height = AIBottom.Height - 86 - bh
  AIReviewText.Width = w - 120
  AIReviewBtn.Left = w - 104: AIReviewBtn.Top = 6: AIReviewBtn.Width = 96: AIReviewBtn.Height = bh
  AIChat.EmptyText = AIEmptyText
END SUB

SUB AIPanelFonts
  AIChat.Font = UIFont
  AIInput.Font = UIFont
  AINote.Font = DimFont
  AINote.Font.Color = TC("page.dim")
  AISentLink.Font = DimFont
  AISentLink.Font.Color = TC("page.link")
  AIStatus.Font = StatusFont
  AIReviewText.Font = UIFont
  AIReviewText.Font.Bold = 1
  AIRevHead.Font = UIFont
  AINewTitle.Font = SubFont
  AINewWhat.Font = UIFont
  AINewCode.Font = MonoFont
  AISentText.Font = MonoFont
END SUB

' What the empty conversation says: how to start, or what is missing.
FUNCTION AIEmptyText AS STRING
  IF AI.Provider = "" THEN
    Result = "The AI assistant needs a provider: Tools > Settings > AI (a local server such as Ollama needs no key)."
  ELSEIF AI.NeedsKey AND AI.HasKey = 0 THEN
    Result = "Add your API key in Tools > Settings > AI, then ask about this project or for a new program."
  ELSE
    Result = "Ask about this project, or for a new program: " + CHR$(34) + "Add a Clear button" + CHR$(34) + ", " + CHR$(34) + "Build me a tip calculator" + CHR$(34) + ". The assistant proposes changes; you decide."
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
  AI.BaseURL = AIBase
  AI.WorkspaceID = AIWorkspace
  AI.MaxTokens = AIMaxTokens
  AI.Permission = AIPermission
  AI.Model = AIModelOf(AIProvider)
  AIFilling = 1
  AIProviderBox.ItemIndex = -1
  FOR k = 0 TO AIProvCount - 1
    IF AIProvIds(k) = AIProvider THEN AIProviderBox.ItemIndex = k
  NEXT
  AIFillModels
  AIFilling = 0
  AINote.Caption = "Send shares your message, this chat and what the assistant reads of this project with " + AIProviderName + ": nothing else, nothing before."
  AIChat.EmptyText = AIEmptyText
  AIEnableSend
END SUB

' The model list: the ones fetched, else the one chosen.
SUB AIFillModels
  DIM all AS STRING, i AS INTEGER, line AS STRING, k AS INTEGER
  AIModelBox.Clear
  all = AI.Models
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
  AISendBtn.Enabled = Flag(AI.Busy = 0 AND AI.Provider <> "" AND AI.Model <> "" AND (AI.NeedsKey = 0 OR AI.HasKey <> 0))
  AIStopBtn.Enabled = Flag(AI.Busy <> 0)
END SUB

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
END SUB

' Turns the assistant on or off: its pane, its tool bar button.
SUB SetAIOn(IsOn AS INTEGER)
  AIOn = Flag(IsOn)
  BuildToolBar
  UpdateCommands
  IF AIOn THEN
    Dock.ShowPane("aipanel")
  ELSE
    AI.Stop
    Dock.HidePane("aipanel")
  END IF
END SUB

' View > AI Assistant (Ctrl+I): the panel, the keyboard in its box; off,
' Tools > Settings > AI.
SUB ShowAIPanel
  IF AIOn = 0 THEN
    ShowSettingsPage 1
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
  AI.ClearFiles
  IF ProjectOpen = 0 THEN
    AI.ProjectFolder = ""
    AI.ProjectFiles = ""
    AI.MainFile = ""
    AI.ProjectName = ""
    EXIT SUB
  END IF
  FOR i = 0 TO Proj.FileCount - 1
    all = all + Proj.File(i) + CHR$(10)
  NEXT
  AI.ProjectFolder = Proj.Folder
  AI.ProjectFiles = all
  AI.MainFile = Proj.MainFile
  AI.ProjectName = Proj.Name
  AI.CompatMode = Proj.CompatMode
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN AI.SetFile DocFile(d), CodeDoc(d).Text
  NEXT
END SUB

SUB AISend
  DIM t AS STRING
  t = AIInput.Text
  IF LTRIM$(RTRIM$(t)) = "" THEN EXIT SUB
  AISyncProject
  AIReviewBar.Visible = 0
  IF AI.Ask(t) THEN
    AIInput.Text = ""
  ELSE
    AIStatus.SimpleText = AI.Error
  END IF
  AIChat.Text = AI.Transcript
  AIEnableSend
END SUB

' (Enter sends; Shift+Enter is a new line)
SUB AIInputKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 AND Shift = 0 THEN
    Key = 0
    AISend
  END IF
END SUB

SUB AIStopClick
  AI.Stop
  AIEnableSend
END SUB

SUB AINewChat
  AI.NewChat
  AIChat.Text = AI.Transcript
  AIReviewBar.Visible = 0
  AIStatus.SimpleText = AI.Status
  AIEnableSend
  AIInput.SetFocus
END SUB

SUB AIUpdate
  AIChat.Text = AI.Transcript
  AIEnableSend
END SUB

SUB AIDone(Status AS STRING)
  DIM n AS INTEGER
  AIChat.Text = AI.Transcript
  ' (one sentence, announced: what happened, and what to do now)
  AIStatus.SimpleText = AI.Status
  AIEnableSend
  n = AI.ProposalCount
  IF Status = "ok" AND n > 0 THEN
    AIReviewText.Caption = AI.Status
    AIReviewBar.Visible = 1
    IF AIProjectProposal THEN AIReviewBtn.Caption = "Preview..." ELSE AIReviewBtn.Caption = "Review..."
  END IF
  IF AI.Usage <> "" THEN AIStatus.Hint = AI.Usage
END SUB

SUB AIErrorSaid(Message AS STRING)
  OutputLine "AI: " + Message
END SUB

SUB AIModelsIn(Count AS INTEGER)
  AIFilling = 1
  ' (none chosen yet: the provider's suggestion)
  IF AI.Model = "" THEN
    AI.Model = AI.SuggestedModel
    AISetModelOf AI.Provider, AI.Model
  END IF
  AIFillModels
  AIFilling = 0
  AIStatus.SimpleText = AI.Status
  AISettingsModelsIn
  AIEnableSend
END SUB

SUB AIProviderPicked
  DIM k AS INTEGER
  IF AIFilling THEN EXIT SUB
  k = AIProviderBox.ItemIndex
  IF k < 0 OR k >= AIProvCount THEN EXIT SUB
  AIProvider = AIProvIds(k)
  AIApplySettings
  SaveSettings
END SUB

SUB AIModelPicked
  IF AIFilling THEN EXIT SUB
  AI.Model = AIModelId(AIModelBox.Text)
  AISetModelOf AI.Provider, AI.Model
  AIEnableSend
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
  AISentWhere.Caption = "Your key is never shown or logged. The audit log of every request and change: " + AI.AuditLocation
  AISentOk.Left = AISent.ClientWidth - 104: AISentOk.Top = 12: AISentOk.Width = 88: AISentOk.Height = 30
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
  FOR i = 0 TO AI.ProposalCount - 1
    IF FIELD$(AI.ProposalInfo(i), CHR$(9), 1) = "project" THEN Result = 1
  NEXT
END FUNCTION

SUB AIReviewClick
  IF AIProjectProposal THEN ShowAINew ELSE ShowAIReview
END SUB

SUB ShowAIReview
  DIM i AS INTEGER, info AS STRING, n AS INTEGER
  n = AI.ProposalCount
  IF n = 0 THEN EXIT SUB
  IF n > AIMAXPROPS THEN n = AIMAXPROPS
  AIRevList.Clear
  FOR i = 0 TO n - 1
    info = AI.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 1) = "create" THEN
      AIRevList.AddItems FIELD$(info, CHR$(9), 2) + " (new)"
    ELSE
      AIRevList.AddItems FIELD$(info, CHR$(9), 2)
    END IF
    AIRevResult(i) = AI.ProposalOld(i)
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
  info = AI.ProposalInfo(i)
  AIRevHead.Caption = "  " + FIELD$(info, CHR$(9), 2) + ": " + FIELD$(info, CHR$(9), 3)
  AIRevDiff.Language = FIELD$(info, CHR$(9), 2)
  AIRevDiff.LeftText = AI.ProposalOld(i)
  AIRevDiff.RightText = AI.ProposalNew(i)
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
  FOR i = 0 TO AI.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AI.ProposalNew(i)
      AIRevSeen(i) = 1
    END IF
  NEXT
  AIRevDiff.AcceptAll
  AIRevSummary
END SUB

SUB AIRevRejectAll
  DIM i AS INTEGER
  FOR i = 0 TO AI.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AI.ProposalOld(i)
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
  FOR i = 0 TO AI.ProposalCount - 1
    IF i >= AIMAXPROPS THEN EXIT FOR
    info = AI.ProposalInfo(i)
    kind = FIELD$(info, CHR$(9), 1)
    path = FIELD$(info, CHR$(9), 2)
    full = ProjectPath(path)
    newText = AIRevResult(i)
    IF newText = AI.ProposalOld(i) THEN
      AI.Decide i, "rejected"
    ELSEIF kind = "create" AND FILEEXISTS(full) = 0 THEN
      f.Open(full, 65535)
      f.WriteStr(newText, LEN(newText))
      f.Close
      IF Proj.AddFile(full) THEN
        IF Proj.Kind = "project" THEN SaveProjectFile
      END IF
      FillProjectTree
      OpenDocument full
      AI.Decide i, "applied"
      done = done + 1
    ELSE
      d = DocOf(full)
      IF d < 0 THEN
        OpenDocument full
        d = DocOf(full)
      END IF
      IF d < 0 THEN
        OutputLine "AI: can't open " + path + " to change it"
      ELSEIF AIMatches(CodeDoc(d).Text, AI.ProposalOld(i)) = 0 THEN
        ' (the user changed it since the assistant read it: never overwritten)
        OutputLine "AI: " + path + " changed since the assistant read it; its change wasn't applied (ask again)"
        AI.Decide i, "rejected"
      ELSE
        patch = AI.Patch(CodeDoc(d).Text, newText)
        IF patch <> "" THEN CodeDoc(d).ApplyPatches patch, 0
        ProjectFileChanged d
        IF newText = AI.ProposalNew(i) THEN AI.Decide i, "applied" ELSE AI.Decide i, "partly"
        done = done + 1
      END IF
    END IF
  NEXT
  AIReviewBar.Visible = 0
  AIChat.Text = AI.Transcript
  IF done = 1 THEN
    AIStatus.SimpleText = "1 file changed: Edit > Undo takes it back."
  ELSE
    AIStatus.SimpleText = LTRIM$(STR$(done)) + " files changed: Edit > Undo in each takes it back."
  END IF
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
  FOR i = 0 TO AI.ProposalCount - 1
    info = AI.ProposalInfo(i)
    IF FIELD$(info, CHR$(9), 1) = "project" THEN
      AINewFiles.AddItems FIELD$(info, CHR$(9), 2)
      IF FIELD$(info, CHR$(9), 5) = "1" THEN mainAt = AINewFiles.ItemCount - 1
      name = FIELD$(info, CHR$(9), 4)
      AINewWhat.Caption = FIELD$(info, CHR$(9), 3)
    END IF
  NEXT
  AINewTitle.Caption = name + ": " + LTRIM$(STR$(AINewFiles.ItemCount)) + " files, previewed: nothing is written until you press Create"
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
  FOR i = 0 TO AI.ProposalCount - 1
    IF FIELD$(AI.ProposalInfo(i), CHR$(9), 1) = "project" THEN
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
  AINewCode.Language = FIELD$(AI.ProposalInfo(i), CHR$(9), 2)
  AINewCode.Text = AI.ProposalNew(i)
  ' (its form, drawn as the program will draw it)
  AINewForm.Source = AI.ProposalNew(i)
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
  DIM main AS STRING
  IF AINewFolder.Text = "" THEN AINewFolder.Text = DefaultFolder
  IF DIREXISTS(AINewFolder.Text) = 0 THEN MKDIR AINewFolder.Text
  main = AI.CreateProject(AINewFolder.Text)
  IF main = "" THEN
    SHOWMESSAGE "Can't create the project: " + AI.Error
    EXIT SUB
  END IF
  AINew.Close
  AIReviewBar.Visible = 0
  AIChat.Text = AI.Transcript
  OpenProject main
  AIStatus.SimpleText = "The new program is open: F5 runs it."
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
    CASE "ai.review": IF AI.ProposalCount > 0 THEN AIReviewClick
    CASE "ai.whatWasSent": ShowWhatWasSent
    CASE "ai.settings": ShowSettingsPage 1
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
  SELECT CASE verb
    CASE "on": SetAIOn 1
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
    CASE "workspace"
      AIWorkspace = arg
      AIApplySettings
    CASE "permission"
      AIPermission = arg
      AIApplySettings
    CASE "key"
      ' (tests and the manual's pictures only: a made-up key, never a real one)
      AI.Provider = AIProvider
      AI.SetKey arg
      AIApplySettings
    CASE "fetch": AI.FetchModels
    CASE "ask"
      AIInput.Text = arg
      AISend
    CASE "wait"
      ' (the next steps wait for the answer: a real provider takes its time)
      IF AI.Busy THEN DoList = "ai:wait," + DoList
    CASE "type": AIInput.Text = arg
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
    CASE "settings": ShowSettingsPage 1
    CASE "sent": ShowWhatWasSent
    CASE ELSE: OutputLine "(ai:" + Stp + ": not a step)"
  END SELECT
END SUB

' (the review's state made without showing it: ai:apply)
SUB ShowAIReviewQuietly
  DIM i AS INTEGER
  FOR i = 0 TO AI.ProposalCount - 1
    IF i < AIMAXPROPS THEN
      AIRevResult(i) = AI.ProposalOld(i)
      AIRevSeen(i) = 0
    END IF
  NEXT
  AIRevAt = -1
END SUB
