' aisetup.inc: Set up RAI (docs/preview4-plan.md R6) — the assistant set up
' in two steps, in plain words, for someone who has never done it:
'
'   1. Where the answers come from: free, on this computer (Ollama, LM
'      Studio or MLX Serve — found if one is running; nothing is started), or
'      a cloud service with your own key (Anthropic, OpenAI, DeepSeek) —
'      with what each costs and what it sends, said plainly.
'   2. On this computer: the server and a model. In the cloud: the
'      provider, the key (a key Studio already finds — the keychain, the
'      environment — can be used as it is), Test, then a model, the fast,
'      low-cost ones first.
'
' Finish turns the assistant on with those choices (a key typed is kept
' in the system's keychain, as Settings > AI keeps it: never in a project)
' and shows it. Settings > AI has every detail afterwards. The wizard has
' RAIs of its own (probes, a test): nothing reaches the assistant before
' Finish; Cancel leaves everything as it was.

DECLARE SUB AISuNext
DECLARE SUB AISuBack
DECLARE SUB AISuCancel
DECLARE SUB AISuLook
DECLARE SUB AISuPicked
DECLARE SUB AISuServerPicked
DECLARE SUB AISuProvPicked
DECLARE SUB AISuTest
DECLARE SUB AISuKeyLinkClick
DECLARE SUB AISuProbe1In(Count AS INTEGER)
DECLARE SUB AISuProbe2In(Count AS INTEGER)
DECLARE SUB AISuProbe3In(Count AS INTEGER)
DECLARE SUB AISuProbe1Err(Message AS STRING)
DECLARE SUB AISuProbe2Err(Message AS STRING)
DECLARE SUB AISuProbe3Err(Message AS STRING)
DECLARE SUB AISuTestIn(Count AS INTEGER)
DECLARE SUB AISuTestErr(Message AS STRING)
DECLARE SUB AISuShowPage(P AS INTEGER)
DECLARE SUB AISuFound
DECLARE SUB LayOutAISetup
DECLARE SUB AISuFinish
DECLARE SUB AISuFillModels(Which AS INTEGER, Models AS STRING)
DECLARE SUB AISuFonts

' (the servers looked for on this computer: names, addresses, what each
' answered — 0 not yet, 1 running, -1 not running —, how many models)
CONST AISUSERVERS = 3
DIM AISuNames(AISUSERVERS) AS STRING
DIM AISuUrls(AISUSERVERS) AS STRING
DIM AISuState(AISUSERVERS) AS INTEGER
DIM AISuCount(AISUSERVERS) AS INTEGER
DIM AISuProbe1 AS RAI
DIM AISuProbe2 AS RAI
DIM AISuProbe3 AS RAI
' (the cloud's: the key typed is tried by this one, kept only by Finish)
DIM AISuCloud AS RAI
' (the page shown: 1 where, 2 this computer, 3 the cloud)
DIM AISuPage AS INTEGER
' (the cloud providers the list shows, by id)
DIM AISuProvIds(4) AS STRING
DIM AISuProvCount AS INTEGER

