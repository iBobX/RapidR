' consent.inc: a question that asks for the person's consent before
' something with consequences is turned on (docs/design-guidelines.md,
' "Confirmations and consent"): its title a short question, one lead line,
' a few rows that can be scanned — a 16-pixel icon, a short bold head, a
' line or two —, a footer, a Learn more link, and buttons that say what
' they do, the safe one the default. Never a wall of text in a message box.
'
'   ConsentBegin "Let AI apps use RapidR Studio?", "An AI app … can then:"
'   ConsentRow "visible", "It can see", "Your open project's files …"
'   …
'   IF ConsentAsk("Turn this off to …", "https://…", "&Turn On", "Cancel") THEN …
'
' InfoRowLayOut places one such row anywhere (Settings > AI Apps uses it
' for the same rows); OpenWebLink opens a page in the browser.

DECLARE SUB ConsentPick(Sender AS RButton)
DECLARE SUB ConsentLinkClick
DECLARE SUB ConsentBegin(Title AS STRING, Lead AS STRING)
DECLARE SUB ConsentRow(Icon AS STRING, Head AS STRING, Body AS STRING)
DECLARE FUNCTION ConsentAsk(Foot AS STRING, Url AS STRING, Yes AS STRING, No AS STRING) AS INTEGER
DECLARE SUB LayOutConsent
DECLARE FUNCTION InfoRowLayOut(Icon AS RImage, Head AS RLabel, Body AS RLabel, X AS INTEGER, Y AS INTEGER, HeadW AS INTEGER, W AS INTEGER) AS INTEGER
DECLARE SUB OpenWebLink(Url AS STRING)

CONST CONSENT_ROWS = 5

DIM ConsentAnswer AS INTEGER
DIM ConsentCount AS INTEGER
DIM ConsentUrl AS STRING
DIM ConsentLogoBmp AS RBitmap
DIM ConsentLogoLoaded AS INTEGER
DIM ConsentMade AS INTEGER
DIM ConsentRowIcon(4) AS RImage
DIM ConsentRowBmp(4) AS RBitmap
DIM ConsentRowHead(4) AS RLabel
DIM ConsentRowBody(4) AS RLabel
DIM ConsentRowIconName(4) AS STRING

CREATE ConsentDialog AS RForm
  Caption = "RapidR Studio"
  Width = 560: Height = 420
  BorderStyle = 3
  ' (Cancel first: the safe answer has the keyboard when the question shows)
  CREATE ConsentNo AS RButton
    Caption = TR$("Cancel"): Tag = 0
    Default = 1
    Cancel = 1
    OnClick = ConsentPick
  END CREATE
  CREATE ConsentYes AS RButton
    Tag = 1
    OnClick = ConsentPick
  END CREATE
  CREATE ConsentLink AS RLabel
    Caption = TR$("Learn more")
    TabStop = 1
    AccessibleDescription = TR$("Opens the manual's page in your browser")
    OnClick = ConsentLinkClick
  END CREATE
  CREATE ConsentLogo AS RImage
    Width = 56: Height = 56
    Stretch = 1
    AccessibleHidden = 1
  END CREATE
  CREATE ConsentTitle AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE ConsentLead AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE ConsentFoot AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  ' (the browser's opener: never shown)
  CREATE ConsentWeb AS RMarkdownView
    Visible = 0
    OpenLinks = 1
    AccessibleHidden = 1
  END CREATE
END CREATE

SUB ConsentPick(Sender AS RButton)
  ConsentAnswer = Sender.Tag
  ConsentDialog.Close
END SUB

SUB ConsentLinkClick
  IF ConsentUrl <> "" THEN OpenWebLink ConsentUrl
END SUB

' A web page in the browser (http, https), as a click on a link does.
SUB OpenWebLink(Url AS STRING)
  DIM ok AS INTEGER
  ok = ConsentWeb.OpenLink(Url)
END SUB

