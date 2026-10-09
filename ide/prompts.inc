' prompts.inc: the question Studio asks before changes would be lost, as
' macOS and Xcode ask it. One document: "Do you want to save the changes you
' made to main.rr?" — Save (Enter), Don't Save, Cancel (Escape). Quitting
' with several documents changed: Save All (Enter), Review Changes... (each
' one asked in turn), Discard Changes, Cancel (Escape). The buttons in the
' system's order: Windows' Save first; macOS' and the others' Don't Save
' apart on the left, the default on the right.

DECLARE SUB PromptPick(Sender AS QBUTTON)

CONST PROMPT_CANCEL = 0
CONST PROMPT_SAVE = 1
CONST PROMPT_DONT = 2
CONST PROMPT_REVIEW = 3

DIM PromptAnswer AS INTEGER
DIM PromptRust AS INTEGER   ' 1: the native build's question (no Rust), else the save questions
DIM PromptIconBmp AS QBITMAP
DIM PromptIconLoaded AS INTEGER

CREATE SavePrompt AS QFORM
  Caption = "RapidR Studio"
  Width = 480: Height = 200
  BorderStyle = 3
  ' (Save first: it has the keyboard when the question shows)
  CREATE PromptSave AS QBUTTON
    Caption = "&Save": Tag = 1
    Default = 1
    OnClick = PromptPick
  END CREATE
  CREATE PromptReview AS QBUTTON
    Caption = "&Review Changes...": Tag = 3
    OnClick = PromptPick
  END CREATE
  CREATE PromptDont AS QBUTTON
    Caption = "Do&n't Save": Tag = 2
    OnClick = PromptPick
  END CREATE
  CREATE PromptCancel AS QBUTTON
    Caption = "Cancel": Tag = 0
    Cancel = 1
    OnClick = PromptPick
  END CREATE
  CREATE PromptIcon AS QIMAGE
    Left = 20: Top = 20: Width = 56: Height = 56
    Stretch = 1
    AccessibleName = "RapidR Studio"
  END CREATE
  CREATE PromptTitle AS QLABEL
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE PromptText AS QLABEL
    WordWrap = 1: AutoSize = 0
  END CREATE
END CREATE

SUB PromptPick(Sender AS QBUTTON)
  PromptAnswer = Sender.Tag
  SavePrompt.Close
END SUB

