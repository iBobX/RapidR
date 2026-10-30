' aisetup.inc: Set up RAI (docs/preview4-plan.md R6, docs/local-ai-plan.md)
' — the assistant set up in two steps, in plain words, for someone who has
' never done it:
'
'   1. Where the answers come from: free, on this computer (a local AI app
'      already running here — found; nothing is started, nothing is
'      downloaded), or a cloud service with your own key (Anthropic, OpenAI,
'      DeepSeek) — with what each costs and what it sends, said plainly.
'   2. On this computer: the models already here, each with whether it fits
'      this computer (a chip: Fits / Fits if you close other apps / Too big
'      for this computer) and one line for a student; Check My Models
'      (optional, cancellable) tries each one here and the best match says
'      so; the licence as the local AI app reports it. Only models on this
'      computer appear: RapidR never names or suggests one the user doesn't
'      have. In the cloud: the provider, the key (a key Studio already finds
'      — the keychain, the environment — can be used as it is), Test, then a
'      model, the fast, low-cost ones first.
'
' Finish turns the assistant on with those choices (a key typed is kept
' in the system's keychain, as Settings > AI keeps it: never in a project;
' a model on this computer with the context and RAI's mode its check chose)
' and shows it. Settings > AI has every detail afterwards. The models are
' looked for and checked by RPROJECT's LocalAI… members (rapidr-studio's
' local_ai.rs, on threads of their own: the dialog polls them); nothing
' reaches the assistant before Finish; Cancel leaves everything as it was.

DECLARE SUB AISuNext
DECLARE SUB AISuBack
DECLARE SUB AISuCancel
DECLARE SUB AISuClosed(Action AS INTEGER)
DECLARE SUB AISuLook
DECLARE SUB AISuPicked
DECLARE SUB AISuProvPicked
DECLARE SUB AISuTest
DECLARE SUB AISuKeyLinkClick
DECLARE SUB AISuProbe1In(Count AS INTEGER)
DECLARE SUB AISuProbe1Err(Message AS STRING)
DECLARE SUB AISuTestIn(Count AS INTEGER)
DECLARE SUB AISuTestErr(Message AS STRING)
DECLARE SUB AISuShowPage(P AS INTEGER)
DECLARE SUB AISuFound
DECLARE SUB LayOutAISetup
DECLARE SUB AISuFinish
DECLARE SUB AISuFillModels(Which AS INTEGER, Models AS STRING)
DECLARE SUB AISuFonts
DECLARE SUB AISuTick
DECLARE SUB AISuFillList
DECLARE SUB AISuDrawModel(Index AS INTEGER, State AS INTEGER, R AS RRect)
DECLARE SUB AISuModelPicked
DECLARE SUB AISuShowDetail
DECLARE SUB AISuCheckClick
DECLARE SUB AISuCheckState
DECLARE SUB AISuAddrLook
DECLARE SUB AISuLicClick
DECLARE SUB AISuLicClose
DECLARE SUB AISuNoneLinkClick
DECLARE FUNCTION AISuLM(I AS INTEGER, Field AS STRING) AS STRING
DECLARE FUNCTION AISuLine(I AS INTEGER) AS STRING
DECLARE FUNCTION AISuChip(I AS INTEGER) AS STRING
DECLARE FUNCTION AISuSay(I AS INTEGER) AS STRING
DECLARE FUNCTION AISuSecs(S AS STRING) AS STRING
DECLARE FUNCTION AISuStepWords(S AS STRING) AS STRING
DECLARE FUNCTION AISuFit(Text AS STRING, Room AS INTEGER) AS STRING

' (a web page's one probe: the address the user typed, asked for its
' models — a browser can't look around this computer by itself)
DIM AISuProbe1 AS RAI
DIM AISuWebUrl AS STRING
' (the cloud's: the key typed is tried by this one, kept only by Finish)
DIM AISuCloud AS RAI
' (the page shown: 1 where, 2 this computer, 3 the cloud)
DIM AISuPage AS INTEGER
' (the cloud providers the list shows, by id)
DIM AISuProvIds(4) AS STRING
DIM AISuProvCount AS INTEGER
' (the models' list: the model picked by its address and name, kept when
' the list is filled again; a web page; the rows shown)
DIM AISuPickedId AS STRING
DIM AISuWeb AS INTEGER
DIM AISuRows AS INTEGER
' (what the dialog polls: the models looked for and checked on threads of
' their own — RPROJECT's LocalAI…)
DIM AISuTimer AS RTimer
DIM AISuMeasure AS RBitmap