' A new question: its title (a short question) and its lead line; rows
' follow (ConsentRow), then ConsentAsk shows it.
SUB ConsentBegin(Title AS STRING, Lead AS STRING)
  DIM k AS INTEGER
  IF ConsentMade = 0 THEN
    FOR k = 0 TO CONSENT_ROWS - 1
      ConsentRowIcon(k).Parent = ConsentDialog
      ConsentRowIcon(k).Width = 16: ConsentRowIcon(k).Height = 16
      ConsentRowIcon(k).AccessibleHidden = 1
      ConsentRowHead(k).Parent = ConsentDialog
      ConsentRowHead(k).AutoSize = 0
      ' (the body says the head too, for a screen reader: one line each)
      ConsentRowHead(k).AccessibleHidden = 1
      ConsentRowBody(k).Parent = ConsentDialog
      ConsentRowBody(k).WordWrap = 1: ConsentRowBody(k).AutoSize = 0
    NEXT
    ConsentMade = 1
  END IF
  FOR k = 0 TO CONSENT_ROWS - 1
    ConsentRowIcon(k).Visible = 0: ConsentRowHead(k).Visible = 0: ConsentRowBody(k).Visible = 0
  NEXT
  ConsentCount = 0
  ConsentTitle.Caption = Title
  ConsentLead.Caption = Lead
END SUB