' (the question's look: RapidQ's sizes in the classic look, RapidR's
' elsewhere; Many: quitting with several documents changed)
SUB LayOutPrompt(Many AS INTEGER)
  DIM pad AS INTEGER, ic AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, bh AS INTEGER, gap AS INTEGER, th AS INTEGER, tx AS INTEGER
  DIM ws AS INTEGER, wd AS INTEGER, wr AS INTEGER, wc AS INTEGER, edge AS INTEGER, onWin AS INTEGER
  IF ThemeName = "classic" THEN
    pad = 14: ic = 32: bh = 23: gap = 6: th = 32: tx = 30
    ws = 75: wd = 75: wr = 110: wc = 75
  ELSE
    pad = 20: ic = 56: bh = 32: gap = 8: th = 38: tx = 36
    ws = 96: wd = 112: wr = 150: wc = 96
  END IF
  IF Many THEN wd = wd + 24
  ' (the Rust question's buttons say more: "Build Interpreted Instead")
  IF PromptRust THEN
    ws = ws + 110: wd = wd + 10
  END IF
  SavePrompt.Font.Name = UIFont.Name: SavePrompt.Font.Size = UIFont.Size
  PromptTitle.Font = SubFont
  PromptText.Font.Color = TC("page.dim")
  ' (the icon, once: RapidR Studio's, drawn at twice the size shown)
  IF PromptIconLoaded = 0 THEN
    PromptIconBmp.LoadFromFile HomeDir + "ide/assets/rapidr-studio.svg"
    PromptIcon.BMPHandle = PromptIconBmp
    PromptIconLoaded = 1
  END IF
  x = pad + ic + 16
  ' (the text as wide as the buttons need: theirs, their gaps, and the
  ' space that sets Don't Save apart)
  w = ws + wd + wc + 2 * gap + 4 * gap
  IF Many THEN w = w + wr + gap
  IF w < 360 THEN w = 360
  PromptIcon.Left = pad: PromptIcon.Top = pad: PromptIcon.Width = ic: PromptIcon.Height = ic
  PromptTitle.Left = x: PromptTitle.Top = pad: PromptTitle.Width = w: PromptTitle.Height = th
  PromptText.Left = x: PromptText.Top = pad + th + 6: PromptText.Width = w: PromptText.Height = tx
  y = pad + th + 6 + tx + pad
  IF y < pad + ic + pad THEN y = pad + ic + pad
  PromptReview.Visible = Many
  IF PromptRust THEN
    PromptSave.Caption = "&Build Interpreted Instead"
    PromptDont.Caption = "&Install Rust..."
  ELSEIF Many THEN
    PromptSave.Caption = "Save &All"
    PromptDont.Caption = "&Discard Changes"
  ELSE
    PromptSave.Caption = "&Save"
    PromptDont.Caption = "Do&n't Save"
  END IF
  PromptSave.Width = ws: PromptDont.Width = wd: PromptReview.Width = wr: PromptCancel.Width = wc
  PromptSave.Height = bh: PromptDont.Height = bh: PromptReview.Height = bh: PromptCancel.Height = bh
  PromptSave.Top = y: PromptDont.Top = y: PromptReview.Top = y: PromptCancel.Top = y
  edge = x + w
  ' (Windows: Save, Don't Save, Cancel from the right edge leftwards in
  ' reading order; elsewhere Don't Save on its own at the left)
  onWin = (Proj.FileManager = "File Explorer")
  IF onWin THEN
    PromptCancel.Left = edge - wc
    PromptDont.Left = PromptCancel.Left - gap - wd
    IF Many THEN
      PromptReview.Left = PromptDont.Left - gap - wr
      PromptSave.Left = PromptReview.Left - gap - ws
    ELSE
      PromptSave.Left = PromptDont.Left - gap - ws
    END IF
  ELSE
    PromptSave.Left = edge - ws
    IF Many THEN
      PromptReview.Left = PromptSave.Left - gap - wr
      PromptCancel.Left = PromptReview.Left - gap - wc
    ELSE
      PromptCancel.Left = PromptSave.Left - gap - wc
    END IF
    PromptDont.Left = x
  END IF
  SavePrompt.ClientWidth = edge + pad
  SavePrompt.ClientHeight = y + bh + pad
END SUB

' Asks Question (Detail under it, dimmed): PROMPT_SAVE (Save, or Save All
' with Many), PROMPT_DONT (Don't Save / Discard Changes), PROMPT_REVIEW
' (Review Changes..., Many) or PROMPT_CANCEL (Cancel, Escape, the window
' closed).
FUNCTION AskSave(Question AS STRING, Detail AS STRING, Many AS INTEGER) AS INTEGER
  LayOutPrompt Many
  PromptTitle.Caption = Question
  PromptText.Caption = Detail
  PromptAnswer = PROMPT_CANCEL
  ' (in the middle of the screen, as an alert is)
  SavePrompt.Center
  SavePrompt.ShowModal
  Result = PromptAnswer
END FUNCTION

' Run > Build Native App without Rust: "Native builds need Rust (free)."
' "interpreted" (Build Interpreted Instead, Enter), "install" (Install Rust...)
' or "" (Cancel, Escape, the window closed).
FUNCTION AskRust AS STRING
  PromptRust = 1
  LayOutPrompt 0
  PromptTitle.Caption = "Native builds need Rust (it is free)."
  PromptText.Caption = "Rust is not installed on this computer. An interpreted app needs none and runs the same program."
  PromptAnswer = PROMPT_CANCEL
  SavePrompt.Center
  SavePrompt.ShowModal
  PromptRust = 0
  IF PromptAnswer = PROMPT_SAVE THEN
    Result = "interpreted"
  ELSEIF PromptAnswer = PROMPT_DONT THEN
    Result = "install"
  ELSE
    Result = ""
  END IF
END FUNCTION

' Whether a document has changes not saved: Studio's window says so (the
' dot in macOS' close button; a browser asks before the page is left).
SUB UpdateModified
  DIM d AS INTEGER, changed AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN changed = 1
  NEXT
  Studio.Modified = changed
END SUB