CREATE AISetup AS RForm
  Caption = TR$("Set Up RAI")
  Width = 700: Height = 600
  BorderStyle = 3
  OnClose = AISuClosed
  CREATE AISuFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE AISuBackBtn AS RButton
      Caption = TR$("< &Back")
      AccessibleName = TR$("Back")
      OnClick = AISuBack
    END CREATE
    CREATE AISuCancelBtn AS RButton
      Caption = TR$("Cancel")
      Cancel = 1
      OnClick = AISuCancel
    END CREATE
    CREATE AISuNextBtn AS RButton
      Caption = TR$("&Next >")
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
    AccessibleName = TR$("Where RAI's answers come from")
    CREATE AISuLocal AS RRadioButton
      Caption = TR$("Free, on &this computer")
      OnClick = AISuPicked
    END CREATE
    CREATE AISuLocalText AS RLabel
      AutoSize = 0: WordWrap = 1
      Caption = TR$("It costs nothing, and Studio sends what you ask only to a local AI app on this computer — a free program you install that keeps AI models on your computer — so what you ask stays here, unless that app is set up to pass it on to a service elsewhere. Studio finds the models it already has and checks which ones fit this computer. Answers are slower and simpler than a cloud model's.")
    END CREATE
    CREATE AISuFoundText AS RLabel
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("The AI models found on this computer")
    END CREATE
    CREATE AISuLookBtn AS RButton
      Caption = TR$("&Look Again")
      AccessibleName = TR$("Look again for AI models on this computer")
      OnClick = AISuLook
    END CREATE
    CREATE AISuCloudRadio AS RRadioButton
      Caption = TR$("A &cloud service, with your own key")
      OnClick = AISuPicked
    END CREATE
    CREATE AISuCloudText AS RLabel
      AutoSize = 0: WordWrap = 1
      Caption = TR$("Anthropic (Claude), OpenAI or DeepSeek: usually the best answers. You make a key on the provider's website and pay it for what you use: a question usually costs from a tenth of a cent (small, fast models) to a few cents (the largest). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to that provider, under its privacy terms. Your key stays in this computer's keychain, never in a project.")
    END CREATE
  END CREATE
  ' ---- 2: on this computer ----
  CREATE AISuPage2 AS RPanel
    Align = 5
    BevelOuter = 0
    Visible = 0
    AccessibleName = TR$("The AI models on this computer")
    CREATE AISuMemText AS RLabel
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("This computer's memory")
    END CREATE
    CREATE AISuModels AS RListBox
      Style = lbOwnerDrawFixed
      AccessibleName = TR$("The AI models on this computer, the best match first")
      AccessibleDescription = TR$("Each says whether it fits this computer and how it does with RAI. Choose one, then Finish.")
      OnDrawItem = AISuDrawModel
      OnClick = AISuModelPicked
      OnChange = AISuModelPicked
    END CREATE
    CREATE AISuNone AS RLabel
      AutoSize = 0: WordWrap = 1
      Visible = 0
    END CREATE
    CREATE AISuNoneLink AS RLabel
      Caption = TR$("How to add a local AI app")
      Cursor = -21
      Visible = 0
      AccessibleName = TR$("How to add a local AI app: the manual's page, in Help")
      OnClick = AISuNoneLinkClick
    END CREATE
    CREATE AISuDetail AS RLabel
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("About the model chosen")
    END CREATE
    CREATE AISuCheckBtn AS RButton
      Caption = TR$("&Check My Models")
      OnClick = AISuCheckClick
    END CREATE
    CREATE AISuLicBtn AS RButton
      Caption = TR$("Read the L&icence...")
      AccessibleName = TR$("Read the chosen model's licence, as its local AI app reports it")
      OnClick = AISuLicClick
    END CREATE
    CREATE AISuCheckNote AS RLabel
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("Check My Models")
    END CREATE
    CREATE AISuAddrLabel AS RLabel
      Caption = TR$("Another &address:")
    END CREATE
    CREATE AISuAddrEdit AS REdit
      AccessibleName = TR$("Another address on this computer")
      AccessibleDescription = TR$("A local AI app's address on this computer, for example 127.0.0.1:8080. Studio only asks this computer.")
    END CREATE
    CREATE AISuAddrBtn AS RButton
      Caption = TR$("Loo&k")
      AccessibleName = TR$("Look for models at this address")
      OnClick = AISuAddrLook
    END CREATE
  END CREATE
  ' ---- 3: a cloud service ----
  CREATE AISuPage3 AS RPanel
    Align = 5
    BevelOuter = 0
    Visible = 0
    AccessibleName = TR$("A cloud service")
    CREATE AISuProvLabel AS RLabel
      Caption = TR$("&Provider:")
      Left = 24: Top = 12: Width = 100
    END CREATE
    CREATE AISuProvBox AS RComboBox
      Style = 2
      Left = 130: Top = 8: Width = 300
      AccessibleName = TR$("The cloud provider")
      OnChange = AISuProvPicked
    END CREATE
    CREATE AISuKeyLink AS RLabel
      Caption = TR$("Get a key")
      Left = 444: Top = 12: Width = 150
      Cursor = -21
      AccessibleName = TR$("Get a key on the provider's website, in the browser")
      OnClick = AISuKeyLinkClick
    END CREATE
    CREATE AISuKeyLabel AS RLabel
      Caption = TR$("API &key:")
      Left = 24: Top = 48: Width = 100
    END CREATE
    CREATE AISuKeyEdit AS REdit
      PasswordChar = "*"
      Left = 130: Top = 44: Width = 300
      AccessibleName = TR$("API key")
      AccessibleDescription = TR$("Paste the key the provider gave you: it is kept in this computer's keychain, never in a project")
    END CREATE
    CREATE AISuTestBtn AS RButton
      Caption = TR$("&Test the Key")
      AccessibleName = TR$("Test the key: ask the provider for its models")
      OnClick = AISuTest
    END CREATE
    CREATE AISuTestNote AS RLabel
      Left = 130: Top = 80: Width = 460: Height = 54
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("The key's test")
    END CREATE
    CREATE AISuCloudModelLabel AS RLabel
      Caption = TR$("&Model:")
      Left = 24: Top = 144: Width = 100
    END CREATE
    CREATE AISuCloudModel AS RComboBox
      Left = 130: Top = 140: Width = 300
      AccessibleName = TR$("Model")
    END CREATE
    CREATE AISuCostText AS RLabel
      Left = 24: Top = 182: Width = 560: Height = 130
      AutoSize = 0: WordWrap = 1
      AccessibleName = TR$("What it costs, and what is sent")
    END CREATE
  END CREATE
END CREATE

' ---- a model's licence, as its local AI app reports it ----
CREATE AISuLicForm AS RForm
  Caption = TR$("Licence")
  Width = 640: Height = 520
  BorderStyle = 3
  CREATE AISuLicFoot AS RPanel
    Align = 2
    Height = 56
    BevelOuter = 0
    CREATE AISuLicNote AS RLabel
      AutoSize = 0: WordWrap = 1
    END CREATE
    CREATE AISuLicCloseBtn AS RButton
      Caption = TR$("Close")
      Default = 1: Cancel = 1
      OnClick = AISuLicClose
    END CREATE
  END CREATE
  CREATE AISuLicText AS RMemo
    Align = 5
    ReadOnly = 1
    WordWrap = 1
    ScrollBars = 2
    AccessibleName = TR$("The licence's text")
  END CREATE
END CREATE

SUB SetUpAISetup
  DIM k AS INTEGER
  AISuProbe1.OnModelsLoaded = AISuProbe1In
  AISuProbe1.OnError = AISuProbe1Err
  AISuCloud.OnModelsLoaded = AISuTestIn
  AISuCloud.OnError = AISuTestErr
  AISuTimer.Interval = 250
  AISuTimer.Enabled = 0
  AISuTimer.OnTimer = AISuTick
  AISuWeb = Flag(Proj.LocalAI("web") = "1")
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
' the models on this computer looked for at once.
SUB ShowAISetup
  AISuFonts
  AISuShowPage 1
  AISuPickedId = ""
  AISuTimer.Enabled = 1
  ' (School Lock: nothing looked for while RAI is off)
  IF LockOffAI = 0 THEN AISuLook
  IF AIProvider = "compatible" OR AIProvider = "" THEN AISuLocal.Checked = 1 ELSE AISuCloudRadio.Checked = 1
  AISuPicked
  ' (School Lock: the page read-only, its strip — schoollock.inc)
  LockSetupShow
  ' (the texts as they now say: their heights)
  LayOutAISetup
  AISetup.Center
  AISetup.ShowModal
  AISuTimer.Enabled = 0
END SUB