' A row: Icon (RapidR's icons: "visible", "lock", "info" …), its head (a
' word or three) and what it says.
SUB ConsentRow(Icon AS STRING, Head AS STRING, Body AS STRING)
  IF ConsentCount >= CONSENT_ROWS THEN EXIT SUB
  ConsentRowIconName(ConsentCount) = Icon
  ConsentRowHead(ConsentCount).Caption = Head
  ConsentRowBody(ConsentCount).Caption = Body
  ConsentRowBody(ConsentCount).AccessibleName = Head + ": " + Body
  ConsentCount = ConsentCount + 1
END SUB

' Shows the question: the footer under the rows, Learn more (Url; none
' when empty) at the left of the buttons, Yes and No at the right — No
' (the safe answer) the default, Escape too. 1 when Yes was pressed.
FUNCTION ConsentAsk(Foot AS STRING, Url AS STRING, Yes AS STRING, No AS STRING) AS INTEGER
  ConsentFoot.Caption = Foot
  ConsentUrl = Url
  ConsentLink.Visible = Flag(Url <> "")
  ConsentYes.Caption = Yes
  ConsentNo.Caption = No
  LayOutConsent
  ConsentAnswer = 0
  ConsentDialog.Center
  ConsentDialog.ShowModal
  Result = ConsentAnswer
END FUNCTION

' One row at (X, Y), W wide: the icon, the head in a column HeadW wide,
' what it says beside it, wrapped; its height.
FUNCTION InfoRowLayOut(Icon AS RImage, Head AS RLabel, Body AS RLabel, X AS INTEGER, Y AS INTEGER, HeadW AS INTEGER, W AS INTEGER) AS INTEGER
  DIM bw AS INTEGER, bh AS INTEGER
  bw = W - 24 - HeadW
  bh = AISetLinesHigh(Body.Caption, bw)
  IF bh < 18 THEN bh = 18
  Icon.Left = X: Icon.Top = Y + 1
  Head.Left = X + 24: Head.Top = Y: Head.Width = HeadW - 8: Head.Height = 18
  Body.Left = X + 24 + HeadW: Body.Top = Y: Body.Width = bw: Body.Height = bh
  Result = bh
END FUNCTION

' (the question's look: RapidQ's sizes in the classic look, RapidR's
' elsewhere — as the save prompt's; the design guide's spacing)
SUB LayOutConsent
  DIM pad AS INTEGER, ic AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, bh AS INTEGER, th AS INTEGER
  DIM hw AS INTEGER, k AS INTEGER, h AS INTEGER, yw AS INTEGER, nw AS INTEGER
  bh = UiButtonH
  IF ThemeName = "classic" THEN
    pad = 14: ic = 32: w = 440: th = 20: hw = 96
  ELSE
    pad = 20: ic = 56: w = 480: th = 24: hw = 104
  END IF
  ConsentDialog.Font.Name = UIFont.Name: ConsentDialog.Font.Size = UIFont.Size
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  ' (the heads' column as wide as the widest head in Studio's language —
  ' they're bold: a tenth more — English's width at least)
  FOR k = 0 TO ConsentCount - 1
    h = AISetMeasure.TextWidth(ConsentRowHead(k).Caption) * 11 \ 10 + 16
    IF h > hw THEN hw = h
  NEXT
  ConsentTitle.Font = SubFont
  ConsentTitle.Font.Color = TC("page.text")
  ConsentLead.Font.Color = TC("page.text")
  ConsentFoot.Font.Color = TC("page.dim")
  ConsentLink.Font = LinkFont
  ConsentLink.Font.Color = TC("page.link")
  ConsentLink.Font.Underline = 1
  ' (the icon: RapidR Studio's, as the save prompt shows)
  IF ConsentLogoLoaded = 0 THEN
    ConsentLogoBmp.LoadFromFile HomeDir + "ide/assets/rapidr-studio.svg"
    ConsentLogo.BMPHandle = ConsentLogoBmp
    ConsentLogoLoaded = 1
  END IF
  ConsentLogo.Left = pad: ConsentLogo.Top = pad: ConsentLogo.Width = ic: ConsentLogo.Height = ic
  x = pad + ic + 16
  y = pad
  ConsentTitle.Left = x: ConsentTitle.Top = y: ConsentTitle.Width = w: ConsentTitle.Height = th
  y = y + th + 8
  h = AISetLinesHigh(ConsentLead.Caption, w)
  ConsentLead.Left = x: ConsentLead.Top = y: ConsentLead.Width = w: ConsentLead.Height = h
  y = y + h + 16
  FOR k = 0 TO ConsentCount - 1
    ConsentRowHead(k).Font = UIFont
    ConsentRowHead(k).Font.Bold = 1
    ConsentRowHead(k).Font.Color = TC("page.text")
    ConsentRowBody(k).Font = UIFont
    ConsentRowBody(k).Font.Color = TC("page.text")
    ConsentRowBmp(k).LoadIcon ConsentRowIconName(k), 16
    ConsentRowIcon(k).BMPHandle = ConsentRowBmp(k)
    h = InfoRowLayOut(ConsentRowIcon(k), ConsentRowHead(k), ConsentRowBody(k), x, y, hw, w)
    ConsentRowIcon(k).Visible = 1: ConsentRowHead(k).Visible = 1: ConsentRowBody(k).Visible = 1
    y = y + h + 12
  NEXT
  y = y + 4
  h = AISetLinesHigh(ConsentFoot.Caption, w)
  ConsentFoot.Left = x: ConsentFoot.Top = y: ConsentFoot.Width = w: ConsentFoot.Height = h
  y = y + h + 20
  ' (the buttons: their words and 12 each side at least, the dialog's width)
  yw = AISetMeasure.TextWidth(REPLACESUBSTR$(ConsentYes.Caption, "&", "")) + 32
  IF yw < 96 THEN yw = 96
  nw = AISetMeasure.TextWidth(REPLACESUBSTR$(ConsentNo.Caption, "&", "")) + 32
  IF nw < 96 THEN nw = 96
  ConsentNo.Width = nw: ConsentNo.Height = bh: ConsentNo.Top = y: ConsentNo.Left = x + w - nw
  ConsentYes.Width = yw: ConsentYes.Height = bh: ConsentYes.Top = y: ConsentYes.Left = ConsentNo.Left - 8 - yw
  ' (a link's text sits 4 inside its rectangle: the text on the column's edge)
  ConsentLink.AutoSize = 0
  ConsentLink.Width = AISetMeasure.TextWidth(ConsentLink.Caption) + 10: ConsentLink.Height = 24
  ConsentLink.Left = x - 4: ConsentLink.Top = y + (bh - 24) \ 2 + 3
  ConsentDialog.ClientWidth = x + w + pad
  ConsentDialog.ClientHeight = y + bh + pad
END SUB
