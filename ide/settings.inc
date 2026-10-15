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
' provider's prices.

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

' (the provider shown on the AI tab, and what was typed for it)
DIM AISetProv AS STRING

CREATE SettingsDialog AS RForm
  Caption = "Settings"
  Width = 600: Height = 560
  BorderStyle = 3
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
  END CREATE
  ' ---- AI ----
  CREATE AIPage AS RPanel
    BevelOuter = 0
    AccessibleName = "AI"
    CREATE AIOnBox AS RCheckBox
      Caption = "&Turn on the AI assistant"
    END CREATE
    CREATE AIOnNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = "Off by default. On, Studio shows the AI Assistant panel (View > AI Assistant, Ctrl+I): ask about your project or for a new program. Nothing is sent until you press Send there, and the assistant changes nothing itself: it proposes, you review the changes and decide."
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
  DIM th AS INTEGER, ph AS INTEGER, lx AS INTEGER, fx AS INTEGER, fw AS INTEGER, row AS INTEGER, fh AS INTEGER
  IF ThemeName = "classic" THEN
    pad = 16: w = 520: bh = 25: bw = 84: nh = 58: lh = 130: side = 120: th = 24: row = 30: fh = 21
  ELSE
    pad = 20: w = 540: bh = 32: bw = 96: nh = 66: lh = 150: side = 140: th = 30: row = 38: fh = 28
  END IF
  SettingsDialog.Font.Name = UIFont.Name: SettingsDialog.Font.Size = UIFont.Size
  ph = 548
  ' (the web: a row more, Remember on this device)
  IF Proj.FileManager = "" THEN ph = ph + 28
  SettingsTabs.Left = pad: SettingsTabs.Top = pad - 8: SettingsTabs.Width = w: SettingsTabs.Height = th
  GenPage.Left = 0: GenPage.Top = pad - 8 + th: GenPage.Width = w + 2 * pad: GenPage.Height = ph
  AIPage.Left = 0: AIPage.Top = pad - 8 + th: AIPage.Width = w + 2 * pad: AIPage.Height = ph

  ' ---- General ----
  y = pad
  IncFoldersLabel.Left = pad: IncFoldersLabel.Top = y
  IncFoldersLabel.Font.Bold = 1
  y = y + 24
  IncFoldersNote.Left = pad: IncFoldersNote.Top = y: IncFoldersNote.Width = w: IncFoldersNote.Height = nh
  IncFoldersNote.Font.Color = TC("page.dim")
  y = y + nh + 8
  IncFolders.Left = pad: IncFolders.Top = y: IncFolders.Width = w - side - 12: IncFolders.Height = lh
  IncAdd.Left = pad + w - side: IncAdd.Top = y: IncAdd.Width = side: IncAdd.Height = bh
  IncRemove.Left = pad + w - side: IncRemove.Top = y + bh + 8: IncRemove.Width = side: IncRemove.Height = bh
  y = y + lh + pad
  SettingsViewsBtn.Left = pad: SettingsViewsBtn.Top = y: SettingsViewsBtn.Width = side + 40: SettingsViewsBtn.Height = bh

  ' ---- AI ----
  lx = pad: fx = pad + 112: fw = w - 112
  y = pad - 4
  AIOnBox.Left = lx: AIOnBox.Top = y: AIOnBox.Width = w: AIOnBox.Height = 22
  AIOnBox.Font.Bold = 1
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
  AIKeyNote.Left = fx: AIKeyNote.Top = y: AIKeyNote.Width = fw: AIKeyNote.Height = 34
  AIKeyNote.Font.Color = TC("page.dim")
  y = y + 36
  ' (the web's Remember on a row of its own; then the rows a provider has:
  ' Anthropic's workspace, a compatible server's URL — in one place, shown
  ' when they apply)
  AIRemember.Left = fx: AIRemember.Top = y: AIRemember.Width = fw: AIRemember.Height = 22
  IF Proj.FileManager = "" THEN y = y + 28
  AIWsLabel.Left = lx: AIWsLabel.Top = y + 5: AIWsLabel.Width = 108
  AIWsEdit.Left = fx: AIWsEdit.Top = y: AIWsEdit.Width = fw: AIWsEdit.Height = fh
  AIBaseLabel.Left = lx: AIBaseLabel.Top = y + 5: AIBaseLabel.Width = 108
  AIBaseBox.Left = fx: AIBaseBox.Top = y: AIBaseBox.Width = fw
  y = y + row
  AIModelLabel.Left = lx: AIModelLabel.Top = y + 5: AIModelLabel.Width = 108
  AIModelCombo.Left = fx: AIModelCombo.Top = y: AIModelCombo.Width = fw - 140
  AIFetchBtn.Left = fx + fw - 132: AIFetchBtn.Top = y - (bh - fh) \ 2: AIFetchBtn.Width = 132: AIFetchBtn.Height = bh
  y = y + row
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
  y = y + row + 4
  AISetLinks.Left = lx - 18: AISetLinks.Top = y - 12: AISetLinks.Width = w + 36: AISetLinks.Height = ph - y + 8
  AISetLinks.Font = DimFont

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
  LayOutSettingsDialog
  IncFolders.Clear
  all = Proj.IncludeFolders
  n = TALLY(all, CHR$(10)) + 1
  FOR i = 1 TO n
    f = FIELD$(all, CHR$(10), i)
    IF f <> "" THEN IncFolders.AddItems f
  NEXT
  SettingsPicked
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
  AISettingsTake
  SettingsDialog.Close
  SaveSettings
END SUB

SUB SettingsCancel
  ' (the AI tab's provider as it was: the panel's)
  AIApplySettings
  SettingsDialog.Close
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

' The AI tab from the settings (and the assistant's state: the key's hint).
SUB AISettingsFill
  DIM k AS INTEGER, all AS STRING, i AS INTEGER, line AS STRING
  AIOnBox.Checked = AIOn
  IF AIProvCount = 0 THEN EXIT SUB
  IF AIProvBox.ItemCount = 0 THEN
    FOR k = 0 TO AIProvCount - 1
      AIProvBox.AddItems AIProvNames(k)
    NEXT
    all = AI.Presets
    FOR i = 1 TO TALLY(all, CHR$(10)) + 1
      line = FIELD$(all, CHR$(10), i)
      IF line <> "" THEN AIBaseBox.AddItems FIELD$(line, CHR$(9), 2)
    NEXT
    AIPermBox.AddItems "Read the project, propose changes you review", "Only read the project and answer"
  END IF
  AISetProv = AIProvider
  ' (none chosen yet: a local server first — it keeps the project on this
  ' computer — but every provider is a pick away)
  IF AISetProv = "" THEN AISetProv = AIProvIds(0)
  FOR k = 0 TO AIProvCount - 1
    IF AIProvIds(k) = AISetProv THEN AIProvBox.ItemIndex = k
  NEXT
  IF AIMaxTokens > 0 THEN AIMaxEdit.Text = LTRIM$(STR$(AIMaxTokens)) ELSE AIMaxEdit.Text = "4096"
  IF AIPriceIn > 0 THEN AIPriceInEdit.Text = LTRIM$(STR$(AIPriceIn)) ELSE AIPriceInEdit.Text = ""
  IF AIPriceOut > 0 THEN AIPriceOutEdit.Text = LTRIM$(STR$(AIPriceOut)) ELSE AIPriceOutEdit.Text = ""
  IF AIPermission = "read-only" THEN AIPermBox.ItemIndex = 1 ELSE AIPermBox.ItemIndex = 0
  AIWsEdit.Text = AIWorkspace
  AISettingsProvider
END SUB

' The rows for the provider shown: its key, its workspace or URL, its
' models, its links.
SUB AISettingsProvider
  DIM keyNote AS STRING, links AS STRING, web AS INTEGER, model AS STRING
  web = Flag(Proj.FileManager = "")
  AI.Provider = AISetProv
  AI.APIKeySource = AIKeySourceOf(AISetProv)
  IF AISetProv = "compatible" THEN
    IF AIBase <> "" THEN AIBaseBox.Text = AIBase ELSE AIBaseBox.Text = AI.BaseURL
  END IF
  AIKeyEdit.Text = ""
  IF AI.NeedsKey = 0 THEN
    keyNote = "A local server needs no key (one is kept if you give it: some servers ask for one)."
  ELSEIF AI.HasKey THEN
    SELECT CASE AI.KeySource
      CASE "environment": keyNote = "Using the key in " + AI.KeyEnv + " (" + AI.KeyHint + "). A key saved here goes first."
      CASE "program": keyNote = "A key " + AI.KeyHint + " is in use until Studio closes: Save Key keeps it."
      CASE ELSE: keyNote = "Your key " + AI.KeyHint + " is kept in " + AI.KeyStore + ", never in a project."
    END SELECT
  ELSEIF web THEN
    keyNote = "Paste your key and press Save Key: it is kept in " + AI.KeyStore + " (or on this device, if you tick the box), never in a project."
  ELSE
    keyNote = "Paste your key and press Save Key: it is kept in " + AI.KeyStore + ", never in a project."
  END IF
  AIKeyNote.Caption = keyNote
  AIKeyRemoveBtn.Enabled = Flag(AI.HasKey AND AI.KeySource <> "environment")
  ' (the web: kept for the tab unless the user asks for this device)
  ' (its own row, above the workspace or the address: LayOutSettingsDialog)
  AIRemember.Visible = web
  AIRemember.Enabled = AI.NeedsKey
  AIWsLabel.Visible = Flag(AISetProv = "anthropic")
  AIWsEdit.Visible = AIWsLabel.Visible
  AIBaseLabel.Visible = Flag(AISetProv = "compatible")
  AIBaseBox.Visible = AIBaseLabel.Visible
  ' (the model the user chose for it; none until they do: Fetch Models
  ' lists the provider's, the fast, low-cost ones first)
  model = AIModelOf(AISetProv)
  AIModelCombo.Clear
  IF model <> "" THEN AIModelCombo.AddItems model
  AIModelCombo.Text = model
  AIPricesLink.Visible = Flag(AI.PricingURL <> "")
  ' (the provider's own pages; what was sent)
  IF AI.KeysURL <> "" THEN links = "[Get a key](" + AI.KeysURL + ") · "
  links = links + "[What was sent](#sent)" + CHR$(10) + CHR$(10)
  links = links + "Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project — nothing outside it, nothing before you press Send. A log of every request and change stays on this computer."
  AISetLinks.Text = links
END SUB

SUB AISetProviderPicked
  DIM k AS INTEGER
  k = AIProvBox.ItemIndex
  IF k < 0 OR k >= AIProvCount THEN EXIT SUB
  AISetProv = AIProvIds(k)
  AISettingsProvider
END SUB

' The key typed, kept (for the provider shown); True when there was one.
FUNCTION AISetKeyTake AS INTEGER
  DIM key AS STRING
  key = LTRIM$(RTRIM$(AIKeyEdit.Text))
  IF key = "" THEN EXIT FUNCTION
  AI.Provider = AISetProv
  AI.APIKeySource = AIKeySourceOf(AISetProv)
  AI.APIKey = key
  ' (kept under Studio's name for the provider: the OS keychain; on the
  ' web the tab's session, or this device when asked)
  IF AI.RememberKey(AIRemember.Checked) THEN
    Result = 1
  ELSE
    SHOWMESSAGE AI.Error
  END IF
  AI.APIKey = ""
  AIKeyEdit.Text = ""
END FUNCTION

SUB AISetKeySave
  IF AISetKeyTake THEN AISettingsProvider
END SUB

SUB AISetKeyRemove
  AI.Provider = AISetProv
  AI.APIKeySource = AIKeySourceOf(AISetProv)
  AI.ForgetKey
  AISettingsProvider
END SUB

SUB AISetFetch
  AISetKeyTake
  AI.Provider = AISetProv
  IF AISetProv = "compatible" THEN AI.BaseURL = AIBaseBox.Text
  IF AISetProv = "anthropic" THEN AI.WorkspaceID = AIWsEdit.Text
  AI.APIKeySource = AIKeySourceOf(AISetProv)
  AI.Model = AIModelOf(AISetProv)
  AIFetchBtn.Enabled = 0
  AIKeyNote.Caption = "Asking the provider for its models..."
  IF AI.RefreshModels = 0 THEN
    AIFetchBtn.Enabled = 1
    AIKeyNote.Caption = AI.Error
  END IF
END SUB

' (AIModelsIn: the list arrived while the settings are open)
SUB AISettingsModelsIn
  DIM all AS STRING, i AS INTEGER, line AS STRING, keep AS STRING
  AIFetchBtn.Enabled = 1
  IF SettingsDialog.Visible = 0 THEN EXIT SUB
  keep = AIModelCombo.Text
  AIModelCombo.Clear
  all = AI.Models
  FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    line = FIELD$(all, CHR$(10), i)
    IF line <> "" THEN AIModelCombo.AddItems AIModelItem(line)
  NEXT
  ' (nothing chosen for the user: what they chose, if the list still has it)
  IF AI.Model = "" THEN keep = ""
  AISettingsProvider
  AIModelCombo.Text = keep
  IF keep = "" THEN
    AIKeyNote.Caption = AI.Status + " Choose one (the fast, low-cost ones are first)."
  ELSE
    AIKeyNote.Caption = AI.Status + " " + AIKeyNote.Caption
  END IF
END SUB

SUB AISetLink(Url AS STRING, Sender AS RMarkdownView)
  IF Url = "#sent" THEN ShowWhatWasSent
END SUB

' OK: the AI tab's choices kept (a key typed is saved too).
SUB AISettingsTake
  DIM was AS INTEGER, n AS INTEGER
  AISetKeyTake
  was = AIOn
  AIProvider = AISetProv
  IF AISetProv = "compatible" THEN AIBase = LTRIM$(RTRIM$(AIBaseBox.Text))
  IF AISetProv = "anthropic" THEN AIWorkspace = LTRIM$(RTRIM$(AIWsEdit.Text))
  AISetModelOf AISetProv, AIModelId(AIModelCombo.Text)
  n = VAL(AIMaxEdit.Text)
  IF n < 256 THEN n = 256
  IF n > 128000 THEN n = 128000
  AIMaxTokens = n
  IF AIPermBox.ItemIndex = 1 THEN AIPermission = "read-only" ELSE AIPermission = "propose"
  AIPriceIn = VAL(AIPriceInEdit.Text)
  AIPriceOut = VAL(AIPriceOutEdit.Text)
  AIApplySettings
  AIOn = Flag(AIOnBox.Checked)
  AIShowOrHide
END SUB

' (See prices: the provider's own page, in the browser)
SUB AISetPricesClick
  IF AI.PricingURL <> "" THEN AISetLinks.OpenLink AI.PricingURL
END SUB