' (the dialog closed however: a check running stops — no model is left
' working for a dialog nobody sees)
SUB AISuClosed(Action AS INTEGER)
  AISuTimer.Enabled = 0
  IF Proj.LocalAI("checking") <> "-1" THEN Proj.LocalAICancel
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
  AISuMemText.Font = UIFont
  AISuMemText.Font.Color = TC("page.dim")
  AISuModels.Font = UIFont
  IF ThemeName = "classic" THEN AISuModels.ItemHeight = 34 ELSE AISuModels.ItemHeight = 46
  AISuNone.Font = UIFont
  AISuNone.Font.Color = TC("page.text")
  AISuNoneLink.Font = UIFont
  AISuNoneLink.Font.Color = TC("page.link")
  AISuDetail.Font = UIFont
  AISuDetail.Font.Color = TC("page.text")
  AISuCheckNote.Font = UIFont
  AISuCheckNote.Font.Color = TC("page.dim")
  AISuTestNote.Font = UIFont
  AISuTestNote.Font.Color = TC("page.text")
  AISuCostText.Font = UIFont
  AISuCostText.Font.Color = TC("page.dim")
  AISuKeyLink.Font = UIFont
  AISuKeyLink.Font.Color = TC("page.link")
  AISuLicText.Font = MonoFont
  AISuLicNote.Font = UIFont
  AISuLicNote.Font.Color = TC("page.dim")
  AISuMeasure.Font.Name = UIFont.Name: AISuMeasure.Font.Size = UIFont.Size
  LayOutAISetup
END SUB

SUB LayOutAISetup
  DIM w AS INTEGER, bh AS INTEGER, fw AS INTEGER, h AS INTEGER, dy AS INTEGER, lk AS INTEGER, y AS INTEGER, ch AS INTEGER
  w = AISetup.ClientWidth
  IF w < 400 THEN w = 400
  bh = UiButtonH
  ch = UiCompactH
  AISuTitle.Width = w - 48
  AISuNextBtn.Left = w - 24 - 110: AISuNextBtn.Top = 12: AISuNextBtn.Width = 110: AISuNextBtn.Height = bh
  AISuCancelBtn.Left = w - 24 - 110 - 8 - 96: AISuCancelBtn.Top = 12: AISuCancelBtn.Width = 96: AISuCancelBtn.Height = bh
  AISuBackBtn.Left = 24: AISuBackBtn.Top = 12: AISuBackBtn.Width = 96: AISuBackBtn.Height = bh
  ' (School Lock: its strip at the top of the first page while RAI is
  ' off, the page under it — schoollock.inc)
  IF LockOffAI THEN lk = LockStripH + 12
  LockSuPlace 24, 6, w - 48
  ' (1: the two choices, each with what it means; what was found, Look Again)
  AISuLocal.Left = 24: AISuLocal.Top = 6 + lk: AISuLocal.Width = w - 48: AISuLocal.Height = 24
  ' (each text as tall as its lines need in Studio's language — English's
  ' heights at least —, what follows it moved down by as much: a longer
  ' language isn't cut off)
  ' (the texts are in the UI's font, as Settings measures them)
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  h = AISetLinesHigh(AISuLocalText.Caption, w - 72)
  IF h < 78 THEN h = 78
  AISuLocalText.Left = 48: AISuLocalText.Top = 32 + lk: AISuLocalText.Width = w - 72: AISuLocalText.Height = h
  dy = h - 78 + lk
  h = AISetLinesHigh(AISuFoundText.Caption, w - 72 - 132)
  IF h < 44 THEN h = 44
  AISuFoundText.Left = 48: AISuFoundText.Top = 114 + dy: AISuFoundText.Width = w - 72 - 132: AISuFoundText.Height = h
  AISuLookBtn.Left = w - 24 - 120: AISuLookBtn.Top = 114 + dy: AISuLookBtn.Width = 120: AISuLookBtn.Height = bh
  dy = dy + h - 44
  AISuCloudRadio.Left = 24: AISuCloudRadio.Top = 174 + dy: AISuCloudRadio.Width = w - 48: AISuCloudRadio.Height = 24
  h = AISetLinesHigh(AISuCloudText.Caption, w - 72)
  IF h < 100 THEN h = 100
  AISuCloudText.Left = 48: AISuCloudText.Top = 200 + dy: AISuCloudText.Width = w - 72: AISuCloudText.Height = h
  ' (2: the memory, the list — or why none is there —, the chosen model,
  ' Check My Models and the licence, another address; 24 from the edges,
  ' 8 between, 16 between groups)
  y = 8
  h = AISetLinesHigh(AISuMemText.Caption, w - 48)
  IF h < 20 THEN h = 20
  AISuMemText.Left = 24: AISuMemText.Top = y: AISuMemText.Width = w - 48: AISuMemText.Height = h
  y = y + h + 8
  AISuModels.Left = 24: AISuModels.Top = y: AISuModels.Width = w - 48: AISuModels.Height = 4 * AISuModels.ItemHeight + 4
  h = AISetLinesHigh(AISuNone.Caption, w - 48)
  AISuNone.Left = 24: AISuNone.Top = y: AISuNone.Width = w - 48: AISuNone.Height = h
  AISuNoneLink.Left = 24: AISuNoneLink.Top = y + h + 8: AISuNoneLink.Width = AISetMeasure.TextWidth(AISuNoneLink.Caption) + 8: AISuNoneLink.Height = 22
  y = y + AISuModels.Height + 8
  h = AISetLinesHigh(AISuDetail.Caption, w - 48)
  IF h < 58 THEN h = 58
  AISuDetail.Left = 24: AISuDetail.Top = y: AISuDetail.Width = w - 48: AISuDetail.Height = h
  y = y + h + 8
  AISuCheckBtn.Left = 24: AISuCheckBtn.Top = y: AISuCheckBtn.Width = AISetMeasure.TextWidth(TR$("Stop Checking")) + 40: AISuCheckBtn.Height = bh
  IF AISuCheckBtn.Width < AISetMeasure.TextWidth(TR$("Check My Models")) + 40 THEN AISuCheckBtn.Width = AISetMeasure.TextWidth(TR$("Check My Models")) + 40
  AISuLicBtn.Width = AISetMeasure.TextWidth(TR$("Read the Licence...")) + 40
  AISuLicBtn.Left = w - 24 - AISuLicBtn.Width: AISuLicBtn.Top = y: AISuLicBtn.Height = bh
  y = y + bh + 8
  h = AISetLinesHigh(AISuCheckNote.Caption, w - 48)
  IF h < 40 THEN h = 40
  AISuCheckNote.Left = 24: AISuCheckNote.Top = y: AISuCheckNote.Width = w - 48: AISuCheckNote.Height = h
  y = y + h + 16
  AISuAddrLabel.Left = 24: AISuAddrLabel.Top = y + 3: AISuAddrLabel.Width = 132: AISuAddrLabel.Height = 20
  AISuAddrEdit.Left = 160: AISuAddrEdit.Top = y: AISuAddrEdit.Width = w - 160 - 24 - 104
  AISuAddrBtn.Left = w - 24 - 96: AISuAddrBtn.Top = y + (AISuAddrEdit.Height - ch) \ 2: AISuAddrBtn.Width = 96: AISuAddrBtn.Height = ch
  ' (the page as tall as its texts need in Studio's language: the dialog
  ' grows, never shrinks while it shows — nothing is cut off)
  IF AISuPage = 2 THEN
    h = y + AISuAddrEdit.Height + 16 + AISuHead.Height + AISuFoot.Height
    IF AISetup.ClientHeight < h THEN AISetup.Height = AISetup.Height + h - AISetup.ClientHeight
  END IF
  ' (3: a label column, the fields, notes under them)
  fw = w - 130 - 24 - 160
  AISuProvBox.Width = fw: AISuKeyEdit.Width = fw: AISuCloudModel.Width = fw
  AISuKeyLink.Left = 130 + fw + 14: AISuKeyLink.Width = 150
  AISuTestBtn.Left = 130 + fw + 8: AISuTestBtn.Top = 42: AISuTestBtn.Width = 150: AISuTestBtn.Height = bh
  h = AISetLinesHigh(AISuTestNote.Caption, w - 130 - 24)
  IF h < 54 THEN h = 54
  AISuTestNote.Width = w - 130 - 24: AISuTestNote.Height = h
  h = AISetLinesHigh(AISuCostText.Caption, w - 48)
  IF h < 130 THEN h = 130
  AISuCostText.Width = w - 48: AISuCostText.Height = h