CREATE AISetup AS RForm
  Caption = "Set Up RAI"
  Width = 660: Height = 560
  BorderStyle = 3
  CREATE AISuFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE AISuBackBtn AS RButton
      Caption = "< &Back"
      AccessibleName = "Back"
      OnClick = AISuBack
    END CREATE
    CREATE AISuCancelBtn AS RButton
      Caption = "Cancel"
      Cancel = 1
      OnClick = AISuCancel
    END CREATE
    CREATE AISuNextBtn AS RButton
      Caption = "&Next >"
      Default = 1
      OnClick = AISuNext
    END CREATE
  END CREATE
  CREATE AISuHead AS RPanel
    Align = 1
    Height = 56
    BevelOuter = 0
    CREATE AISuTitle AS RLabel
      Left = 24: Top = 18: Width = 560: Height = 28
      AutoSize = 0
    END CREATE
  END CREATE
  ' ---- 1: where the answers come from ----
  CREATE AISuPage1 AS RPanel
    Align = 5
    BevelOuter = 0
    AccessibleName = "Where RAI's answers come from"
    CREATE AISuLocal AS RRadioButton
      Caption = "Free, on &this computer"
      OnClick = AISuPicked
    END CREATE
    CREATE AISuLocalText AS RLabel
      AutoSize = 0: WordWrap = 1
      Caption = "Nothing you ask leaves this computer, and it costs nothing. A model runs here with Ollama, LM Studio or MLX Serve (free programs you install). It needs a fast computer with plenty of memory (16 GB or more); answers are slower and simpler than a cloud model's."
    END CREATE
    CREATE AISuFoundText AS RLabel
      AutoSize = 0: WordWrap = 1
      AccessibleName = "The servers found on this computer"
    END CREATE
    CREATE AISuLookBtn AS RButton
      Caption = "&Look Again"
      AccessibleName = "Look again for a server on this computer"
      OnClick = AISuLook
    END CREATE
    CREATE AISuCloudRadio AS RRadioButton
      Caption = "A &cloud service, with your own key"
      OnClick = AISuPicked
    END CREATE
    CREATE AISuCloudText AS RLabel
      AutoSize = 0: WordWrap = 1
      Caption = "Anthropic (Claude), OpenAI or DeepSeek: usually the best answers. You make a key on the provider's website and pay it for what you use: a question usually costs from a tenth of a cent (small, fast models) to a few cents (the largest). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to that provider, under its privacy terms. Your key stays in this computer's keychain, never in a project."
    END CREATE
  END CREATE
  ' ---- 2: on this computer ----
  CREATE AISuPage2 AS RPanel
    Align = 5
    BevelOuter = 0
    Visible = 0
    AccessibleName = "A server on this computer"
    CREATE AISuServerLabel AS RLabel
      Caption = "&Server:"
      Left = 24: Top = 12: Width = 100
    END CREATE
    CREATE AISuServerBox AS RComboBox
      Style = 2
      Left = 130: Top = 8: Width = 300
      AccessibleName = "The server on this computer"
      OnChange = AISuServerPicked
    END CREATE
    CREATE AISuAddrLabel AS RLabel
      Caption = "&Address:"
      Left = 24: Top = 48: Width = 100
    END CREATE
    CREATE AISuAddrEdit AS REdit
      Left = 130: Top = 44: Width = 300
      AccessibleName = "The server's address"
    END CREATE
    CREATE AISuLocalModelLabel AS RLabel
      Caption = "&Model:"
      Left = 24: Top = 84: Width = 100
    END CREATE
    CREATE AISuLocalModel AS RComboBox
      Left = 130: Top = 80: Width = 300
      AccessibleName = "Model"
    END CREATE
    CREATE AISuLocalNote AS RLabel
      Left = 24: Top = 124: Width = 560: Height = 96
      AutoSize = 0: WordWrap = 1
      AccessibleName = "About this server"
    END CREATE
  END CREATE
  ' ---- 3: a cloud service ----
  CREATE AISuPage3 AS RPanel
    Align = 5
    BevelOuter = 0
    Visible = 0
    AccessibleName = "A cloud service"
    CREATE AISuProvLabel AS RLabel
      Caption = "&Provider:"
      Left = 24: Top = 12: Width = 100
    END CREATE
    CREATE AISuProvBox AS RComboBox
      Style = 2
      Left = 130: Top = 8: Width = 300
      AccessibleName = "The cloud provider"
      OnChange = AISuProvPicked
    END CREATE
    CREATE AISuKeyLink AS RLabel
      Caption = "Get a key"
      Left = 444: Top = 12: Width = 150
      Cursor = -21
      AccessibleName = "Get a key on the provider's website, in the browser"
      OnClick = AISuKeyLinkClick
    END CREATE
    CREATE AISuKeyLabel AS RLabel
      Caption = "API &key:"
      Left = 24: Top = 48: Width = 100
    END CREATE
    CREATE AISuKeyEdit AS REdit
      PasswordChar = "*"
      Left = 130: Top = 44: Width = 300
      AccessibleName = "API key"
      AccessibleDescription = "Paste the key the provider gave you: it is kept in this computer's keychain, never in a project"
    END CREATE
    CREATE AISuTestBtn AS RButton
      Caption = "&Test the Key"
      AccessibleName = "Test the key: ask the provider for its models"
      OnClick = AISuTest
    END CREATE
    CREATE AISuTestNote AS RLabel
      Left = 130: Top = 80: Width = 460: Height = 54
      AutoSize = 0: WordWrap = 1
      AccessibleName = "The key's test"
    END CREATE
    CREATE AISuCloudModelLabel AS RLabel
      Caption = "&Model:"
      Left = 24: Top = 144: Width = 100
    END CREATE
    CREATE AISuCloudModel AS RComboBox
      Left = 130: Top = 140: Width = 300
      AccessibleName = "Model"
    END CREATE
    CREATE AISuCostText AS RLabel
      Left = 24: Top = 182: Width = 560: Height = 130
      AutoSize = 0: WordWrap = 1
      AccessibleName = "What it costs, and what is sent"
    END CREATE
  END CREATE
END CREATE

SUB SetUpAISetup
  DIM k AS INTEGER
  AISuNames(0) = "Ollama": AISuUrls(0) = "http://localhost:11434/v1"
  AISuNames(1) = "LM Studio": AISuUrls(1) = "http://localhost:1234/v1"
  AISuNames(2) = "MLX Serve": AISuUrls(2) = "http://localhost:8080/v1"
  AISuProbe1.OnModelsLoaded = AISuProbe1In
  AISuProbe2.OnModelsLoaded = AISuProbe2In
  AISuProbe3.OnModelsLoaded = AISuProbe3In
  AISuProbe1.OnError = AISuProbe1Err
  AISuProbe2.OnError = AISuProbe2Err
  AISuProbe3.OnError = AISuProbe3Err
  AISuCloud.OnModelsLoaded = AISuTestIn
  AISuCloud.OnError = AISuTestErr
  ' (the cloud providers, as RAI lists them: all but a compatible server)
  AISuProvCount = 0
  FOR k = 0 TO AIProvCount - 1
    IF AIProvIds(k) <> "compatible" AND AISuProvCount < 4 THEN
      AISuProvIds(AISuProvCount) = AIProvIds(k)
      AISuProvBox.AddItems AIProvNames(k)
      AISuProvCount = AISuProvCount + 1
    END IF
  NEXT
END SUB

' Tools > Set up RAI (and Ctrl+I before it is set up): from the first page,
' the servers on this computer looked for at once.
SUB ShowAISetup
  AISuFonts
  AISuShowPage 1
  AISuLook
  IF AIProvider = "compatible" OR AIProvider = "" THEN AISuLocal.Checked = 1 ELSE AISuCloudRadio.Checked = 1
  AISuPicked
  AISetup.Center
  AISetup.ShowModal
END SUB

SUB AISuFonts
  AISuTitle.Font = SubFont
  AISuTitle.Font.Color = TC("page.text")
  AISuLocal.Font = UIFont
  AISuLocal.Font.Bold = 1
  AISuCloudRadio.Font = UIFont
  AISuCloudRadio.Font.Bold = 1
  AISuLocalText.Font = UIFont
  AISuLocalText.Font.Color = TC("page.dim")
  AISuCloudText.Font = UIFont
  AISuCloudText.Font.Color = TC("page.dim")
  AISuFoundText.Font = UIFont
  AISuFoundText.Font.Color = TC("page.text")
  AISuLocalNote.Font = UIFont
  AISuLocalNote.Font.Color = TC("page.dim")
  AISuTestNote.Font = UIFont
  AISuTestNote.Font.Color = TC("page.text")
  AISuCostText.Font = UIFont
  AISuCostText.Font.Color = TC("page.dim")
  AISuKeyLink.Font = UIFont
  AISuKeyLink.Font.Color = TC("page.link")
  LayOutAISetup
END SUB

SUB LayOutAISetup
  DIM w AS INTEGER, bh AS INTEGER, fw AS INTEGER
  w = AISetup.ClientWidth
  IF w < 400 THEN w = 400
  IF ThemeName = "classic" THEN bh = 25 ELSE bh = 30
  AISuTitle.Width = w - 48
  AISuNextBtn.Left = w - 24 - 110: AISuNextBtn.Top = 12: AISuNextBtn.Width = 110: AISuNextBtn.Height = bh
  AISuCancelBtn.Left = w - 24 - 110 - 8 - 96: AISuCancelBtn.Top = 12: AISuCancelBtn.Width = 96: AISuCancelBtn.Height = bh
  AISuBackBtn.Left = 24: AISuBackBtn.Top = 12: AISuBackBtn.Width = 96: AISuBackBtn.Height = bh
  ' (1: the two choices, each with what it means; what was found, Look Again)
  AISuLocal.Left = 24: AISuLocal.Top = 6: AISuLocal.Width = w - 48: AISuLocal.Height = 24
  AISuLocalText.Left = 48: AISuLocalText.Top = 32: AISuLocalText.Width = w - 72: AISuLocalText.Height = 78
  AISuFoundText.Left = 48: AISuFoundText.Top = 114: AISuFoundText.Width = w - 72 - 132: AISuFoundText.Height = 44
  AISuLookBtn.Left = w - 24 - 120: AISuLookBtn.Top = 114: AISuLookBtn.Width = 120: AISuLookBtn.Height = bh
  AISuCloudRadio.Left = 24: AISuCloudRadio.Top = 174: AISuCloudRadio.Width = w - 48: AISuCloudRadio.Height = 24
  AISuCloudText.Left = 48: AISuCloudText.Top = 200: AISuCloudText.Width = w - 72: AISuCloudText.Height = 100
  ' (2 and 3: a label column, the fields, notes under them)
  fw = w - 130 - 24 - 160
  AISuServerBox.Width = fw: AISuAddrEdit.Width = fw: AISuLocalModel.Width = fw
  AISuLocalNote.Width = w - 48: AISuLocalNote.Height = 96
  AISuProvBox.Width = fw: AISuKeyEdit.Width = fw: AISuCloudModel.Width = fw
  AISuKeyLink.Left = 130 + fw + 14: AISuKeyLink.Width = 150
  AISuTestBtn.Left = 130 + fw + 8: AISuTestBtn.Top = 42: AISuTestBtn.Width = 150: AISuTestBtn.Height = bh
  AISuTestNote.Width = w - 130 - 24: AISuTestNote.Height = 54
  AISuCostText.Width = w - 48: AISuCostText.Height = 130
END SUB

SUB AISuShowPage(P AS INTEGER)
  AISuPage = P
  AISuPage1.Visible = Flag(P = 1)
  AISuPage2.Visible = Flag(P = 2)
  AISuPage3.Visible = Flag(P = 3)
  AISuBackBtn.Visible = Flag(P > 1)
  SELECT CASE P
    CASE 1
      AISuTitle.Caption = "Where should RAI's answers come from?"
      AISuNextBtn.Caption = "&Next >"
    CASE 2
      AISuTitle.Caption = "RAI on this computer"
      AISuNextBtn.Caption = "&Finish"
    CASE 3
      AISuTitle.Caption = "RAI from a cloud service"
      AISuNextBtn.Caption = "&Finish"
  END SELECT
  AISuTitle.AccessibleName = AISuTitle.Caption
  AISetup.Caption = "Set Up RAI — " + AISuTitle.Caption
END SUB

' (the choice on page 1: the found servers say so; Next goes on)
SUB AISuPicked
  AISuFound
END SUB

' ---- this computer: the servers looked for ----------------------------------------

' Each address asked for its models (a GET: nothing is started or loaded).
SUB AISuLook
  DIM k AS INTEGER
  FOR k = 0 TO AISUSERVERS - 1
    AISuState(k) = 0
    AISuCount(k) = 0
  NEXT
  AISuProbe1.Provider = "compatible": AISuProbe1.BaseURL = AISuUrls(0)
  AISuProbe2.Provider = "compatible": AISuProbe2.BaseURL = AISuUrls(1)
  AISuProbe3.Provider = "compatible": AISuProbe3.BaseURL = AISuUrls(2)
  IF AISuProbe1.RefreshModels = 0 THEN AISuState(0) = -1
  IF AISuProbe2.RefreshModels = 0 THEN AISuState(1) = -1
  IF AISuProbe3.RefreshModels = 0 THEN AISuState(2) = -1
  AISuFound
END SUB

SUB AISuProbe1In(Count AS INTEGER)
  AISuState(0) = 1: AISuCount(0) = Count
  AISuFound
END SUB
SUB AISuProbe2In(Count AS INTEGER)
  AISuState(1) = 1: AISuCount(1) = Count
  AISuFound
END SUB
SUB AISuProbe3In(Count AS INTEGER)
  AISuState(2) = 1: AISuCount(2) = Count
  AISuFound
END SUB
SUB AISuProbe1Err(Message AS STRING)
  AISuState(0) = -1
  AISuFound
END SUB
SUB AISuProbe2Err(Message AS STRING)
  AISuState(1) = -1
  AISuFound
END SUB
SUB AISuProbe3Err(Message AS STRING)
  AISuState(2) = -1
  AISuFound
END SUB

' What was found, in words ("Found: Ollama (3 models). Not running: LM
' Studio, MLX Serve."); the servers' list on page 2 the running ones first.
SUB AISuFound
  DIM k AS INTEGER, found AS STRING, notRun AS STRING, asking AS STRING, n AS STRING
  FOR k = 0 TO AISUSERVERS - 1
    SELECT CASE AISuState(k)
      CASE 1
        IF AISuCount(k) = 1 THEN n = "1 model" ELSE n = LTRIM$(STR$(AISuCount(k))) + " models"
        IF AISuCount(k) = 0 THEN n = "no model yet: download one in it first"
        IF found <> "" THEN found = found + "; "
        found = found + AISuNames(k) + " (" + n + ")"
      CASE -1
        IF notRun <> "" THEN notRun = notRun + ", "
        notRun = notRun + AISuNames(k)
      CASE ELSE
        IF asking <> "" THEN asking = asking + ", "
        asking = asking + AISuNames(k)
    END SELECT
  NEXT
  IF found <> "" THEN
    AISuFoundText.Caption = "✓ Found on this computer: " + found + "."
  ELSEIF asking <> "" THEN
    AISuFoundText.Caption = "Looking for a server on this computer…"
  ELSE
    AISuFoundText.Caption = "None found running: install Ollama, LM Studio or MLX Serve, start it, then Look Again."
  END IF
  IF notRun <> "" AND found <> "" THEN AISuFoundText.Caption = AISuFoundText.Caption + " Not running: " + notRun + "."
  AISuFoundText.AccessibleDescription = AISuFoundText.Caption
END SUB

' ---- the pages ------------------------------------------------------------------

SUB AISuNext
  DIM k AS INTEGER, first AS INTEGER
  SELECT CASE AISuPage
    CASE 1
      IF AISuLocal.Checked THEN
        ' (the servers, the running ones first; the address can be another)
        AISuServerBox.Clear
        first = -1
        FOR k = 0 TO AISUSERVERS - 1
          IF AISuState(k) = 1 THEN
            AISuServerBox.AddItems AISuNames(k) + " (running)"
            IF first < 0 THEN first = k
          ELSE
            AISuServerBox.AddItems AISuNames(k) + " (not running)"
          END IF
        NEXT
        IF first < 0 THEN first = 0
        AISuServerBox.ItemIndex = first
        AISuShowPage 2
        AISuServerPicked
        AISuServerBox.SetFocus
      ELSE
        AISuShowPage 3
        IF AISuProvBox.ItemIndex < 0 THEN
          AISuProvBox.ItemIndex = 0
          FOR k = 0 TO AISuProvCount - 1
            IF AISuProvIds(k) = AIProvider THEN AISuProvBox.ItemIndex = k
          NEXT
        END IF
        AISuProvPicked
        AISuKeyEdit.SetFocus
      END IF
    CASE 2, 3
      AISuFinish
  END SELECT
END SUB

SUB AISuBack
  AISuShowPage 1
END SUB

SUB AISuCancel
  AISuCloud.APIKey = ""
  AISuKeyEdit.Text = ""
  AISetup.Close
END SUB

' (a server picked on page 2: its address, its models)
SUB AISuServerPicked
  DIM k AS INTEGER, models AS STRING
  k = AISuServerBox.ItemIndex
  IF k < 0 THEN EXIT SUB
  AISuAddrEdit.Text = AISuUrls(k)
  SELECT CASE k
    CASE 0: models = AISuProbe1.Models
    CASE 1: models = AISuProbe2.Models
    CASE 2: models = AISuProbe3.Models
  END SELECT
  AISuFillModels 2, models
  IF AISuState(k) = 1 AND AISuCount(k) = 0 THEN
    AISuLocalNote.Caption = AISuNames(k) + " is running here, but has no model yet: download one in " + AISuNames(k) + " (Studio never downloads or starts one for you), then go Back and Look Again."
  ELSEIF AISuState(k) = 1 THEN
    AISuLocalNote.Caption = AISuNames(k) + " is running here. Choose a model and Finish. Free, and nothing you ask leaves this computer. A model of a small window (under 16,000 tokens) gets RAI's lite instructions, so it can still help."
  ELSE
    AISuLocalNote.Caption = AISuNames(k) + " isn't running at " + AISuUrls(k) + ". Start it (Studio never starts it for you), then go Back and Look Again; or type another address."
  END IF
  AISuLocalNote.AccessibleDescription = AISuLocalNote.Caption
END SUB

' (a provider picked on page 3: what it costs, where its keys are made, a
' key Studio already finds for it)
SUB AISuProvPicked
  DIM k AS INTEGER, id AS STRING, name AS STRING
  k = AISuProvBox.ItemIndex
  IF k < 0 THEN EXIT SUB
  id = AISuProvIds(k)
  name = AISuProvBox.Item(k)
  AISuCloud.Provider = id
  AISuCloud.APIKeySource = AIKeySourceOf(id)
  AISuCloud.APIKey = ""
  AISuCloudModel.Clear
  AISuCloudModel.Text = ""
  IF AISuCloud.HasKey THEN
    AISuTestNote.Caption = "A key for " + name + " is already found (" + AISuCloud.KeyHint + ", " + AISuCloud.KeyStore + "): Test it, or paste another."
  ELSE
    AISuTestNote.Caption = "Paste the key " + name + " gave you (Get a key opens its page), then Test the Key."
  END IF
  AISuTestNote.AccessibleDescription = AISuTestNote.Caption
  SELECT CASE id
    CASE "anthropic"
      AISuCostText.Caption = "Claude's small, fast models (Haiku) cost about a tenth of a cent a question; the largest (Opus) a few cents. Anthropic charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to Anthropic, under its privacy terms."
    CASE "openai"
      AISuCostText.Caption = "OpenAI's small models (mini, nano) cost a fraction of a cent a question; the largest a few cents. OpenAI charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to OpenAI, under its privacy terms."
    CASE "deepseek"
      AISuCostText.Caption = "DeepSeek's models cost a fraction of a cent a question. DeepSeek charges your account for what you use (you pay in advance on its website). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to DeepSeek, in China, under its privacy terms."
    CASE ELSE
      AISuCostText.Caption = "You pay the provider for what you use. What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms."
  END SELECT
  AISuCostText.Caption = AISuCostText.Caption + " Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key."
  AISuCostText.AccessibleDescription = AISuCostText.Caption
END SUB

SUB AISuKeyLinkClick
  IF AISuCloud.KeysURL <> "" THEN AISetLinks.OpenLink AISuCloud.KeysURL
END SUB

' Test the Key: the provider asked for its models with the key typed (or
' the one found) — a refused key said plainly; the list fills, the fast,
' low-cost ones first.
SUB AISuTest
  DIM key AS STRING
  key = LTRIM$(RTRIM$(AISuKeyEdit.Text))
  IF key <> "" THEN AISuCloud.APIKey = key
  IF AISuCloud.HasKey = 0 THEN
    AISuTestNote.Caption = "Paste your API key first: the provider lists its models only to a key it knows."
    AISuKeyEdit.SetFocus
    EXIT SUB
  END IF
  AISuTestBtn.Enabled = 0
  AISuTestNote.Caption = "Testing the key…"
  IF AISuCloud.RefreshModels = 0 THEN AISuTestErr AISuCloud.Error
END SUB

SUB AISuTestIn(Count AS INTEGER)
  AISuTestBtn.Enabled = 1
  AISuFillModels 3, AISuCloud.Models
  IF Count = 0 THEN
    AISuTestNote.Caption = "The key works, but it can chat with no model: check it on the provider's website."
  ELSE
    AISuTestNote.Caption = "✓ The key works: " + LTRIM$(STR$(Count)) + " models. The first, " + AIModelId(AISuCloudModel.Text) + ", is fast and low-cost: a good start. Finish when you have chosen."
    AISuCloudModel.SetFocus
  END IF
  AISuTestNote.AccessibleDescription = AISuTestNote.Caption
END SUB

SUB AISuTestErr(Message AS STRING)
  AISuTestBtn.Enabled = 1
  AISuTestNote.Caption = Message
  AISuTestNote.AccessibleDescription = Message
END SUB

' (a models list into a box: the fast, low-cost ones first, the first one
' chosen — the user picks another)
SUB AISuFillModels(Which AS INTEGER, Models AS STRING)
  DIM i AS INTEGER, line AS STRING, n AS INTEGER
  IF Which = 2 THEN AISuLocalModel.Clear ELSE AISuCloudModel.Clear
  IF Models <> "" THEN
    FOR i = 1 TO TALLY(Models, CHR$(10)) + 1
      line = FIELD$(Models, CHR$(10), i)
      IF FIELD$(line, CHR$(9), 1) <> "" THEN
        IF Which = 2 THEN AISuLocalModel.AddItems AIModelItem(line) ELSE AISuCloudModel.AddItems AIModelItem(line)
        n = n + 1
      END IF
    NEXT
  END IF
  IF Which = 2 THEN
    IF n > 0 THEN AISuLocalModel.ItemIndex = 0 ELSE AISuLocalModel.Text = ""
  ELSE
    IF n > 0 THEN AISuCloudModel.ItemIndex = 0 ELSE AISuCloudModel.Text = ""
  END IF
END SUB

' Finish: the assistant turned on with the choices; a key typed kept in the
' keychain (as Settings > AI's Save Key); its pane shown.
SUB AISuFinish
  DIM model AS STRING, key AS STRING, k AS INTEGER, id AS STRING
  IF AISuPage = 2 THEN
    model = AIModelId(AISuLocalModel.Text)
    IF model = "" THEN
      AISuLocalNote.Caption = "Choose a model first (the server lists its own): start the server and Look Again if the list is empty."
      AISuLocalModel.SetFocus
      EXIT SUB
    END IF
    AIProvider = "compatible"
    AIBase = LTRIM$(RTRIM$(AISuAddrEdit.Text))
    AISetModelOf "compatible", model
  ELSE
    k = AISuProvBox.ItemIndex
    IF k < 0 THEN EXIT SUB
    id = AISuProvIds(k)
    model = AIModelId(AISuCloudModel.Text)
    IF model = "" THEN
      AISuTestNote.Caption = "Test the key first: then choose one of the provider's models."
      AISuTestBtn.SetFocus
      EXIT SUB
    END IF
    key = LTRIM$(RTRIM$(AISuKeyEdit.Text))
    IF key <> "" THEN
      AISuCloud.APIKey = key
      IF AISuCloud.RememberKey(0) = 0 THEN
        AISuTestNote.Caption = AISuCloud.Error
        EXIT SUB
      END IF
    END IF
    AISuCloud.APIKey = ""
    AISuKeyEdit.Text = ""
    AIProvider = id
    AISetModelOf id, model
  END IF
  AIOn = 1
  AIApplySettings
  SaveSettings
  AISetup.Close
  AIShowOrHide
  IF AIReady THEN
    ShowAIPanel
    ' (POLISH-2) the provider's models asked for at once, as ↻ does: the
    ' model's context window and its kind of thinking come with them
    IF AI.Busy = 0 THEN
      IF AI.RefreshModels THEN
        AIRefreshBtn.Enabled = 0
        ' (their arrival keeps saying it's ready: ai.inc's AIModelsIn)
        AISuReady = 1
      END IF
    END IF
    AIStatus.SimpleText = "RAI is ready: " + AI.Model + "."
  ELSE
    SHOWMESSAGE "RAI isn't ready yet: " + AIEmptyText
  END IF
END SUB

' (tests and the manual's scenes: ai:setup, ai:setupprobes=url|url|url,
' ai:setuplocal, ai:setupcloud, ai:setupnext, ai:setupback,
' ai:setupprov=N, ai:setupkey=fake, ai:setuptest, ai:setupserver=N,
' ai:setupmodel=N, ai:setupcancel; True when Verb was one — never a real
' key: the tests type a made-up one)
FUNCTION AISetupStep(Verb AS STRING, Arg AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Verb
    CASE "setup": ShowAISetup
    CASE "setupprobes"
      ' (the tests' addresses in place of this computer's: never the user's
      ' own servers)
      AISuUrls(0) = FIELD$(Arg, "|", 1)
      AISuUrls(1) = FIELD$(Arg, "|", 2)
      AISuUrls(2) = FIELD$(Arg, "|", 3)
    CASE "setuplook": AISuLook
    CASE "setuplocal"
      AISuLocal.Checked = 1
      AISuPicked
    CASE "setupcloud"
      AISuCloudRadio.Checked = 1
      AISuPicked
    CASE "setupnext": AISuNext
    CASE "setupback": AISuBack
    CASE "setupprov"
      AISuProvBox.ItemIndex = VAL(Arg)
      AISuProvPicked
    CASE "setupkey": AISuKeyEdit.Text = Arg
    CASE "setuptest": AISuTest
    CASE "setupserver"
      AISuServerBox.ItemIndex = VAL(Arg)
      AISuServerPicked
    CASE "setupmodel"
      IF AISuPage = 2 THEN AISuLocalModel.ItemIndex = VAL(Arg) ELSE AISuCloudModel.ItemIndex = VAL(Arg)
    CASE "setupcancel": AISuCancel
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION
