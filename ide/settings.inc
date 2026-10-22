' settings.inc: Tools > Settings, Studio's own settings (kept in the
' per-user store: project.inc's LoadSettings / SaveSettings), on two tabs:
'
' General: the include folders — where $INCLUDE looks after the program's
' own folder, in order, as RapidQ's include folder — for everything Studio
' compiles: Open, the language service, Run, Build, Import RapidQ
' (RPROJECT.IncludeFolders; docs/manual/importing-rapidq.md, "Include
' files"). The themes and the views stay where they were: the command
' palette's View commands.
'
' AI: the AI assistant (ai.inc; docs/manual/ai-assistant.md) — off by
' default; its provider, the API key (kept in the OS keychain, a browser
' tab's session on the web: never in these settings), the model (listed by
' the provider: Fetch Models), the longest reply, what it may do, and the
' provider's prices. The tab has an RAI of its own (SetAI): browsing the
' providers, fetching their lists, a key typed and tried change nothing of
' the assistant's until OK — Cancel leaves it exactly as it was. (Save Key
' and Remove act on the keychain at once, as their names say.)

DECLARE SUB SettingsAddFolder
DECLARE SUB SettingsRemoveFolder
DECLARE SUB SettingsPicked
DECLARE SUB SettingsViews
DECLARE SUB SettingsOk
DECLARE SUB SettingsCancel
DECLARE SUB SettingsTabChanged
DECLARE SUB SetIncludeFolders(Folders AS STRING)
DECLARE SUB AISetProviderPicked
DECLARE SUB AISetKeySave
DECLARE SUB AISetKeyRemove
DECLARE SUB AISetFetch
DECLARE SUB AISetLink(Url AS STRING, Sender AS RMarkdownView)
DECLARE SUB AISetPricesClick
DECLARE SUB AISetModelsIn(Count AS INTEGER)
DECLARE SUB AISetError(Message AS STRING)
DECLARE SUB AISetDone(Status AS STRING)
DECLARE SUB AISetKeyNote
DECLARE SUB AISetFillModels(Keep AS STRING)
DECLARE SUB AISetTakeRows
DECLARE SUB AISetNote(Text AS STRING)
DECLARE FUNCTION AISetLinesHigh(Text AS STRING, W AS INTEGER) AS INTEGER
DECLARE SUB SettingsClosed(Action AS INTEGER)
DECLARE SUB SettingsSetupClick
DECLARE SUB SettingsPathRefresh
DECLARE SUB SettingsFillPath

' (the provider shown on the AI tab; its list being filled: its OnChange
' isn't the user's)
DIM AISetProv AS STRING
DIM AISetFilling AS INTEGER
' (the two notes' heights, as their text wraps: a note takes the room it
' needs, an empty one none — LayOutSettingsDialog; measured in the
' dialog's font)
DIM AIKeyNoteH AS INTEGER, AIFetchNoteH AS INTEGER
DIM AISetMeasure AS RBitmap

CREATE SettingsDialog AS RForm
  Caption = "Settings"
  Width = 600: Height = 560
  BorderStyle = 3
  OnClose = SettingsClosed
  CREATE SettingsTabs AS RTabControl
    AccessibleName = "Settings pages"
    OnChange = SettingsTabChanged
  END CREATE
  ' ---- General ----
  CREATE GenPage AS RPanel
    BevelOuter = 0
    AccessibleName = "General"
    CREATE IncFoldersLabel AS RLabel
      Caption = "&Include folders"
    END CREATE
    CREATE IncFoldersNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = "Where $INCLUDE looks after the program's own folder, in this order (as RapidQ's include folder): add the folder that holds your RapidQ includes (WINDOWS.INC, RAPIDQ2.INC, your libraries). RAPIDQ.INC needs none: RapidR has it built in."
    END CREATE
    CREATE IncFolders AS RListBox
      AccessibleName = "Include folders"
      OnClick = SettingsPicked
    END CREATE
    CREATE IncAdd AS RButton
      Caption = "&Add Folder..."
      AccessibleName = "Add an include folder"
      OnClick = SettingsAddFolder
    END CREATE
    CREATE IncRemove AS RButton
      Caption = "&Remove"
      AccessibleName = "Remove the selected include folder"
      OnClick = SettingsRemoveFolder
    END CREATE
    CREATE SettingsViewsBtn AS RButton
      Caption = "Themes and &Views..."
      AccessibleName = "The themes and views, in the command palette"
      OnClick = SettingsViews
    END CREATE
    ' (S2) the start-up's offer to reopen the last project
    CREATE ReopenBox AS RCheckBox
      Caption = "&Offer to reopen the last project and its files at start-up"
      AccessibleDescription = "On: the Welcome page offers the project you had open last, with the files you had open in it; Enter reopens it"
    END CREATE
    ' (E) the code's helpers: what the code editor shows beside the code
    CREATE HelpersLabel AS RLabel
      Caption = "Code &helpers"
    END CREATE
    CREATE HelpersBox AS RComboBox
      Style = 2
      AccessibleName = "Code helpers"
      AccessibleDescription = "What the code editor shows beside your code: colour swatches, pictures, a line's help, the argument names"
    END CREATE
    CREATE HelpersNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = "Swatches for colours, thumbnails for pictures (a red mark when the file is missing), the caret line's help and the argument names. Click a mark, or press Ctrl+. on a value, to change it; hold Ctrl a moment to see every argument's name."
    END CREATE
    ' (S6) the PATH the programs run from here get: your login shell's
    CREATE PathLabel AS RLabel
      Caption = "Programs' &PATH"
    END CREATE
    CREATE PathNote AS RLabel
      WordWrap = 1: AutoSize = 0
    END CREATE
    CREATE PathList AS RListBox
      AccessibleName = "The folders programs run from Studio find commands in, in order"
    END CREATE
    CREATE PathRefresh AS RButton
      Caption = "Re&fresh"
      AccessibleName = "Read your login shell's PATH again"
      OnClick = SettingsPathRefresh
    END CREATE
    ' (S9: the debugging layout — debug.inc)
    CREATE DebugPanesLabel AS RLabel
      Caption = "&Debug panes"
    END CREATE
    CREATE DebugPanesBox AS RComboBox
      Style = 2
      AccessibleName = "Debug panes"
    END CREATE
    CREATE DebugPanesNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = "Automatic: Variables, Watch, Call Stack and Breakpoints appear beside Output while a program runs. View > Debug Panes (Ctrl+Shift+D) shows or hides them."
    END CREATE
  END CREATE
  ' ---- AI ----
  CREATE AIPage AS RPanel
    BevelOuter = 0
    AccessibleName = "AI"
    CREATE AIOnBox AS RCheckBox
      Caption = "&Turn on the AI assistant"
    END CREATE
    ' (R6: the guided way — free on this computer, or a cloud key)
    CREATE AISetupBtn AS RButton
      Caption = "Set &Up RAI..."
      AccessibleName = "Set up RAI, step by step"
      AccessibleDescription = "Free on this computer, or a cloud service with your key: what each costs and sends, in plain words"
      OnClick = SettingsSetupClick
    END CREATE
    CREATE AIOnNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = "Off by default. On, Studio shows the AI Assistant panel (View > AI Assistant, Ctrl+I): ask about your project or for a new program. Nothing is sent until you press Send there. A new program it makes opens in Studio at once (Undo takes it back); changes to your files are shown for you to review, and nothing runs until you press Run."
    END CREATE
    CREATE AIProvLabel AS RLabel
      Caption = "&Provider:"
    END CREATE
    CREATE AIProvBox AS RComboBox
      Style = 2
      AccessibleName = "AI provider"
      OnChange = AISetProviderPicked
    END CREATE
    CREATE AIPricesLink AS RLabel
      Caption = "See prices"
      Cursor = -21
      AccessibleName = "The provider's prices, in the browser"
      OnClick = AISetPricesClick
    END CREATE
    CREATE AIKeyLabel AS RLabel
      Caption = "API &key:"
    END CREATE
    CREATE AIKeyEdit AS REdit
      PasswordChar = "*"
      AccessibleName = "API key"
      AccessibleDescription = "Paste the key the provider gave you; it is kept in your system's keychain, never in a project"
    END CREATE
    CREATE AIKeySaveBtn AS RButton
      Caption = "&Save Key"
      AccessibleName = "Keep the API key"
      OnClick = AISetKeySave
    END CREATE
    CREATE AIKeyRemoveBtn AS RButton
      Caption = "Remo&ve"
      AccessibleName = "Remove the API key"
      OnClick = AISetKeyRemove
    END CREATE
    CREATE AIKeyNote AS RLabel
      WordWrap = 1: AutoSize = 0
    END CREATE
    CREATE AIRemember AS RCheckBox
      Caption = "R&emember on this device"
      AccessibleDescription = "Kept in this browser's storage for the page: anyone using this browser profile could read it"
    END CREATE
    CREATE AIWsLabel AS RLabel
      Caption = "&Workspace ID:"
    END CREATE
    CREATE AIWsEdit AS REdit
      AccessibleName = "Anthropic workspace ID (optional)"
    END CREATE
    CREATE AIBaseLabel AS RLabel
      Caption = "&Base URL:"
    END CREATE
    CREATE AIBaseBox AS RComboBox
      AccessibleName = "The server's base URL"
    END CREATE
    CREATE AIModelLabel AS RLabel
      Caption = "&Model:"
    END CREATE
    CREATE AIModelCombo AS RComboBox
      AccessibleName = "Model"
    END CREATE
    CREATE AIFetchBtn AS RButton
      Caption = "&Fetch Models"
      AccessibleName = "Ask the provider which models your key can use"
      OnClick = AISetFetch
    END CREATE
    ' (what Fetch Models found, or why it couldn't: under the list it fills)
    CREATE AIFetchNote AS RLabel
      WordWrap = 1: AutoSize = 0
      AccessibleName = "Fetch Models' result"
    END CREATE
    CREATE AIMaxLabel AS RLabel
      Caption = "Longest &reply:"
    END CREATE
    CREATE AIMaxEdit AS REdit
      AccessibleName = "Longest reply, in tokens"
    END CREATE
    CREATE AIMaxUnit AS RLabel
      Caption = "tokens (a cost limit for each answer)"
    END CREATE
    CREATE AIPriceLabel AS RLabel
      Caption = "&Prices:"
    END CREATE
    CREATE AIPriceInEdit AS REdit
      AccessibleName = "Price of a million tokens sent, in dollars (optional)"
    END CREATE
    CREATE AIPriceOutEdit AS REdit
      AccessibleName = "Price of a million tokens received, in dollars (optional)"
    END CREATE
    CREATE AIPriceUnit AS RLabel
      Caption = "$ per million tokens sent / received"
    END CREATE
    CREATE AIPermLabel AS RLabel
      Caption = "&It may:"
    END CREATE
    CREATE AIPermBox AS RComboBox
      Style = 2
      AccessibleName = "What the assistant may do"
    END CREATE
    CREATE AIReviewBox AS RCheckBox
      Caption = "Ask before the assistant &creates or changes files"
      AccessibleDescription = "On: a new program is shown to you first and created when you press Create. Off: a new program opens in Studio at once, with Run it and Undo."
    END CREATE
    CREATE AISetLinks AS RMarkdownView
      OpenLinks = 1
      AccessibleName = "The provider's links"
      OnLinkClick = AISetLink
    END CREATE
  END CREATE
  CREATE SettingsOkBtn AS RButton
    Caption = "OK"
    Default = 1
    OnClick = SettingsOk
  END CREATE
  CREATE SettingsNo AS RButton
    Caption = "Cancel"
    Cancel = 1
    OnClick = SettingsCancel
  END CREATE
END CREATE

' (RapidQ's sizes in the classic look, the RAPIDR look's elsewhere — as
' LayOutAppDialog does)
SUB LayOutSettingsDialog
  DIM pad AS INTEGER, w AS INTEGER, bh AS INTEGER, bw AS INTEGER, y AS INTEGER, nh AS INTEGER, lh AS INTEGER, side AS INTEGER
  DIM th AS INTEGER, ph AS INTEGER, lx AS INTEGER, fx AS INTEGER, fw AS INTEGER, row AS INTEGER, fh AS INTEGER, gh AS INTEGER
  IF ThemeName = "classic" THEN
    pad = 16: w = 520: bh = 25: bw = 84: nh = 58: lh = 130: side = 120: th = 24: row = 30: fh = 21
  ELSE
    pad = 20: w = 540: bh = 32: bw = 96: nh = 66: lh = 150: side = 140: th = 30: row = 38: fh = 28
  END IF
  SettingsDialog.Font.Name = UIFont.Name: SettingsDialog.Font.Size = UIFont.Size
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  ' (the pages' height follows the AI tab's rows — a provider's own row,
  ' the notes as long as their text — with room for the links under them:
  ' the dialog grows when Fetch Models has more to say, nothing is cut)
  SettingsTabs.Left = pad: SettingsTabs.Top = pad - 8: SettingsTabs.Width = w: SettingsTabs.Height = th

  ' ---- General ----
  y = pad
  IncFoldersLabel.Left = pad: IncFoldersLabel.Top = y
  IncFoldersLabel.Font.Bold = 1
  y = y + 24
  IncFoldersNote.Left = pad: IncFoldersNote.Top = y: IncFoldersNote.Width = w: IncFoldersNote.Height = nh
  IncFoldersNote.Font.Color = TC("page.dim")
  y = y + nh + 8
  IncFolders.Left = pad: IncFolders.Top = y: IncFolders.Width = w - side - 12: IncFolders.Height = lh
  IF ThemeName = "classic" THEN IncFolders.ItemHeight = 16 ELSE IncFolders.ItemHeight = 22
  IncAdd.Left = pad + w - side: IncAdd.Top = y: IncAdd.Width = side: IncAdd.Height = bh
  IncRemove.Left = pad + w - side: IncRemove.Top = y + bh + 8: IncRemove.Width = side: IncRemove.Height = bh
  y = y + lh + pad
  SettingsViewsBtn.Left = pad: SettingsViewsBtn.Top = y: SettingsViewsBtn.Width = side + 40: SettingsViewsBtn.Height = bh
  y = y + bh + pad
  ReopenBox.Left = pad: ReopenBox.Top = y: ReopenBox.Width = w: ReopenBox.Height = 22
  y = y + 22 + pad
  ' (the code's helpers: the label and its choice on a row, the note under)
  HelpersLabel.Left = pad: HelpersLabel.Top = y + (bh - 18) \ 2
  HelpersLabel.Font.Bold = 1
  HelpersBox.Left = pad + side: HelpersBox.Top = y: HelpersBox.Width = w - side
  y = y + bh + 6
  nh = AISetLinesHigh(HelpersNote.Caption, w)
  HelpersNote.Left = pad: HelpersNote.Top = y: HelpersNote.Width = w: HelpersNote.Height = nh
  HelpersNote.Font.Color = TC("page.dim")
  y = y + nh + pad
  ' (the programs' PATH: where it isn't changed — Windows, the web — not shown)
  IF PathLabel.Visible THEN
    ' (Refresh on the heading's row: the folders get the whole width)
    PathLabel.Left = pad: PathLabel.Top = y + (bh - 18) \ 2
    PathLabel.Font.Bold = 1
    PathRefresh.Left = pad + w - side: PathRefresh.Top = y: PathRefresh.Width = side: PathRefresh.Height = bh
    y = y + bh + 6
    nh = AISetLinesHigh(PathNote.Caption, w)
    PathNote.Left = pad: PathNote.Top = y: PathNote.Width = w: PathNote.Height = nh
    PathNote.Font.Color = TC("page.dim")
    y = y + nh + 8
    ' (a row each, as tall as the dialog's text needs)
    PathList.Left = pad: PathList.Top = y: PathList.Width = w: PathList.Height = 5 * row \ 2 + 30
    IF ThemeName = "classic" THEN PathList.ItemHeight = 16 ELSE PathList.ItemHeight = 22
    y = y + PathList.Height + pad
  END IF
  ' (lined up with the code's helpers above: a label, its choice from the same column)
  DebugPanesLabel.Left = pad: DebugPanesLabel.Top = y + (bh - 18) \ 2: DebugPanesLabel.Width = side - 4
  DebugPanesLabel.Font.Bold = 1
  DebugPanesBox.Left = pad + side: DebugPanesBox.Top = y: DebugPanesBox.Width = 160
  y = y + row
  DebugPanesNote.Left = pad: DebugPanesNote.Top = y: DebugPanesNote.Width = w: DebugPanesNote.Height = 36
  DebugPanesNote.Font.Color = TC("page.dim")
  y = y + 36 + pad
  ' (the General page as tall as what's on it)
  gh = y

  ' ---- AI ----
  lx = pad: fx = pad + 112: fw = w - 112
  y = pad - 4
  AIOnBox.Left = lx: AIOnBox.Top = y: AIOnBox.Width = w - 156: AIOnBox.Height = 22
  AIOnBox.Font.Bold = 1
  AISetupBtn.Left = lx + w - 148: AISetupBtn.Top = y - (bh - 22) \ 2: AISetupBtn.Width = 148: AISetupBtn.Height = bh
  y = y + 26
  AIOnNote.Left = lx: AIOnNote.Top = y: AIOnNote.Width = w: AIOnNote.Height = 68
  AIOnNote.Font.Color = TC("page.dim")
  y = y + 74
  AIProvLabel.Left = lx: AIProvLabel.Top = y + 5: AIProvLabel.Width = 108
  AIProvBox.Left = fx: AIProvBox.Top = y: AIProvBox.Width = fw - 100
  AIPricesLink.Left = fx + fw - 90: AIPricesLink.Top = y + 5: AIPricesLink.Width = 90
  AIPricesLink.Font.Color = TC("page.link")
  y = y + row
  AIKeyLabel.Left = lx: AIKeyLabel.Top = y + 5: AIKeyLabel.Width = 108
  AIKeyEdit.Left = fx: AIKeyEdit.Top = y: AIKeyEdit.Width = fw - 196: AIKeyEdit.Height = fh
  AIKeySaveBtn.Left = fx + fw - 188: AIKeySaveBtn.Top = y - (bh - fh) \ 2: AIKeySaveBtn.Width = 96: AIKeySaveBtn.Height = bh
  AIKeyRemoveBtn.Left = fx + fw - 84: AIKeyRemoveBtn.Top = y - (bh - fh) \ 2: AIKeyRemoveBtn.Width = 84: AIKeyRemoveBtn.Height = bh
  y = y + fh + 4
  AIKeyNote.Left = fx: AIKeyNote.Top = y: AIKeyNote.Width = fw: AIKeyNote.Height = AIKeyNoteH
  AIKeyNote.Font.Color = TC("page.dim")
  ' (the next row 6 under the note; with none, a row's step as everywhere)
  IF AIKeyNoteH > 0 THEN y = y + AIKeyNoteH + 6 ELSE y = y - fh - 4 + row
  ' (the web's Remember on a row of its own; then the rows a provider has:
  ' Anthropic's workspace, a compatible server's URL — in one place, shown
  ' when they apply)
  AIRemember.Left = fx: AIRemember.Top = y: AIRemember.Width = fw: AIRemember.Height = 22
  IF Proj.FileManager = "" THEN y = y + 28
  AIWsLabel.Left = lx: AIWsLabel.Top = y + 5: AIWsLabel.Width = 108
  AIWsEdit.Left = fx: AIWsEdit.Top = y: AIWsEdit.Width = fw: AIWsEdit.Height = fh
  AIBaseLabel.Left = lx: AIBaseLabel.Top = y + 5: AIBaseLabel.Width = 108
  AIBaseBox.Left = fx: AIBaseBox.Top = y: AIBaseBox.Width = fw
  ' (a provider with neither has no row there: the rows close up, the
  ' links at the bottom get the room — AISettingsProvider lays them out
  ' again for each provider)
  IF AIWsLabel.Visible OR AIBaseLabel.Visible THEN y = y + row
  AIModelLabel.Left = lx: AIModelLabel.Top = y + 5: AIModelLabel.Width = 108
  AIModelCombo.Left = fx: AIModelCombo.Top = y: AIModelCombo.Width = fw - 140
  AIFetchBtn.Left = fx + fw - 132: AIFetchBtn.Top = y - (bh - fh) \ 2: AIFetchBtn.Width = 132: AIFetchBtn.Height = bh
  y = y + fh + 4
  ' (what Fetch Models says — a refused key's or a server's explanation
  ' takes a few lines: all of it shows, wrapped; nothing said, no room)
  AIFetchNote.Left = fx: AIFetchNote.Top = y: AIFetchNote.Width = fw: AIFetchNote.Height = AIFetchNoteH
  AIFetchNote.Font.Color = TC("page.dim")
  IF AIFetchNoteH > 0 THEN y = y + AIFetchNoteH + 6 ELSE y = y - fh - 4 + row
  AIMaxLabel.Left = lx: AIMaxLabel.Top = y + 5: AIMaxLabel.Width = 108
  AIMaxEdit.Left = fx: AIMaxEdit.Top = y: AIMaxEdit.Width = 80: AIMaxEdit.Height = fh
  AIMaxUnit.Left = fx + 90: AIMaxUnit.Top = y + 5: AIMaxUnit.Width = fw - 90
  AIMaxUnit.Font.Color = TC("page.dim")
  y = y + row
  AIPriceLabel.Left = lx: AIPriceLabel.Top = y + 5: AIPriceLabel.Width = 108
  AIPriceInEdit.Left = fx: AIPriceInEdit.Top = y: AIPriceInEdit.Width = 60: AIPriceInEdit.Height = fh
  AIPriceOutEdit.Left = fx + 68: AIPriceOutEdit.Top = y: AIPriceOutEdit.Width = 60: AIPriceOutEdit.Height = fh
  AIPriceUnit.Left = fx + 136: AIPriceUnit.Top = y + 5: AIPriceUnit.Width = fw - 136
  AIPriceUnit.Font.Color = TC("page.dim")
  y = y + row
  AIPermLabel.Left = lx: AIPermLabel.Top = y + 5: AIPermLabel.Width = 108
  AIPermBox.Left = fx: AIPermBox.Top = y: AIPermBox.Width = fw
  y = y + row
  AIReviewBox.Left = fx: AIReviewBox.Top = y: AIReviewBox.Width = fw: AIReviewBox.Height = 22
  y = y + 28 + 4
  ' (the links and what Send shares: up to five lines and their margins (high contrast's face is wider))
  ph = y + 124
  IF ph < 420 THEN ph = 420
  IF ph < gh THEN ph = gh
  AISetLinks.Left = lx - 18: AISetLinks.Top = y - 12: AISetLinks.Width = w + 36: AISetLinks.Height = ph - y + 8
  AISetLinks.Font = DimFont
  GenPage.Left = 0: GenPage.Top = pad - 8 + th: GenPage.Width = w + 2 * pad: GenPage.Height = ph
  AIPage.Left = 0: AIPage.Top = pad - 8 + th: AIPage.Width = w + 2 * pad: AIPage.Height = ph

  y = pad - 8 + th + ph + 8
  SettingsNo.Width = bw: SettingsNo.Height = bh: SettingsNo.Left = pad + w - bw: SettingsNo.Top = y
  SettingsOkBtn.Width = bw: SettingsOkBtn.Height = bh: SettingsOkBtn.Left = pad + w - 2 * bw - 8: SettingsOkBtn.Top = y
  SettingsDialog.ClientWidth = w + 2 * pad
  SettingsDialog.ClientHeight = y + bh + pad
END SUB

SUB ShowSettings
  ShowSettingsPage 0
END SUB

' Tools > Settings, on page k (0 General, 1 AI).
SUB ShowSettingsPage(k AS INTEGER)
  DIM n AS INTEGER, i AS INTEGER, f AS STRING, all AS STRING
  ReopenBox.Checked = ReopenLast
  IF HelpersBox.ItemCount = 0 THEN
    HelpersBox.AddItems "Off"
    HelpersBox.AddItems "Quiet: the marks only"
    HelpersBox.AddItems "Helpful: marks, the line's help, argument names"
    HelpersBox.AddItems "Everything: every argument's name, choices marked too"
  END IF
  SELECT CASE EditorHelpers
    CASE "off": HelpersBox.ItemIndex = 0
    CASE "quiet": HelpersBox.ItemIndex = 1
    CASE "everything": HelpersBox.ItemIndex = 3
    CASE ELSE: HelpersBox.ItemIndex = 2
  END SELECT
  SettingsFillPath
  LayOutSettingsDialog
  IncFolders.Clear
  all = Proj.IncludeFolders
  n = TALLY(all, CHR$(10)) + 1
  FOR i = 1 TO n
    f = FIELD$(all, CHR$(10), i)
    IF f <> "" THEN IncFolders.AddItems f
  NEXT
  SettingsPicked
  ' (S9: Debug panes, as it is now)
  DebugPanesBox.Clear
  DebugPanesBox.AddItems "Automatic", "Always", "Never"
  SELECT CASE DebugPanesMode
    CASE "always": DebugPanesBox.ItemIndex = 1
    CASE "never": DebugPanesBox.ItemIndex = 2
    CASE ELSE: DebugPanesBox.ItemIndex = 0
  END SELECT
  AISettingsFill
  SettingsTabs.TabIndex = k
  SettingsTabChanged
  SettingsDialog.Center
  SettingsDialog.ShowModal
END SUB

SUB SettingsTabChanged
  GenPage.Visible = Flag(SettingsTabs.TabIndex = 0)
  AIPage.Visible = Flag(SettingsTabs.TabIndex = 1)
END SUB

SUB SettingsPicked
  IncRemove.Enabled = (IncFolders.ItemIndex >= 0)
END SUB

SUB SettingsAddFolder
  DIM i AS INTEGER, f AS STRING
  OpenDlg.Caption = "Add an Include Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN f = OpenDlg.FileName
  OpenDlg.PickFolder = 0
  IF f = "" THEN EXIT SUB
  ' (once each)
  FOR i = 0 TO IncFolders.ItemCount - 1
    IF LCASE$(IncFolders.Item(i)) = LCASE$(f) THEN EXIT SUB
  NEXT
  IncFolders.AddItems f
  IncFolders.ItemIndex = IncFolders.ItemCount - 1
  SettingsPicked
END SUB

SUB SettingsRemoveFolder
  IF IncFolders.ItemIndex < 0 THEN EXIT SUB
  IncFolders.DelItems IncFolders.ItemIndex
  SettingsPicked
END SUB

SUB SettingsViews
  SettingsDialog.Close
  ShowPalette "View: "
END SUB

SUB SettingsOk
  DIM i AS INTEGER, all AS STRING
  FOR i = 0 TO IncFolders.ItemCount - 1
    IF all <> "" THEN all = all + CHR$(10)
    all = all + IncFolders.Item(i)
  NEXT
  SetIncludeFolders all
  ReopenLast = Flag(ReopenBox.Checked)
  ' (S9: Debug panes)
  SELECT CASE DebugPanesBox.ItemIndex
    CASE 1: SetDebugPanesMode "always"
    CASE 2: SetDebugPanesMode "never"
    CASE ELSE: SetDebugPanesMode "auto"
  END SELECT
  ' (the code's helpers, in every open document at once)
  SELECT CASE HelpersBox.ItemIndex
    CASE 0: EditorHelpers = "off"
    CASE 1: EditorHelpers = "quiet"
    CASE 3: EditorHelpers = "everything"
    CASE ELSE: EditorHelpers = "helpful"
  END SELECT
  FOR i = 0 TO DocCount - 1
    IF DocOpen(i) THEN CodeDoc(i).Helpers = EditorHelpers
  NEXT
  AISettingsTake
  SettingsDialog.Close
  SaveSettings
END SUB

' (S6) The programs' PATH: its folders, where it came from — shown where
' Studio changes it (macOS, Linux), not on Windows or the web.
SUB SettingsFillPath
  DIM p AS STRING, n AS INTEGER, i AS INTEGER, shown AS INTEGER
  p = Session.ShellPath
  shown = Flag(p <> "")
  PathLabel.Visible = shown: PathNote.Visible = shown: PathList.Visible = shown: PathRefresh.Visible = shown
  PathList.Clear
  IF shown = 0 THEN EXIT SUB
  PathNote.Caption = "Programs run from Studio, and their SHELL and RUN commands, find commands in these folders, in this order. " + Session.ShellPathSource
  n = TALLY(p, ":") + 1
  FOR i = 1 TO n
    IF FIELD$(p, ":", i) <> "" THEN PathList.AddItems FIELD$(p, ":", i)
  NEXT
END SUB

' Refresh: your login shell read again (its startup files changed) — the
' next Run gets the new PATH.
SUB SettingsPathRefresh
  Session.RefreshShellPath
  SettingsFillPath
  LayOutSettingsDialog
  OutputLine "Programs' PATH read again. " + Session.ShellPathSource
END SUB

' (nothing of the AI tab reaches the assistant: its own RAI is dropped)
SUB SettingsCancel
  SettingsDialog.Close
END SUB

' (Set Up RAI…: the settings left as they were, the wizard instead)
SUB SettingsSetupClick
  SettingsCancel
  ShowAISetup
END SUB

' However it closes (OK, Cancel, Escape, its close box, Themes and Views):
' a models list still on its way is no longer waited for.
SUB SettingsClosed(Action AS INTEGER)
  SetAI.Cancel
END SUB

' The include folders from now on (a folder a line), said in Output; the
' open program's problems found again with them.
SUB SetIncludeFolders(Folders AS STRING)
  Proj.IncludeFolders = Folders
  IF Folders = "" THEN
    OutputLine "Include folders: none ($INCLUDE looks in the program's folder)"
  ELSE
    OutputLine "Include folders: " + REPLACESUBSTR$(Folders, CHR$(10), ", ")
  END IF
  IF ProjectOpen THEN FillProblemsOf ProjectMain
END SUB

' ---- the AI tab ------------------------------------------------------------------------
' Everything here talks to SetAI, the tab's own RAI (decl.inc), never to the
' assistant: OK hands the choices to it (AISettingsTake → AIApplySettings).

' (how tall Text is, wrapped at spaces to W pixels in the dialog's font as
' a WordWrap label wraps it; 0 for no text)
FUNCTION AISetLinesHigh(Text AS STRING, W AS INTEGER) AS INTEGER
  DIM lines AS INTEGER, cur AS STRING, wd AS STRING, i AS INTEGER, n AS INTEGER, t AS STRING
  Result = 0
  t = LTRIM$(RTRIM$(Text))
  IF t = "" THEN EXIT FUNCTION
  lines = 1
  n = TALLY(t, " ") + 1
  FOR i = 1 TO n
    wd = FIELD$(t, " ", i)
    IF cur = "" THEN
      cur = wd
    ELSEIF AISetMeasure.TextWidth(cur + " " + wd) > W - 2 THEN
      lines = lines + 1
      cur = wd
    ELSE
      cur = cur + " " + wd
    END IF
  NEXT
  Result = lines * AISetMeasure.TextHeight(" ") + 2
END FUNCTION

' What Fetch Models says, under the Model list: the rows below make room
' for it, and close up again when there is nothing to say.
SUB AISetNote(Text AS STRING)
  AIFetchNote.Caption = Text
  AIFetchNoteH = AISetLinesHigh(Text, AIFetchNote.Width)
  LayOutSettingsDialog
END SUB

' The AI tab from the settings.
SUB AISettingsFill
  DIM k AS INTEGER, all AS STRING, i AS INTEGER, line AS STRING
  AIOnBox.Checked = AIOn
  IF AIProvCount = 0 THEN EXIT SUB
  IF AIProvBox.ItemCount = 0 THEN
    FOR k = 0 TO AIProvCount - 1
      AIProvBox.AddItems AIProvNames(k)
    NEXT
    all = SetAI.Presets
    FOR i = 1 TO TALLY(all, CHR$(10)) + 1
      line = FIELD$(all, CHR$(10), i)
      IF line <> "" THEN AIBaseBox.AddItems FIELD$(line, CHR$(9), 2)
    NEXT
    AIPermBox.AddItems "Read the project, propose changes you review", "Only read the project and answer"
  END IF
  ' (a list still coming from the last time the tab was open: not waited
  ' for; a key typed then and not kept: forgotten)
  SetAI.Cancel
  SetAI.APIKey = ""
  AIFetchBtn.Enabled = 1
  AISetNote ""
  AISetProv = AIProvider
  ' (none chosen yet: the first of the list — every provider is a pick away)
  IF AISetProv = "" THEN AISetProv = AIProvIds(0)
  AISetFilling = 1
  FOR k = 0 TO AIProvCount - 1
    IF AIProvIds(k) = AISetProv THEN AIProvBox.ItemIndex = k
  NEXT
  AISetFilling = 0
  IF AIMaxTokens > 0 THEN AIMaxEdit.Text = LTRIM$(STR$(AIMaxTokens)) ELSE AIMaxEdit.Text = "4096"
  IF AIPriceIn > 0 THEN AIPriceInEdit.Text = LTRIM$(STR$(AIPriceIn)) ELSE AIPriceInEdit.Text = ""
  IF AIPriceOut > 0 THEN AIPriceOutEdit.Text = LTRIM$(STR$(AIPriceOut)) ELSE AIPriceOutEdit.Text = ""
  IF AIPermission = "read-only" THEN AIPermBox.ItemIndex = 1 ELSE AIPermBox.ItemIndex = 0
  AIReviewBox.Checked = AIReviewFirst
  AIWsEdit.Text = AIWorkspace
  IF AIBase <> "" THEN AIBaseBox.Text = AIBase ELSE AIBaseBox.Text = ""
  AISettingsProvider
END SUB

' (the address and workspace as typed, into the tab's RAI: the provider's
' list is asked of them)
SUB AISetTakeRows
  SetAI.Provider = AISetProv
  SetAI.APIKeySource = AIKeySourceOf(AISetProv)
  IF AISetProv = "compatible" THEN SetAI.BaseURL = LTRIM$(RTRIM$(AIBaseBox.Text)) ELSE SetAI.BaseURL = ""
  IF AISetProv = "anthropic" THEN SetAI.WorkspaceID = LTRIM$(RTRIM$(AIWsEdit.Text)) ELSE SetAI.WorkspaceID = ""
END SUB

' The rows for the provider shown: its key, its workspace or URL, its
' models, its links.
SUB AISettingsProvider
  DIM links AS STRING, web AS INTEGER
  web = Flag(Proj.FileManager = "")
  AISetTakeRows
  ' (a compatible server's address: the one kept, else the provider's own)
  IF AISetProv = "compatible" AND LTRIM$(AIBaseBox.Text) = "" THEN
    AIBaseBox.Text = SetAI.BaseURL
    SetAI.BaseURL = AIBaseBox.Text
  END IF
  AIKeyEdit.Text = ""
  AISetKeyNote
  ' (the web: kept for the tab unless the user asks for this device; its
  ' own row, above the workspace or the address: LayOutSettingsDialog)
  AIRemember.Visible = web
  AIRemember.Enabled = SetAI.NeedsKey
  AIWsLabel.Visible = Flag(AISetProv = "anthropic")
  AIWsEdit.Visible = AIWsLabel.Visible
  AIBaseLabel.Visible = Flag(AISetProv = "compatible")
  AIBaseBox.Visible = AIBaseLabel.Visible
  LayOutSettingsDialog
  AISetFillModels AIModelOf(AISetProv)
  AIPricesLink.Visible = Flag(SetAI.PricingURL <> "")
  ' (the provider's own pages; what was sent)
  IF SetAI.KeysURL <> "" THEN links = "[Get a key](" + SetAI.KeysURL + ") · "
  links = links + "[What was sent](#sent)" + CHR$(10) + CHR$(10)
  links = links + "Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project — nothing outside it, nothing before you press Send. A log of every request and change stays on this computer."
  AISetLinks.Text = links
END SUB

' (where the shown provider's key is: never the key, its last four
' characters at most)
SUB AISetKeyNote
  DIM keyNote AS STRING, web AS INTEGER
  web = Flag(Proj.FileManager = "")
  IF SetAI.NeedsKey = 0 THEN
    keyNote = "A local server needs no key (one is kept if you give it: some servers ask for one)."
  ELSEIF SetAI.HasKey THEN
    SELECT CASE SetAI.KeySource
      CASE "environment": keyNote = "Using the key in " + SetAI.KeyEnv + " (" + SetAI.KeyHint + "). A key saved here goes first."
      CASE "program": keyNote = "Trying the key you typed (" + SetAI.KeyHint + "): OK or Save Key keeps it."
      CASE ELSE: keyNote = "Your key " + SetAI.KeyHint + " is kept in " + SetAI.KeyStore + ", never in a project."
    END SELECT
  ELSEIF web THEN
    keyNote = "Paste your key and press Save Key: it is kept in " + SetAI.KeyStore + " (or on this device, if you tick the box), never in a project."
  ELSE
    keyNote = "Paste your key and press Save Key: it is kept in " + SetAI.KeyStore + ", never in a project."
  END IF
  AIKeyNote.Caption = keyNote
  AIKeyNoteH = AISetLinesHigh(keyNote, AIKeyNote.Width)
  IF SettingsDialog.Visible THEN LayOutSettingsDialog
  AIKeyRemoveBtn.Enabled = Flag(SetAI.HasKey AND SetAI.KeySource <> "environment" AND SetAI.KeySource <> "program")
END SUB

' The Model list: the provider's, once fetched (the fast, low-cost ones
' first), else the model chosen before; Keep stays chosen if it's there.
SUB AISetFillModels(Keep AS STRING)
  DIM all AS STRING, i AS INTEGER, line AS STRING, k AS INTEGER
  AIModelCombo.Clear
  all = SetAI.Models
  k = -1
  FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    line = FIELD$(all, CHR$(10), i)
    IF line <> "" THEN
      AIModelCombo.AddItems AIModelItem(line)
      IF FIELD$(line, CHR$(9), 1) = Keep AND Keep <> "" THEN k = AIModelCombo.ItemCount - 1
    END IF
  NEXT
  IF k < 0 AND Keep <> "" AND SetAI.ModelCount = 0 THEN
    AIModelCombo.AddItems Keep
    k = AIModelCombo.ItemCount - 1
  END IF
  AIModelCombo.ItemIndex = k
  IF k < 0 THEN AIModelCombo.Text = ""
END SUB

SUB AISetProviderPicked
  DIM k AS INTEGER
  IF AISetFilling THEN EXIT SUB
  k = AIProvBox.ItemIndex
  IF k < 0 OR k >= AIProvCount THEN EXIT SUB
  AISetProv = AIProvIds(k)
  ' (another provider: a list still coming for the last one is dropped —
  ' SetAI.Provider does it — and Fetch Models can be pressed for this one)
  AIFetchBtn.Enabled = 1
  AISetNote ""
  AISettingsProvider
END SUB

' The key typed, kept (for the provider shown); True when there was one.
FUNCTION AISetKeyTake AS INTEGER
  DIM key AS STRING
  key = LTRIM$(RTRIM$(AIKeyEdit.Text))
  IF key = "" THEN EXIT FUNCTION
  AISetTakeRows
  SetAI.APIKey = key
  ' (kept under Studio's name for the provider: the OS keychain; on the
  ' web the tab's session, or this device when asked)
  IF SetAI.RememberKey(AIRemember.Checked) THEN
    Result = 1
  ELSE
    SHOWMESSAGE SetAI.Error
  END IF
  SetAI.APIKey = ""
  AIKeyEdit.Text = ""
END FUNCTION

SUB AISetKeySave
  IF AISetKeyTake THEN
    AISetKeyNote
    ' (the assistant's key is the keychain's: it has it at once)
    AIEnableSend
    AIShowOrHide
  END IF
END SUB

SUB AISetKeyRemove
  AISetTakeRows
  SetAI.ForgetKey
  AISetKeyNote
  AIEnableSend
  AIShowOrHide
END SUB

' Fetch Models: the provider's list, asked with the key typed (tried, kept
' only by OK or Save Key) or the one kept, at the address and workspace
' typed. The button waits greyed; it comes back however the asking ends —
' the list, a refusal, no answer in time, another provider picked, the
' dialog closed.
SUB AISetFetch
  DIM key AS STRING
  AISetTakeRows
  key = LTRIM$(RTRIM$(AIKeyEdit.Text))
  SetAI.APIKey = key
  IF key <> "" THEN AISetKeyNote
  IF SetAI.NeedsKey AND SetAI.HasKey = 0 THEN
    AISetNote "Paste your API key first: the provider lists its models only to a key it knows (Get a key, below, opens the page that makes one)."
    AIFetchBtn.AccessibleDescription = AIFetchNote.Caption
    AIKeyEdit.SetFocus
    EXIT SUB
  END IF
  AIFetchBtn.Enabled = 0
  AISetNote "Asking " + AIProvNames(AIProvBox.ItemIndex) + " for its models..."
  AIFetchBtn.AccessibleDescription = AIFetchNote.Caption
  IF SetAI.RefreshModels = 0 THEN AISetError SetAI.Error
END SUB

' (the list came)
SUB AISetModelsIn(Count AS INTEGER)
  DIM keep AS STRING
  IF SetAI.Busy = 0 THEN AIFetchBtn.Enabled = 1
  IF SettingsDialog.Visible = 0 THEN EXIT SUB
  ' (the model chosen in the list, or before; nothing chosen for the user)
  keep = AIModelId(AIModelCombo.Text)
  IF keep = "" THEN keep = AIModelOf(AISetProv)
  AISetFillModels keep
  IF Count = 0 THEN
    AISetNote SetAI.Status + " None this key can chat with: check the key, or the server's models."
  ELSEIF AIModelCombo.ItemIndex < 0 THEN
    AISetNote SetAI.Status + " Choose one in the Model list (the fast, low-cost ones are first)."
  ELSE
    AISetNote SetAI.Status + " " + AIModelId(AIModelCombo.Text) + " is still chosen."
  END IF
  AIFetchBtn.AccessibleDescription = AIFetchNote.Caption
  ' (the next thing to do: choose a model — a screen reader reads the list)
  IF Count > 0 AND AIPage.Visible THEN AIModelCombo.SetFocus
END SUB

' (no list: a refused key, a wrong address, no answer in time … — said in
' plain words under the list; Fetch Models can be pressed again at once)
SUB AISetError(Message AS STRING)
  IF SetAI.Busy = 0 THEN AIFetchBtn.Enabled = 1
  IF SettingsDialog.Visible = 0 THEN EXIT SUB
  AISetNote Message
  AIFetchBtn.AccessibleDescription = Message
  AISetKeyNote
  ' (Anthropic wants a workspace for this key: the box to fill in)
  IF AIWsEdit.Visible AND INSTR(Message, "Workspace ID") > 0 AND AIPage.Visible THEN AIWsEdit.SetFocus
END SUB

' (stopped: another provider picked, the dialog closed, a newer request)
SUB AISetDone(Status AS STRING)
  IF SetAI.Busy = 0 THEN AIFetchBtn.Enabled = 1
  IF Status = "cancelled" AND SettingsDialog.Visible AND SetAI.Busy = 0 THEN AISetNote ""
END SUB

SUB AISetLink(Url AS STRING, Sender AS RMarkdownView)
  IF Url = "#sent" THEN ShowWhatWasSent
END SUB

' OK: the AI tab's choices kept (a key typed is saved too), and given to
' the assistant.
SUB AISettingsTake
  DIM n AS INTEGER
  AISetKeyTake
  AIProvider = AISetProv
  IF AISetProv = "compatible" THEN AIBase = LTRIM$(RTRIM$(AIBaseBox.Text))
  IF AISetProv = "anthropic" THEN AIWorkspace = LTRIM$(RTRIM$(AIWsEdit.Text))
  AISetModelOf AISetProv, AIModelId(AIModelCombo.Text)
  n = VAL(AIMaxEdit.Text)
  IF n < 256 THEN n = 256
  IF n > 128000 THEN n = 128000
  AIMaxTokens = n
  IF AIPermBox.ItemIndex = 1 THEN AIPermission = "read-only" ELSE AIPermission = "propose"
  AIReviewFirst = Flag(AIReviewBox.Checked)
  AIPriceIn = VAL(AIPriceInEdit.Text)
  AIPriceOut = VAL(AIPriceOutEdit.Text)
  ' (on first: Send is enabled from the assistant's state as it now is)
  AIOn = Flag(AIOnBox.Checked)
  AIApplySettings
  AIShowOrHide
END SUB

' (See prices: the provider's own page, in the browser)
SUB AISetPricesClick
  IF SetAI.PricingURL <> "" THEN AISetLinks.OpenLink SetAI.PricingURL
END SUB