END SUB

SUB AISuShowPage(P AS INTEGER)
  ' (School Lock: never past the first page while RAI is off)
  IF P > 1 AND LockOffAI THEN P = 1
  AISuPage = P
  AISuPage1.Visible = Flag(P = 1)
  AISuPage2.Visible = Flag(P = 2)
  AISuPage3.Visible = Flag(P = 3)
  AISuBackBtn.Visible = Flag(P > 1)
  SELECT CASE P
    CASE 1
      AISuTitle.Caption = TR$("Where should RAI's answers come from?")
      AISuNextBtn.Caption = TR$("&Next >")
    CASE 2
      AISuTitle.Caption = TR$("RAI on this computer")
      AISuNextBtn.Caption = TR$("&Finish")
    CASE 3
      AISuTitle.Caption = TR$("RAI from a cloud service")
      AISuNextBtn.Caption = TR$("&Finish")
  END SELECT
  AISuTitle.AccessibleName = AISuTitle.Caption
  AISetup.Caption = TR$("Set Up RAI — {title}", "title", AISuTitle.Caption)
END SUB

' (the choice on page 1: what was found says so; Next goes on)
SUB AISuPicked
  AISuFound
END SUB

' ---- this computer: the models looked for ------------------------------------------

' A model's field, the best match first (rapidr-studio's local_ai.rs).
FUNCTION AISuLM(I AS INTEGER, Field AS STRING) AS STRING
  Result = Proj.LocalAIModel(I, Field)
END FUNCTION

' The models looked for: on the desktop the servers on this computer at
' their usual ports and the address typed (a GET each: nothing is started
' or loaded); a web page asks the address typed only.
SUB AISuLook
  IF AISuWeb THEN
    AISuAddrLook
  ELSE
    IF Proj.LocalAIFind("") = 0 THEN AISuCheckNote.Caption = Proj.Error
  END IF
  AISuFound
END SUB

' (the timer: what the threads found or checked since, shown)
SUB AISuTick
  IF Proj.LocalAI("news") <> "1" THEN EXIT SUB
  AISuFound
  AISuFillList
  AISuCheckState
END SUB

' What was found, in words, on page 1 ("✓ Found 9 models in a local AI app
' on this computer.").
SUB AISuFound
  DIM n AS INTEGER, k AS INTEGER
  n = VAL(Proj.LocalAI("chatcount"))
  k = VAL(Proj.LocalAI("servers"))
  IF LockOffAI THEN EXIT SUB
  IF Proj.LocalAI("looking") = "1" THEN
    AISuFoundText.Caption = TR$("Looking for AI apps on this computer…")
  ELSEIF n > 0 AND k > 1 THEN
    AISuFoundText.Caption = TRN$("✓ Found {n} model in {k} local AI apps on this computer.", "✓ Found {n} models in {k} local AI apps on this computer.", n, "k", LTRIM$(STR$(k)))
  ELSEIF n > 0 THEN
    AISuFoundText.Caption = TRN$("✓ Found {n} model in a local AI app on this computer.", "✓ Found {n} models in a local AI app on this computer.", n)
  ELSEIF AISuWeb THEN
    AISuFoundText.Caption = TR$("In a browser Studio can't look around this computer: Next asks for your local AI app's address.")
  ELSEIF k > 0 THEN
    AISuFoundText.Caption = TR$("A local AI app answers on this computer, but it has no model yet: add one in it, then Look Again.")
  ELSE
    AISuFoundText.Caption = TR$("No local AI app found running on this computer: Next says what one is and how to add one.")
  END IF
  AISuFoundText.AccessibleDescription = AISuFoundText.Caption
  IF AISetup.Visible THEN LayOutAISetup
END SUB

' (seconds as a student reads them: "about 4", at least 1)
FUNCTION AISuSecs(S AS STRING) AS STRING
  DIM v AS DOUBLE
  v = VAL(S)
  IF v < 1 THEN v = 1
  Result = LTRIM$(STR$(INT(v + 0.5)))
END FUNCTION

FUNCTION AISuStepWords(S AS STRING) AS STRING
  SELECT CASE S
    CASE "speed": Result = TR$("timing an answer")
    CASE "tools": Result = TR$("trying a tool call")
    CASE "task": Result = TR$("building a tiny app")
    CASE ELSE: Result = TR$("loading it")
  END SELECT
END FUNCTION

' A model's chip: whether it fits this computer.
FUNCTION AISuChip(I AS INTEGER) AS STRING
  IF AISuLM(I, "remote") = "1" THEN
    Result = TR$("Runs elsewhere")
    EXIT FUNCTION
  END IF
  SELECT CASE AISuLM(I, "fit")
    CASE "fits": Result = TR$("Fits")
    CASE "close": Result = TR$("Fits if you close other apps")
    CASE "toobig": Result = TR$("Too big for this computer")
    CASE ELSE: Result = TR$("Size not known")
  END SELECT
END FUNCTION

' A model's line for a student: what it can do here.
FUNCTION AISuLine(I AS INTEGER) AS STRING
  DIM f AS STRING, s AS STRING, k AS STRING
  f = AISuLM(I, "fit")
  s = AISuSecs(AISuLM(I, "first"))
  k = LTRIM$(STR$(VAL(AISuLM(I, "context")) \ 1024))
  IF AISuLM(I, "checking") = "1" THEN
    Result = TR$("Checking it now: {step}…", "step", AISuStepWords(AISuLM(I, "step")))
  ELSEIF AISuLM(I, "queued") = "1" THEN
    Result = TR$("Waiting for its check…")
  ELSEIF AISuLM(I, "chat") = "0" THEN
    Result = TR$("Can't chat: it only turns text into numbers for searching.")
  ELSEIF AISuLM(I, "remote") = "1" THEN
    Result = TR$("Its local AI app runs this model on a service elsewhere: what you ask is sent there.")
  ELSEIF f = "toobig" THEN
    Result = TR$("Too big for this computer: it needs {need}; a model can have {most} here at most.", "need", AISuLM(I, "needwords"), "most", Proj.LocalAI("mostwords"))
  ELSEIF AISuLM(I, "error") <> "" THEN
    Result = TR$("Its check couldn't finish: {why}", "why", AISuLM(I, "error"))
  ELSEIF AISuLM(I, "checked") = "1" THEN
    IF AISuLM(I, "toolsok") = "0" THEN
      Result = TR$("Can chat, but can't build apps with RAI: its tool calls didn't work here.")
    ELSEIF AISuLM(I, "task") = "1" THEN
      IF AISuLM(I, "fast") = "1" AND AISuLM(I, "best") = "1" THEN
        Result = TR$("Best match for this computer: it built Studio's test app, answers in about {s} s.", "s", s)
      ELSEIF AISuLM(I, "fast") = "1" THEN
        Result = TR$("It built Studio's test app, answers in about {s} s.", "s", s)
      ELSE
        Result = TR$("Works, but slowly here (about {s} s per answer).", "s", s)
      END IF
    ELSE
      Result = TR$("Uses RAI's tools, but couldn't build the test app here: RAI starts its programs from RapidR's recipes.")
    END IF
  ELSEIF AISuLM(I, "tools") = "0" THEN
    Result = TR$("Its local AI app says it takes no tools, so RAI can't build apps with it.")
  ELSEIF f = "unknown" THEN
    Result = TR$("Its size isn't known: Check My Models to see how it does here.")
  ELSEIF AISuLM(I, "best") = "1" THEN
    Result = TR$("Best match for this computer by its size: Check My Models to see how it does.")
  ELSE
    IF f = "close" THEN Result = TR$("Needs {need}: close other apps, Look Again, then Check My Models.", "need", AISuLM(I, "needwords")) ELSE Result = TR$("Needs {need} with a {k}K context. Not checked yet.", "need", AISuLM(I, "needwords"), "k", k)
  END IF
END FUNCTION

' A row as a screen reader says it: the model, its chip, its line.
FUNCTION AISuSay(I AS INTEGER) AS STRING
  Result = AISuLM(I, "id") + ". " + AISuChip(I) + ". " + AISuLine(I)
END FUNCTION

' (the list again: the rows as they now say, the model picked kept)
SUB AISuFillList
  DIM n AS INTEGER, i AS INTEGER, pick AS INTEGER, best AS INTEGER
  n = VAL(Proj.LocalAI("count"))
  AISuRows = n
  AISuModels.Clear
  pick = -1
  FOR i = 0 TO n - 1
    AISuModels.AddItems AISuSay(i)
    IF AISuPickedId <> "" AND AISuLM(i, "base") + "|" + AISuLM(i, "id") = AISuPickedId THEN pick = i
  NEXT
  ' (none picked yet: the best match)
  best = VAL(Proj.LocalAI("best"))
  IF pick < 0 AND best >= 0 AND best < n THEN pick = best
  IF pick < 0 AND n > 0 THEN pick = 0
  IF pick >= 0 THEN AISuModels.ItemIndex = pick
  ' (none here: what a local AI app is, and how to add one)
  AISuModels.Visible = Flag(n > 0)
  AISuNone.Visible = Flag(n = 0)
  AISuNoneLink.Visible = Flag(n = 0)
  IF AISuWeb THEN
    AISuMemText.Caption = TR$("A browser can't tell how much memory this computer has free, so a model's fit can't be checked here: Studio on the desktop can.")
    AISuNone.Caption = TR$("Type your local AI app's address on this computer (for example http://127.0.0.1:11434), then Look. It must allow this page to ask it: the manual says how.")
  ELSE
    IF Proj.LocalAI("totalwords") <> "" THEN AISuMemText.Caption = TR$("This computer: {total} of memory; {free} free for a model now ({most} with other apps closed).", "total", Proj.LocalAI("totalwords"), "free", Proj.LocalAI("freewords"), "most", Proj.LocalAI("mostwords"))
    IF Proj.LocalAI("looking") = "1" THEN
      AISuNone.Caption = TR$("Looking for AI apps on this computer…")
    ELSE
      AISuNone.Caption = TR$("No AI model answered on this computer. A local AI app is a free program you install: it keeps AI models on your computer and answers RAI here, so what you ask stays on this computer, unless the app is set up to pass it on. Install one, add a model in it, start it, then go Back and Look Again — or type its address below if it uses another one.")
    END IF
  END IF
  AISuMemText.AccessibleDescription = AISuMemText.Caption
  AISuNone.AccessibleName = AISuNone.Caption
  AISuShowDetail
  LayOutAISetup
END SUB

' (text cut to fit Room pixels, "…" at its end)
FUNCTION AISuFit(Text AS STRING, Room AS INTEGER) AS STRING
  DIM k AS INTEGER
  IF AISuMeasure.TextWidth(Text) <= Room OR LEN(Text) < 4 THEN
    Result = Text
    EXIT FUNCTION
  END IF
  FOR k = LEN(Text) - 1 TO 1 STEP -1
    IF AISuMeasure.TextWidth(LEFT$(Text, k) + "…") <= Room THEN EXIT FOR
  NEXT
  Result = LEFT$(Text, k) + "…"
END FUNCTION

' A row: the model's name and its chip (Fits: success; Fits if you close
' other apps: warning; Too big: plain, a grey border; not known: info), its
' line under them in the dim colour.
SUB AISuDrawModel(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, dimc AS LONG, chip AS STRING, k AS STRING, cw AS INTEGER, cx AS INTEGER, cy AS INTEGER, chh AS INTEGER, sel AS INTEGER, fit AS STRING, fill AS LONG, edge AS LONG, ink AS LONG, line1 AS INTEGER, line2 AS INTEGER
  IF Index < 0 OR Index >= AISuRows THEN EXIT SUB
  sel = Flag(Index = AISuModels.ItemIndex)
  IF sel THEN
    bg = TC("selected"): fg = TC("selected.text"): dimc = fg
  ELSE
    bg = TC("window"): fg = TC("text"): dimc = TC("page.dim")
  END IF
  AISuModels.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  ' (the chip, at the right of the first line)
  fit = AISuLM(Index, "fit")
  chip = AISuChip(Index)
  IF AISuLM(Index, "remote") = "1" THEN fit = "remote"
  SELECT CASE fit
    CASE "fits": k = "status.success"
    CASE "close": k = "status.warning"
    CASE "toobig": k = ""
    CASE ELSE: k = "status.info"
  END SELECT
  chh = 20
  IF ThemeName = "classic" THEN chh = 16
  cw = AISuMeasure.TextWidth(chip) + 20
  cx = R.Right - 10 - cw
  cy = R.Top + 4
  IF k = "" THEN
    fill = bg: edge = dimc: ink = dimc
  ELSE
    fill = TC(k): edge = TC(k + ".border"): ink = TC(k + ".text")
  END IF
  AISuModels.RoundRect cx, cy, cx + cw, cy + chh, chh, chh, edge
  AISuModels.RoundRect cx + 1, cy + 1, cx + cw - 1, cy + chh - 1, chh - 2, chh - 2, fill
  AISuModels.TextOut cx + 10, cy + (chh - AISuMeasure.TextHeight("A")) \ 2, chip, ink, -1
  ' (the name, then the line under it)
  IF ThemeName = "classic" THEN
    line1 = R.Top + 2: line2 = R.Top + 17
  ELSE
    line1 = R.Top + 5: line2 = R.Top + 25
  END IF
  AISuModels.TextOut R.Left + 10, line1, AISuFit(AISuLM(Index, "id"), cx - R.Left - 20), fg, -1
  AISuModels.TextOut R.Left + 10, line2, AISuFit(AISuLine(Index), R.Right - R.Left - 20), dimc, -1
END SUB

SUB AISuModelPicked
  DIM k AS INTEGER
  k = AISuModels.ItemIndex
  IF k >= 0 AND k < AISuRows THEN AISuPickedId = AISuLM(k, "base") + "|" + AISuLM(k, "id")
  AISuShowDetail
  AISuModels.Repaint
END SUB

' The model chosen, in a few lines: what its app reports, the context RAI
' gets, its licence as reported — and the note for apps to sell.
SUB AISuShowDetail
  DIM k AS INTEGER, t AS STRING, part AS STRING, lic AS STRING, ctx AS STRING
  k = AISuModels.ItemIndex
  IF k < 0 OR k >= AISuRows THEN
    AISuDetail.Caption = ""
    AISuLicBtn.Enabled = 0
    AISuDetail.AccessibleDescription = ""
    EXIT SUB
  END IF
  ' (the row's line whole — the list may cut it — then what its app reports)
  t = AISuLine(k) + CHR$(10) + AISuLM(k, "id")
  part = AISuLM(k, "sizewords")
  IF part <> "" THEN t = t + " · " + part
  part = AISuLM(k, "params")
  IF part <> "" THEN t = t + " · " + TR$("{n} parameters", "n", part)
  part = AISuLM(k, "quant")
  IF part <> "" THEN t = t + " · " + part
  IF AISuLM(k, "tools") = "1" THEN t = t + " · " + TR$("takes tools")
  IF AISuLM(k, "tools") = "0" THEN t = t + " · " + TR$("no tools")
  ctx = LTRIM$(STR$(VAL(AISuLM(k, "context")) \ 1024)) + "K"
  IF AISuLM(k, "chat") = "1" AND AISuLM(k, "fit") <> "toobig" THEN
    t = t + " · " + TR$("RAI gets a {k} context", "k", ctx)
    IF AISuLM(k, "mode") = "small" THEN t = t + " " + TR$("and its small-model mode (shorter instructions, a recipe first)")
  END IF
  t = t + "." + CHR$(10)
  lic = AISuLM(k, "licence")
  IF lic <> "" THEN
    t = t + TR$("Licence, as its local AI app reports it (Studio doesn't check it; it can be wrong or out of date): {licence}.", "licence", lic) + " "
  ELSE
    t = t + TR$("Its local AI app doesn't say its licence: check it where you got the model.") + " "
  END IF
  t = t + TR$("Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.")
  AISuDetail.Caption = t
  AISuDetail.AccessibleDescription = t
  AISuLicBtn.Enabled = Flag(AISuLM(k, "licencetext") <> "")
END SUB

' Check My Models (or Stop Checking while it runs): every model that fits
' and isn't checked, one at a time, on a thread of Studio's own.
SUB AISuCheckClick
  DIM n AS INTEGER
  IF Proj.LocalAI("checking") <> "-1" THEN
    Proj.LocalAICancel
    AISuCheckNote.Caption = TR$("Stopping the check…")
    EXIT SUB
  END IF
  n = Proj.LocalAICheck(-1)
  IF n = 0 THEN
    IF Proj.Error <> "" THEN AISuCheckNote.Caption = Proj.Error ELSE AISuCheckNote.Caption = TR$("Every model that fits is checked already.")
  END IF
  AISuCheckState
END SUB

' (the check's button and its note: what it does, how long, where it is)
SUB AISuCheckState
  DIM n AS INTEGER, m AS INTEGER, k AS INTEGER, done AS INTEGER, q AS INTEGER, say AS STRING
  k = VAL(Proj.LocalAI("checking"))
  IF AISuWeb THEN
    AISuCheckBtn.Enabled = 0
    AISuCheckNote.Caption = TR$("Checking models runs in RapidR Studio on the desktop.")
  ELSEIF k >= 0 AND Proj.LocalAI("stopping") = "1" THEN
    AISuCheckBtn.Caption = TR$("&Stop Checking")
    AISuCheckBtn.Enabled = 0
    AISuCheckNote.Caption = TR$("Stopping the check…")
  ELSEIF k >= 0 THEN
    AISuCheckBtn.Caption = TR$("&Stop Checking")
    AISuCheckBtn.Enabled = 1
    done = VAL(Proj.LocalAI("done"))
    q = VAL(Proj.LocalAI("queue"))
    say = TR$("Checking {model} ({k} of {n}): {step}…", "model", AISuLM(k, "id"), "k", LTRIM$(STR$(done + 1)), "n", LTRIM$(STR$(q)), "step", AISuStepWords(Proj.LocalAI("step")))
    IF say <> AISuCheckNote.Caption THEN Application.Announce say
    AISuCheckNote.Caption = say
  ELSE
    AISuCheckBtn.Caption = TR$("&Check My Models")
    n = VAL(Proj.LocalAI("unchecked"))
    AISuCheckBtn.Enabled = Flag(n > 0)
    m = (VAL(Proj.LocalAI("estimate")) + 59) \ 60
    IF n > 0 AND Proj.LocalAI("stopped") = "1" THEN
      AISuCheckNote.Caption = TRN$("The check stopped. {n} model isn't checked yet: Check My Models goes on with it.", "The check stopped. {n} models aren't checked yet: Check My Models goes on with them.", n)
    ELSEIF n > 0 THEN
      AISuCheckNote.Caption = TRN$("Optional, about {n} minute: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.", "Optional, about {n} minutes: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.", m)
    ELSEIF VAL(Proj.LocalAI("count")) > 0 THEN
      AISuCheckNote.Caption = TR$("Every model that fits is checked. Choose one, then Finish.")
    ELSE
      AISuCheckNote.Caption = ""
    END IF
  END IF
  AISuCheckBtn.AccessibleDescription = AISuCheckNote.Caption
  AISuCheckNote.AccessibleDescription = AISuCheckNote.Caption
END SUB

' Look at the address typed (this computer's only: anything else is
' refused before anything is asked).
SUB AISuAddrLook
  DIM a AS STRING
  a = LTRIM$(RTRIM$(AISuAddrEdit.Text))
  IF AISuWeb THEN
    IF a = "" THEN
      AISuFillList
      EXIT SUB
    END IF
    ' (this computer's only: checked before anything is sent)
    a = Proj.LocalAIAddress(a)
    IF a = "" THEN
      AISuCheckNote.Caption = TR$("Studio only asks this computer: {why}", "why", Proj.Error)
      AISuCheckNote.AccessibleDescription = AISuCheckNote.Caption
      Application.Announce AISuCheckNote.Caption
      AISuAddrEdit.SetFocus
      EXIT SUB
    END IF
    AISuWebUrl = a
    AISuProbe1.Provider = "compatible"
    AISuProbe1.BaseURL = a
    AISuCheckNote.Caption = TR$("Asking {url}…", "url", a)
    IF AISuProbe1.RefreshModels = 0 THEN AISuProbe1Err AISuProbe1.Error
    EXIT SUB
  END IF
  IF Proj.LocalAIFind(a) = 0 THEN
    AISuCheckNote.Caption = TR$("Studio only asks this computer: {why}", "why", Proj.Error)
    AISuCheckNote.AccessibleDescription = AISuCheckNote.Caption
    Application.Announce AISuCheckNote.Caption
    AISuAddrEdit.SetFocus
  END IF
END SUB

' (a web page: the address typed answered with its models)
SUB AISuProbe1In(Count AS INTEGER)
  IF Proj.LocalAIAddModels(AISuWebUrl, AISuProbe1.Models) = 0 AND Proj.Error <> "" THEN AISuCheckNote.Caption = Proj.Error
  AISuFillList
  AISuFound
END SUB

SUB AISuProbe1Err(Message AS STRING)
  AISuCheckNote.Caption = TR$("Nothing answered at {url}: {why}", "url", AISuWebUrl, "why", Message)
  AISuCheckNote.AccessibleDescription = AISuCheckNote.Caption
  Application.Announce AISuCheckNote.Caption
END SUB

' (Read the Licence…: the text as the local AI app reports it)
SUB AISuLicClick
  DIM k AS INTEGER
  k = AISuModels.ItemIndex
  IF k < 0 OR k >= AISuRows THEN EXIT SUB
  AISuLicForm.Caption = TR$("Licence of {model}", "model", AISuLM(k, "id"))
  AISuLicText.Text = AISuLM(k, "licencetext")
  AISuLicNote.Caption = TR$("As its local AI app reports it (Studio doesn't check it). Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.")
  AISuLicNote.Left = 24: AISuLicNote.Top = 10: AISuLicNote.Width = AISuLicForm.ClientWidth - 24 - 24 - 96 - 16: AISuLicNote.Height = 40
  AISuLicCloseBtn.Left = AISuLicForm.ClientWidth - 24 - 96: AISuLicCloseBtn.Top = 12: AISuLicCloseBtn.Width = 96: AISuLicCloseBtn.Height = UiButtonH
  AISuLicForm.Center
  AISuLicForm.ShowModal
END SUB

SUB AISuLicClose
  AISuLicForm.Close
END SUB

SUB AISuNoneLinkClick
  HelpShowManual "local-ai.md"
END SUB

' ---- the pages ------------------------------------------------------------------

SUB AISuNext
  DIM k AS INTEGER
  SELECT CASE AISuPage
    CASE 1
      IF AISuLocal.Checked THEN
        AISuShowPage 2
        AISuFillList
        AISuCheckState
        IF AISuModels.Visible THEN AISuModels.SetFocus ELSE AISuAddrEdit.SetFocus
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
    AISuTestNote.Caption = TR$("A key for {name} is already found ({hint}, {store}): Test it, or paste another.", "name", name, "hint", AISuCloud.KeyHint, "store", AISuCloud.KeyStore)
  ELSE
    AISuTestNote.Caption = TR$("Paste the key {name} gave you (Get a key opens its page), then Test the Key.", "name", name)
  END IF
  AISuTestNote.AccessibleDescription = AISuTestNote.Caption
  SELECT CASE id
    CASE "anthropic"
      AISuCostText.Caption = TR$("Claude's small, fast models (Haiku) cost about a tenth of a cent a question; the largest (Opus) a few cents. Anthropic charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to Anthropic, under its privacy terms." + " Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.")
    CASE "openai"
      AISuCostText.Caption = TR$("OpenAI's small models (mini, nano) cost a fraction of a cent a question; the largest a few cents. OpenAI charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to OpenAI, under its privacy terms." + " Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.")
    CASE "deepseek"
      AISuCostText.Caption = TR$("DeepSeek's models cost a fraction of a cent a question. DeepSeek charges your account for what you use (you pay in advance on its website). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to DeepSeek, in China, under its privacy terms." + " Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.")
    CASE ELSE
      AISuCostText.Caption = TR$("You pay the provider for what you use. What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms." + " Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.")
  END SELECT
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
    AISuTestNote.Caption = TR$("Paste your API key first: the provider lists its models only to a key it knows.")
    AISuKeyEdit.SetFocus
    EXIT SUB
  END IF
  AISuTestBtn.Enabled = 0
  AISuTestNote.Caption = TR$("Testing the key…")
  IF AISuCloud.RefreshModels = 0 THEN AISuTestErr AISuCloud.Error
END SUB

SUB AISuTestIn(Count AS INTEGER)
  AISuTestBtn.Enabled = 1
  AISuFillModels 3, AISuCloud.Models
  IF Count = 0 THEN
    AISuTestNote.Caption = TR$("The key works, but it can chat with no model: check it on the provider's website.")
  ELSE
    AISuTestNote.Caption = TRN$("✓ The key works: {n} model. The first, {model}, is fast and low-cost: a good start. Finish when you have chosen.", "✓ The key works: {n} models. The first, {model}, is fast and low-cost: a good start. Finish when you have chosen.", Count, "model", AIModelId(AISuCloudModel.Text))
    AISuCloudModel.SetFocus
  END IF
  AISuTestNote.AccessibleDescription = AISuTestNote.Caption
END SUB

SUB AISuTestErr(Message AS STRING)
  AISuTestBtn.Enabled = 1
  AISuTestNote.Caption = Message
  AISuTestNote.AccessibleDescription = Message
END SUB

' (a cloud models list into its box: the fast, low-cost ones first, the
' first one chosen — the user picks another)
SUB AISuFillModels(Which AS INTEGER, Models AS STRING)
  DIM i AS INTEGER, line AS STRING, n AS INTEGER
  AISuCloudModel.Clear
  IF Models <> "" THEN
    FOR i = 1 TO TALLY(Models, CHR$(10)) + 1
      line = FIELD$(Models, CHR$(10), i)
      IF FIELD$(line, CHR$(9), 1) <> "" THEN
        AISuCloudModel.AddItems AIModelItem(line)
        n = n + 1
      END IF
    NEXT
  END IF
  IF n > 0 THEN AISuCloudModel.ItemIndex = 0 ELSE AISuCloudModel.Text = ""
END SUB

' Finish: the assistant turned on with the choices; a key typed kept in the
' keychain (as Settings > AI's Save Key); a model on this computer with the
' context and the mode its check chose; its pane shown.
SUB AISuFinish
  DIM model AS STRING, key AS STRING, k AS INTEGER, id AS STRING
  IF AISuPage = 2 THEN
    k = AISuModels.ItemIndex
    IF k < 0 OR k >= AISuRows THEN k = VAL(Proj.LocalAI("best"))
    IF k < 0 OR k >= AISuRows THEN
      AISuCheckNote.Caption = TR$("Choose a model first: Studio lists the ones on this computer. Start your local AI app and Look Again if the list is empty.")
      Application.Announce AISuCheckNote.Caption
      IF AISuModels.Visible THEN AISuModels.SetFocus ELSE AISuAddrEdit.SetFocus
      EXIT SUB
    END IF
    model = AISuLM(k, "id")
    IF AISuLM(k, "remote") = "1" THEN
      AISuCheckNote.Caption = TR$("{model} runs on a service elsewhere, not on this computer: choose a model that runs here, or set that service up as a cloud service.", "model", model)
      Application.Announce AISuCheckNote.Caption
      AISuModels.SetFocus
      EXIT SUB
    END IF
    IF AISuLM(k, "fit") = "toobig" OR AISuLM(k, "chat") = "0" THEN
      IF AISuLM(k, "chat") = "0" THEN AISuCheckNote.Caption = TR$("{model} can't chat: choose another model.", "model", model) ELSE AISuCheckNote.Caption = TR$("{model} is too big for this computer: choose another model.", "model", model)
      Application.Announce AISuCheckNote.Caption
      AISuModels.SetFocus
      EXIT SUB
    END IF
    ' (a check running stops: the model chosen is the one used now)
    IF Proj.LocalAI("checking") <> "-1" THEN Proj.LocalAICancel
    AIProvider = "compatible"
    AIBase = AISuLM(k, "base")
    AISetModelOf "compatible", model
    AILocalPick = AIBase + "|" + model + "|" + AISuLM(k, "context") + "|" + AISuLM(k, "mode")
  ELSE
    k = AISuProvBox.ItemIndex
    IF k < 0 THEN EXIT SUB
    id = AISuProvIds(k)
    model = AIModelId(AISuCloudModel.Text)
    IF model = "" THEN
      AISuTestNote.Caption = TR$("Test the key first: then choose one of the provider's models.")
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
    AIStatus.SimpleText = TR$("RAI is ready: {model}.", "model", AI.Model)
  ELSE
    SHOWMESSAGE TR$("RAI isn't ready yet: {why}", "why", AIEmptyText)
  END IF
END SUB

' (tests and the manual's scenes: ai:setup, ai:setupprobes=url|url|url (the
' servers asked in place of this computer's usual ports — never the
' user's own), ai:setupfacts=total=…,free=… (the computer's memory, MB),
' ai:setuplocal, ai:setupcloud, ai:setupnext, ai:setupback,
' ai:setupprov=N, ai:setupkey=fake, ai:setuptest, ai:setupmodel=N,
' ai:setupaddr=url, ai:setuplookaddr, ai:setupcheck, ai:setuplicence,
' ai:setuplicclose, ai:setuprefresh, ai:setupcancel; True when Verb was
' one — never a real key: the tests type a made-up one)
FUNCTION AISetupStep(Verb AS STRING, Arg AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Verb
    CASE "setup": ShowAISetup
    CASE "setupprobes"
      Proj.LocalAITest("addresses", Arg)
      ' (a web page: the first one as the address typed)
      IF AISuWeb THEN AISuAddrEdit.Text = FIELD$(Arg, "|", 1)
    CASE "setupfacts": Proj.LocalAITest("facts", Arg)
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
    CASE "setupmodel"
      IF AISuPage = 2 THEN
        AISuModels.ItemIndex = VAL(Arg)
        AISuModelPicked
      ELSE
        AISuCloudModel.ItemIndex = VAL(Arg)
      END IF
    CASE "setupaddr": AISuAddrEdit.Text = Arg
    CASE "setuplookaddr": AISuAddrLook
    CASE "setupcheck": AISuCheckClick
    CASE "setuplicence": AISuLicClick
    CASE "setuplicclose": AISuLicClose
    CASE "setuprefresh": AISuTick
    CASE "setupcancel": AISuCancel
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION
