' schoollock.inc: School Lock (docs/manual/for-teachers.md) — a teacher
' turns features of RapidR Studio off behind a password; a school's IT can
' do it for every account of a computer with a policy file. For now the
' features are its AI: RAI in Studio ("ai"), AI apps over MCP ("ai_apps"),
' and RAI kept in Coach mode ("coach": RAI stays on with hints only —
' aicoach.inc); the list is open for more (rapidr-studio's school_lock.rs
' keeps the lock, the hash, the wrong tries and the policy; RPROJECT's
' SchoolLock… members reach it).
'
' Off by default. Tools > School Lock… turns it on: what to lock, a
' password typed twice (6 characters at least, a strength hint). While it
' locks:
'   - RAI's pane shows a calm card, "AI is turned off by your teacher",
'     with Unlock…; nothing is sent to an AI;
'   - Set Up RAI, Settings > AI and Settings > AI Apps are read-only, with
'     the same note and Unlock…;
'   - the MCP server can't start (and stops when it runs);
'   - the AI commands in the menus and the palette are disabled, the
'     reason beside them;
'   - the status bar has a chip, "School Lock".
' Unlock… asks the password: Unlock Until Studio Closes, or Turn School
' Lock Off (the password again). Five wrong passwords in a row: a minute's
' wait, counted down. Only a salted, slow hash of the password is kept (in
' Studio's settings), never the password. The computer's policy wins: then
' the lock is unlocked for this run only, never turned off here.
'
' While the lock is on (unlocked until Studio closes or not), RapidR's
' crash reports offer no strip and no "Report on GitHub" (crash.inc's
' CrashSchoolLock: a GitHub account is 13+); SchoolLockHidesReporting
' says the same to anything else that should hide with it.
'
' Test steps (lock:<what>[=<value>]): lock:set=<password>[|<features>],
' lock:unlock=<password>, lock:off=<password>, lock:again, lock:dialog,
' lock:state (the state in the Output).

DECLARE SUB KeepSchoolLock
DECLARE FUNCTION LockUnlocked AS INTEGER
DECLARE FUNCTION SchoolLockHidesReporting AS INTEGER
DECLARE FUNCTION LockWhatText AS STRING
DECLARE SUB LockShowMode(Mode AS STRING)
DECLARE SUB LayOutLockDialog
DECLARE SUB LockMainClick
DECLARE SUB LockOffClick
DECLARE SUB LockCancelClick
DECLARE SUB LockLinkClick
DECLARE SUB LockPwChanged
DECLARE SUB LockTick
DECLARE SUB LockFocusFirst
DECLARE SUB LockSay(Text AS STRING)
DECLARE SUB LockShowWait
DECLARE FUNCTION LockTimeText(S AS INTEGER) AS STRING
DECLARE FUNCTION LockTry(Outcome AS INTEGER) AS INTEGER
DECLARE SUB LockSetOn(Password AS STRING, Features AS STRING)
DECLARE SUB LockUnlockWith(Password AS STRING)
DECLARE SUB LockOffWith(Password AS STRING)
DECLARE SUB LockAgain
DECLARE SUB LayOutLockCard
DECLARE SUB LockCardUnlock
DECLARE SUB LockChipShow
DECLARE SUB LockBoxesChanged
DECLARE SUB LockStripLayOut(Strip AS RPanel, Mark AS RPanel, Icon AS RImage, Head AS RLabel, Body AS RLabel, Btn AS RButton, W AS INTEGER)

' (the dialog's mode: "set" — off, turn it on —, "locked", "unlocked")
DIM LockMode AS STRING
' (what Lock… / Unlock… answered last: said in the hint line)
DIM LockAnswer AS INTEGER
DIM LockLogoBmp AS RBitmap
DIM LockLogoLoaded AS INTEGER
' (RAI's pane: the card shown instead of the chat while AI is off)
DIM LockCard AS RPanel
DIM LockCardIcon AS RImage
DIM LockCardBmp AS RBitmap
DIM LockCardTitle AS RLabel
DIM LockCardBody AS RLabel
DIM LockCardBtn AS RButton
' (the status bar's chip: a soft info pill, the lock and its words)
DIM LockChipBox AS RPanel
DIM LockChip AS RCanvas
DIM LockChipIcon AS RImage
DIM LockChipBmp AS RBitmap
DIM LockChipText AS RLabel
' (Settings > AI and AI Apps: a strip over the page — a card, its mark,
' the lock, the note, Unlock…)
DIM LockSetStrip AS RPanel
DIM LockSetMark AS RPanel
DIM LockSetIcon AS RImage
DIM LockSetBmp AS RBitmap
DIM LockSetHead AS RLabel
DIM LockSetBody AS RLabel
DIM LockSetBtn AS RButton
' (Set Up RAI's first page: the same strip)
DIM LockSuStrip AS RPanel
DIM LockSuMark AS RPanel
DIM LockSuIcon AS RImage
DIM LockSuBmp AS RBitmap
DIM LockSuHead AS RLabel
DIM LockSuBody AS RLabel
DIM LockSuBtn AS RButton
' (the lock as it was when last applied: its changes are said once)
DIM LockWasAI AS INTEGER
DIM LockWasApps AS INTEGER
DIM LockWasCoach AS INTEGER
DIM LockMade AS INTEGER

CREATE LockDialog AS RForm
  Caption = TR$("School Lock")
  Width = 600: Height = 420
  BorderStyle = 3
  ' (the keyboard in the password's field — or on the main button)
  OnShow = LockFocusFirst
  CREATE LockLogo AS RImage
    Width = 56: Height = 56
    Stretch = 1
    AccessibleHidden = 1
  END CREATE
  CREATE LockTitle AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE LockLead AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE LockWhatLabel AS RLabel
    Caption = TR$("What to lock:")
  END CREATE
  CREATE LockAIBox AS RCheckBox
    Caption = TR$("&AI in Studio (RAI)")
    Checked = 1
    AccessibleDescription = TR$("The AI Assistant pane, Set Up RAI and Settings > AI: nothing is sent to an AI")
    OnClick = LockBoxesChanged
  END CREATE
  CREATE LockAppsBox AS RCheckBox
    Caption = TR$("AI apps (&MCP)")
    Checked = 1
    AccessibleDescription = TR$("AI apps on this computer can't connect to RapidR Studio")
  END CREATE
  ' (RAI stays on, in Coach mode only: hints, never the answer — for when
  ' AI in Studio isn't locked)
  CREATE LockCoachBox AS RCheckBox
    Caption = TR$("Keep RAI in &Coach mode")
    AccessibleDescription = TR$("RAI stays on but only gives hints. It never writes the answer into the student's code. Uncheck AI in Studio to choose this.")
  END CREATE
  CREATE LockPwLabel AS RLabel
    Caption = TR$("&Password:")
  END CREATE
  CREATE LockPwEdit AS REdit
    PasswordChar = "*"
    AccessibleName = TR$("Password")
    OnChange = LockPwChanged
  END CREATE
  CREATE LockPw2Label AS RLabel
    Caption = TR$("Type it a&gain:")
  END CREATE
  CREATE LockPw2Edit AS REdit
    PasswordChar = "*"
    AccessibleName = TR$("Type the password again")
    OnChange = LockPwChanged
  END CREATE
  ' (the strength, a mismatch, a wrong password, the wait's countdown)
  CREATE LockHint AS RLabel
    WordWrap = 1: AutoSize = 0
    AccessibleName = TR$("About the password")
  END CREATE
  CREATE LockFoot AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE LockLink AS RLabel
    Caption = TR$("Learn more about School Lock")
    TabStop = 1
    AccessibleDescription = TR$("Opens the manual's page for teachers in your browser")
    OnClick = LockLinkClick
  END CREATE
  CREATE LockOffBtn AS RButton
    Caption = TR$("&Turn School Lock Off")
    OnClick = LockOffClick
  END CREATE
  CREATE LockMainBtn AS RButton
    Default = 1
    OnClick = LockMainClick
  END CREATE
  CREATE LockCancelBtn AS RButton
    Caption = TR$("Cancel")
    Cancel = 1
    OnClick = LockCancelClick
  END CREATE
  CREATE LockTimer AS RTimer
    Interval = 1000
    Enabled = 0
    OnTimer = LockTick
  END CREATE
END CREATE

' ---- the lock's state ---------------------------------------------------------------

FUNCTION LockOffAI AS INTEGER
  Result = Flag(Proj.SchoolLocked("ai") <> 0)
END FUNCTION

FUNCTION LockOffApps AS INTEGER
  Result = Flag(Proj.SchoolLocked("ai_apps") <> 0)
END FUNCTION

' (RAI kept in Coach mode: aicoach.inc's dial shows it)
FUNCTION LockKeepsCoach AS INTEGER
  Result = Flag(Proj.SchoolLocked("coach") <> 0)
END FUNCTION

FUNCTION LockIsOn AS INTEGER
  Result = Flag(Proj.SchoolLock("on") = "1")
END FUNCTION

FUNCTION LockUnlocked AS INTEGER
  Result = Flag(Proj.SchoolLock("unlocked") = "1")
END FUNCTION

FUNCTION LockByPolicy AS INTEGER
  Result = Flag(Proj.SchoolLock("policy") = "1")
END FUNCTION

' What School Lock also hides — a crash report's strip and its "Report on
' GitHub" (CrashSchoolLock): True while the lock is on, unlocked or not.
FUNCTION SchoolLockHidesReporting AS INTEGER
  Result = LockIsOn
END FUNCTION

' The per-user setting, kept with Studio's others (never the password:
' its salted hash) — read even with --fresh: a lock isn't left behind by
' starting Studio another way; and the computer's policy.
SUB LoadSchoolLock
  DIM setting AS STRING, ok AS INTEGER
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    IF Reg.ValueExists("SchoolLock") THEN setting = Reg.ReadString("SchoolLock")
  END IF
  ok = Proj.SchoolLockLoad(setting)
  LockWasAI = LockOffAI
  LockWasApps = LockOffApps
  LockWasCoach = LockKeepsCoach
  ' (no crash strip, no Report on GitHub while it is on: crash.inc — before
  ' its strip is offered, once Studio's window is up)
  CrashSchoolLock SchoolLockHidesReporting
END SUB

' (the setting kept at once: a lock never waits for Studio to close)
SUB KeepSchoolLock
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("SchoolLock", Proj.SchoolLock("setting"))
END SUB

' (what the lock turns off, in a sentence — RAI off says it all: Coach
' mode is RAI's)
FUNCTION LockWhatText AS STRING
  DIM f AS STRING
  f = "," + Proj.SchoolLock("features") + ","
  IF INSTR(f, ",ai,") > 0 AND INSTR(f, ",ai_apps,") > 0 THEN
    Result = TR$("RAI and AI apps are off in RapidR Studio on this computer.")
  ELSEIF INSTR(f, ",ai,") > 0 THEN
    Result = TR$("RAI is off in RapidR Studio on this computer.")
  ELSEIF INSTR(f, ",coach,") > 0 AND INSTR(f, ",ai_apps,") > 0 THEN
    Result = TR$("RAI stays in Coach mode, and AI apps can't connect to RapidR Studio on this computer.")
  ELSEIF INSTR(f, ",coach,") > 0 THEN
    Result = TR$("RAI stays in Coach mode in RapidR Studio on this computer: it gives hints one step at a time and doesn't change the student's files.")
  ELSE
    Result = TR$("AI apps can't connect to RapidR Studio on this computer.")
  END IF
END FUNCTION

FUNCTION LockTitleText AS STRING
  IF LockByPolicy THEN Result = TR$("AI is turned off by your school") ELSE Result = TR$("AI is turned off by your teacher")
END FUNCTION

' (beside a disabled AI command: the menu's hint, the palette's title)
FUNCTION LockReason AS STRING
  IF LockByPolicy THEN Result = TR$("turned off by your school") ELSE Result = TR$("turned off by your teacher")
END FUNCTION

' ---- setting up ----------------------------------------------------------------------

' (after SetUpAI, SetUpMcp and the Settings dialog: the card, the chip,
' the strips; then the lock applied — the MCP server never started)
SUB SetUpSchoolLock
  ' (RAI's pane: the card, shown in place of the chat)
  LockCard.Parent = AIPanel
  LockCard.Align = 5
  LockCard.BevelOuter = 0
  LockCard.Visible = 0
  LockCardIcon.Parent = LockCard
  LockCardIcon.Width = 24: LockCardIcon.Height = 24
  LockCardIcon.AccessibleHidden = 1
  LockCardTitle.Parent = LockCard
  LockCardTitle.WordWrap = 1: LockCardTitle.AutoSize = 0
  LockCardBody.Parent = LockCard
  LockCardBody.WordWrap = 1: LockCardBody.AutoSize = 0
  LockCardBtn.Parent = LockCard
  LockCardBtn.OnClick = LockCardUnlock
  LockCard.OnResize = LayOutLockCard
  ' (the status bar's chip: in the bar, at the right end of the project's
  ' panel — a child of the bar, never of the window, whose aligned parts
  ' it would push)
  LockChipBox.Parent = Status
  LockChipBox.BevelOuter = 0
  LockChipBox.Visible = 0
  LockChip.Parent = LockChipBox
  LockChip.Tag = 0
  LockChip.OnPaint = ChipPaint
  LockChip.AccessibleHidden = 1
  LockChipIcon.Parent = LockChipBox
  LockChipIcon.Width = 12: LockChipIcon.Height = 12
  LockChipIcon.AccessibleHidden = 1
  LockChipText.Parent = LockChipBox
  LockChipText.Transparent = 1
  LockChipText.AccessibleHidden = 1
  ' (Settings' strip: over the AI and AI Apps pages)
  LockSetStrip.Parent = SettingsDialog
  LockSetStrip.BevelOuter = 0
  LockSetStrip.Visible = 0
  LockSetMark.Parent = LockSetStrip
  LockSetMark.BevelOuter = 0
  LockSetMark.AccessibleHidden = 1
  LockSetIcon.Parent = LockSetStrip
  LockSetIcon.Width = 16: LockSetIcon.Height = 16
  LockSetIcon.AccessibleHidden = 1
  LockSetHead.Parent = LockSetStrip
  LockSetHead.Transparent = 1: LockSetHead.AutoSize = 0
  LockSetBody.Parent = LockSetStrip
  LockSetBody.Transparent = 1: LockSetBody.AutoSize = 0
  LockSetBtn.Parent = LockSetStrip
  LockSetBtn.OnClick = LockCardUnlock
  ' (Set Up RAI's first page: the same)
  LockSuStrip.Parent = AISuPage1
  LockSuStrip.BevelOuter = 0
  LockSuStrip.Visible = 0
  LockSuMark.Parent = LockSuStrip
  LockSuMark.BevelOuter = 0
  LockSuMark.AccessibleHidden = 1
  LockSuIcon.Parent = LockSuStrip
  LockSuIcon.Width = 16: LockSuIcon.Height = 16
  LockSuIcon.AccessibleHidden = 1
  LockSuHead.Parent = LockSuStrip
  LockSuHead.Transparent = 1: LockSuHead.AutoSize = 0
  LockSuBody.Parent = LockSuStrip
  LockSuBody.Transparent = 1: LockSuBody.AutoSize = 0
  LockSuBtn.Parent = LockSuStrip
  LockSuBtn.OnClick = LockCardUnlock
  LockMade = 1
  ' (a policy file that can't be read: everything stays locked — said once)
  IF Proj.SchoolLock("error") <> "" THEN OutputLine TR$("School Lock: this computer's policy ({file}) can't be read: {error}. AI stays off until your school's IT fixes it.", "file", Proj.SchoolLock("file"), "error", Proj.SchoolLock("error"))
  SchoolLockChrome
  SchoolLockApply
END SUB

' The lock applied everywhere: RAI, AI apps, the commands, the chip, the
' strips. What changed is said (the Output and a screen reader).
SUB SchoolLockApply
  DIM aiOff AS INTEGER, appsOff AS INTEGER, said AS STRING, coachOn AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  aiOff = LockOffAI
  appsOff = LockOffApps
  coachOn = LockKeepsCoach
  ' (Coach kept on, or let go: the dial — aicoach.inc)
  AIDialApply
  LayOutAIPanel
  ' (RAI: nothing is asked while it is off; its pane shows the card)
  IF aiOff THEN AI.Cancel
  AIShowOrHide
  ' (AI apps: the server stopped; started again when unlocked, as the
  ' user had it)
  IF appsOff THEN
    IF Mcp.Listening THEN
      McpStopServer
      OutputLine TR$("AI apps: stopped, School Lock is on. Every AI app was disconnected.")
    END IF
  ELSEIF LockWasApps AND McpOn AND McpConsentOk AND Proj.FileManager <> "" THEN
    McpStart
  END IF
  McpShowState
  ' (the commands: disabled ones say why; the tool bar's AI button)
  FillPalette
  UpdateCommands
  LockChipShow
  LockSettingsShow
  LockSetupShow
  ' (Unlock… pressed on a strip: the dialog under it laid out again —
  ' Set Up RAI looks for servers now)
  IF SettingsDialog.Visible THEN LayOutSettingsDialog
  IF AISetup.Visible THEN
    LayOutAISetup
    IF aiOff = 0 AND LockWasAI <> 0 THEN AISuLook
  END IF
  IF aiOff <> LockWasAI OR appsOff <> LockWasApps OR coachOn <> LockWasCoach THEN
    IF aiOff OR appsOff OR coachOn THEN
      said = TR$("School Lock is on: {what}", "what", LockWhatText)
    ELSEIF LockIsOn THEN
      said = TR$("School Lock is unlocked until Studio closes.")
    ELSE
      said = TR$("School Lock is off.")
    END IF
    OutputLine said
    Application.Announce said
  END IF
  LockWasAI = aiOff
  LockWasApps = appsOff
  LockWasCoach = coachOn
  ' (the crash strip and Report on GitHub hidden while it is on: crash.inc)
  CrashSchoolLock SchoolLockHidesReporting
END SUB

' (the look: the card's, the chip's, the strips' colours and fonts)
SUB SchoolLockChrome
  IF LockMade = 0 THEN EXIT SUB
  LockCard.Color = TC("face")
  LockCardTitle.Font = CardFont
  LockCardTitle.Font.Color = TC("page.text")
  LockCardBody.Font = UIFont
  LockCardBody.Font.Color = TC("page.dim")
  LockCardBtn.Font = UIFont
  LockCardBmp.LoadIcon "lock", 24
  LockCardIcon.BMPHandle = LockCardBmp
  LockChipBox.Color = TC("statusbar")
  LockChip.Color = TC("statusbar")
  LockChipText.Font = StatusFont
  LockChipText.Font.Color = TC("status.info.text")
  LockSetStrip.Color = TC("card")
  LockSetMark.Color = TC("status.info.border")
  LockSetHead.Font = UIFont
  LockSetHead.Font.Bold = 1
  LockSetHead.Font.Color = TC("page.text")
  LockSetBody.Font = UIFont
  LockSetBody.Font.Color = TC("page.dim")
  LockSetBtn.Font = UIFont
  LockSetBmp.LoadIcon "lock", 16
  LockSetIcon.BMPHandle = LockSetBmp
  LockSuStrip.Color = TC("card")
  LockSuMark.Color = TC("status.info.border")
  LockSuHead.Font = UIFont
  LockSuHead.Font.Bold = 1
  LockSuHead.Font.Color = TC("page.text")
  LockSuBody.Font = UIFont
  LockSuBody.Font.Color = TC("page.dim")
  LockSuBtn.Font = UIFont
  LockSuBmp.LoadIcon "lock", 16
  LockSuIcon.BMPHandle = LockSuBmp
  LockChipShow
  LockCardShow
  LockSettingsShow
  LockSetupShow
END SUB

SUB SchoolLockLanguageChanged
  SchoolLockChrome
  FillPalette
  UpdateCommands
END SUB

' ---- RAI's pane: the card -----------------------------------------------------------

' (ai.inc's AIShowOrHide: the card in place of the chat while RAI is off)
SUB LockCardShow
  DIM off AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  off = LockOffAI
  ' (only when the lock changes: otherwise the pane's parts keep their own
  ' states — the chats list, the live strip …)
  IF off <> Flag(LockCard.Visible <> 0) THEN
    IF off THEN
      ' (the chats list closed first: the chat comes back with the unlock)
      IF AIChatsPanel.Visible THEN AIShowChats 0
      AITop.Visible = 0: AIStatus.Visible = 0: AIBottom.Visible = 0: AIChat.Visible = 0
      AISlashBar.Visible = 0: AIEditBar.Visible = 0: AIQueueBar.Visible = 0: AIReviewBar.Visible = 0: AILive.Visible = 0
      LockCard.Visible = 1
    ELSE
      LockCard.Visible = 0
      AITop.Visible = 1: AIStatus.Visible = 1: AIBottom.Visible = 1: AIChat.Visible = 1
      ' (changes waiting for review before the lock: their bar again)
      AIShowReviewBar
    END IF
  END IF
  LockCardTitle.Caption = LockTitleText
  LockCardBody.Caption = LockWhatText + " " + TR$("Everything else works as usual.")
  LockCardBtn.Caption = TR$("&Unlock...")
  LockCardBtn.AccessibleDescription = TR$("Asks for the password")
  LockCard.AccessibleName = LockTitleText
  LayOutLockCard
END SUB

' (the card: the lock, the title and what is off, Unlock… — in the top
' third of the pane, 16 from its edges)
SUB LayOutLockCard
  DIM w AS INTEGER, x AS INTEGER, y AS INTEGER, h AS INTEGER, bw AS INTEGER
  w = LockCard.ClientWidth
  IF w < 200 THEN w = 360
  x = 16
  y = 32
  LockCardIcon.Left = x: LockCardIcon.Top = y
  y = y + 24 + 12
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  h = AISetLinesHigh(LockCardTitle.Caption, w - 2 * x) + 2
  LockCardTitle.Left = x: LockCardTitle.Top = y: LockCardTitle.Width = w - 2 * x: LockCardTitle.Height = h
  y = y + h + 8
  h = AISetLinesHigh(LockCardBody.Caption, w - 2 * x)
  LockCardBody.Left = x: LockCardBody.Top = y: LockCardBody.Width = w - 2 * x: LockCardBody.Height = h
  y = y + h + 16
  bw = UITextWidth(LockCardBtn.Caption) + 24
  IF bw < 96 THEN bw = 96
  LockCardBtn.Left = x: LockCardBtn.Top = y: LockCardBtn.Width = bw: LockCardBtn.Height = UiCompactH
END SUB

' (Unlock… on the card or a strip: the dialog)
SUB LockCardUnlock
  ShowSchoolLock
END SUB

' ---- the status bar's chip ------------------------------------------------------------

SUB LockChipShow
  DIM shown AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  shown = LockIsOn
  IF shown THEN
    IF LockUnlocked THEN
      LockChipText.Caption = TR$("School Lock: unlocked")
      LockChipBmp.LoadIcon "unlock", 12
      LockChipBox.AccessibleName = TR$("School Lock is unlocked until Studio closes")
    ELSE
      LockChipText.Caption = TR$("School Lock")
      LockChipBmp.LoadIcon "lock", 12
      LockChipBox.AccessibleName = TR$("School Lock is on: {what}", "what", LockWhatText)
    END IF
    LockChipIcon.BMPHandle = LockChipBmp
  END IF
  IF LockChipBox.Visible <> shown THEN
    LockChipBox.Visible = shown
  END IF
  LayOutStatus
  LockChip.Repaint
END SUB

' (chrome.inc's LayOutStatus: the room the chip takes at the right of the
' project's panel — 0 when it isn't there)
FUNCTION LockChipRoom AS INTEGER
  IF LockMade = 0 THEN EXIT FUNCTION
  IF LockChipBox.Visible = 0 THEN EXIT FUNCTION
  StatusMeasure.Font.Name = StatusFont.Name: StatusMeasure.Font.Size = StatusFont.Size
  Result = 8 + 12 + 6 + StatusMeasure.TextWidth(LockChipText.Caption) + 12 + 8
END FUNCTION

' (the chip ending at X, on the status bar's centre line: 8, the icon, 6,
' the words, 12 — the design guide's chip, as tall as the bar allows: the
' bar is shorter than a pane's header, so its icon is 12 pixels)
SUB LockChipPlace(X AS INTEGER)
  DIM w AS INTEGER, h AS INTEGER, th AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  IF LockChipBox.Visible = 0 THEN EXIT SUB
  w = LockChipRoom - 8
  h = Status.Height - 2
  IF h > UiCompactH THEN h = UiCompactH
  IF h < 16 THEN h = 16
  LockChipBox.Left = X - w: LockChipBox.Top = (Status.Height - h) \ 2
  LockChipBox.Width = w: LockChipBox.Height = h
  LockChip.Left = 0: LockChip.Top = 0: LockChip.Width = w: LockChip.Height = h
  LockChipIcon.Width = 12: LockChipIcon.Height = 12
  LockChipIcon.Left = 8: LockChipIcon.Top = (h - 12) \ 2
  th = LockChipText.Height
  LockChipText.Left = 8 + 12 + 6: LockChipText.Top = (h - th) \ 2
END SUB

' ---- the strips in Settings and Set Up RAI ----------------------------------------------

' (the strip's height: the note's two lines and their margins)
FUNCTION LockStripH AS INTEGER
  IF ThemeName = "classic" THEN Result = 48 ELSE Result = 56
END FUNCTION

' A strip W wide: a card, its 4-pixel mark, the lock, the head and the
' note under it, Unlock… at the right, compact, on the strip's centre.
SUB LockStripLayOut(Strip AS RPanel, Mark AS RPanel, Icon AS RImage, Head AS RLabel, Body AS RLabel, Btn AS RButton, W AS INTEGER)
  DIM h AS INTEGER, bw AS INTEGER, tw AS INTEGER
  h = LockStripH
  Strip.Width = W: Strip.Height = h
  Mark.Left = 0: Mark.Top = 0: Mark.Width = 4: Mark.Height = h
  bw = UITextWidth(Btn.Caption) + 24
  IF bw < 96 THEN bw = 96
  Btn.Width = bw: Btn.Height = UiCompactH
  Btn.Left = W - 12 - bw: Btn.Top = (h - UiCompactH) \ 2
  Icon.Left = 16: Icon.Top = (h - 36) \ 2 + 1
  tw = W - 16 - 24 - 12 - bw - 12
  Head.Left = 40: Head.Top = (h - 36) \ 2: Head.Width = tw: Head.Height = 18
  Body.Left = 40: Body.Top = (h - 36) \ 2 + 18: Body.Width = tw: Body.Height = 18
END SUB

' (Settings' AI and AI Apps pages: read-only while their feature is off,
' the strip under the greyed switch in place of its note — settings.inc
' places it)
SUB LockSettingsShow
  DIM aiOff AS INTEGER, appsOff AS INTEGER, tabNow AS INTEGER, shown AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  aiOff = LockOffAI
  appsOff = LockOffApps
  tabNow = SettingsTabs.TabIndex
  shown = Flag((tabNow = 1 AND aiOff <> 0) OR (tabNow = 2 AND appsOff <> 0))
  LockSetStrip.Visible = shown
  LockSetHead.Caption = LockTitleText + "."
  LockSetBody.Caption = TR$("These settings can't be changed while School Lock is on.")
  LockSetBtn.Caption = TR$("&Unlock...")
  LockSetBtn.AccessibleDescription = TR$("Asks for the password")
  LockSetStrip.AccessibleName = LockTitleText + ". " + LockSetBody.Caption
  ' (the switches' notes: the strip in their place)
  AIOnNote.Visible = Flag(aiOff = 0)
  McpOnNote.Visible = Flag(appsOff = 0)
  ' (the AI page's controls: read-only)
  AIOnBox.Enabled = Flag(aiOff = 0)
  AISetupBtn.Enabled = Flag(aiOff = 0)
  AIProvBox.Enabled = Flag(aiOff = 0)
  AIPricesLink.Enabled = Flag(aiOff = 0)
  AIKeyEdit.Enabled = Flag(aiOff = 0)
  AIKeySaveBtn.Enabled = Flag(aiOff = 0)
  AIKeyRemoveBtn.Enabled = Flag(aiOff = 0)
  AIRemember.Enabled = Flag(aiOff = 0)
  AIWsEdit.Enabled = Flag(aiOff = 0)
  AIBaseBox.Enabled = Flag(aiOff = 0)
  AIModelCombo.Enabled = Flag(aiOff = 0)
  AIFetchBtn.Enabled = Flag(aiOff = 0)
  AIMaxEdit.Enabled = Flag(aiOff = 0)
  AIPriceInEdit.Enabled = Flag(aiOff = 0)
  AIPriceOutEdit.Enabled = Flag(aiOff = 0)
  AIPermBox.Enabled = Flag(aiOff = 0)
  AIReviewBox.Enabled = Flag(aiOff = 0)
  ' (AI Apps: the switch off and greyed — the user's choice is kept for
  ' when it is unlocked —, the rest read-only)
  McpOnBox.Enabled = Flag(appsOff = 0)
  IF appsOff THEN McpOnBox.Checked = 0 ELSE McpOnBox.Checked = McpOn
  McpPortEdit.Enabled = Flag(appsOff = 0)
  McpCopyAddrBtn.Enabled = Flag(appsOff = 0)
  McpCopyHeaderBtn.Enabled = Flag(appsOff = 0)
  McpShowTokenBtn.Enabled = Flag(appsOff = 0)
  McpResetBtn.Enabled = Flag(appsOff = 0)
  McpPageModeBox.Enabled = Flag(appsOff = 0)
  McpClearBtn.Enabled = Flag(appsOff = 0)
END SUB

' (Set Up RAI while RAI is off: its first page read-only, the strip on it,
' nothing looked for)
SUB LockSetupShow
  DIM off AS INTEGER
  IF LockMade = 0 THEN EXIT SUB
  off = LockOffAI
  LockSuStrip.Visible = off
  LockSuHead.Caption = LockTitleText + "."
  LockSuBody.Caption = TR$("RAI can't be set up while School Lock is on.")
  LockSuBtn.Caption = TR$("&Unlock...")
  LockSuBtn.AccessibleDescription = TR$("Asks for the password")
  LockSuStrip.AccessibleName = LockTitleText + ". " + LockSuBody.Caption
  ' (nothing is looked for: no "Looking for a server…")
  IF off THEN AISuFoundText.Caption = ""
  AISuLocal.Enabled = Flag(off = 0)
  AISuCloudRadio.Enabled = Flag(off = 0)
  AISuLookBtn.Enabled = Flag(off = 0)
  AISuNextBtn.Enabled = Flag(off = 0)
END SUB

' (settings.inc: Settings' strip at (X, Y), W wide)
SUB LockSetPlace(X AS INTEGER, Y AS INTEGER, W AS INTEGER)
  LockSetStrip.Left = X: LockSetStrip.Top = Y
  LockStripLayOut LockSetStrip, LockSetMark, LockSetIcon, LockSetHead, LockSetBody, LockSetBtn, W
END SUB

' (aisetup.inc: Set Up RAI's strip at (X, Y), W wide)
SUB LockSuPlace(X AS INTEGER, Y AS INTEGER, W AS INTEGER)
  LockSuStrip.Left = X: LockSuStrip.Top = Y
  LockStripLayOut LockSuStrip, LockSuMark, LockSuIcon, LockSuHead, LockSuBody, LockSuBtn, W
END SUB

' (ai.inc's ShowAIPanel: the keyboard on the card's Unlock…)
SUB LockCardFocus
  LockCardBtn.SetFocus
END SUB

' ---- the dialog ------------------------------------------------------------------------

' Tools > School Lock…, Unlock… on the card or a strip: the dialog, in the
' mode the lock is in.
SUB ShowSchoolLock
  IF LockIsOn = 0 THEN
    LockShowMode "set"
  ELSEIF LockUnlocked THEN
    LockShowMode "unlocked"
  ELSE
    LockShowMode "locked"
  END IF
  LockDialog.Center
  LockDialog.ShowModal
  LockTimer.Enabled = 0
END SUB

SUB LockShowMode(Mode AS STRING)
  DIM isSet AS INTEGER
  LockMode = Mode
  isSet = Flag(Mode = "set")
  LockPwEdit.Text = ""
  LockPw2Edit.Text = ""
  LockHint.Caption = ""
  LockWhatLabel.Visible = isSet
  LockAIBox.Visible = isSet
  LockAppsBox.Visible = isSet
  LockCoachBox.Visible = isSet
  LockPw2Label.Visible = isSet
  LockPw2Edit.Visible = isSet
  SELECT CASE Mode
    CASE "set"
      LockAIBox.Checked = 1
      LockAppsBox.Checked = 1
      LockCoachBox.Checked = 0
      LockBoxesChanged
      LockTitle.Caption = TR$("Turn on School Lock?")
      LockLead.Caption = TR$("Features of RapidR Studio are turned off for everyone who uses this account on this computer, until someone types the password. Everything else keeps working.")
      LockFoot.Caption = TR$("RapidR keeps only a one-way scrambled form of the password on this computer, never the password itself. Anyone who knows it can unlock until Studio closes, so keep it to yourself. Forgot it? The manual's page for teachers says what to do.")
      LockMainBtn.Caption = TR$("&Lock")
      LockMainBtn.AccessibleDescription = TR$("Turns School Lock on with this password")
      LockOffBtn.Visible = 0
      LockHint.Caption = TR$("At least 6 characters.")
    CASE "locked"
      LockTitle.Caption = LockTitleText
      LockLead.Caption = LockWhatText + " " + TR$("Type the password to unlock it until Studio closes.")
      IF LockByPolicy THEN
        LockFoot.Caption = TR$("Your school's IT set this for every account on this computer. It can be unlocked until Studio closes, but only IT can turn it off.")
      ELSE
        LockFoot.Caption = TR$("Turn School Lock Off ends the lock on this account until a teacher turns it on again. It needs the password too.")
      END IF
      LockMainBtn.Caption = TR$("&Unlock Until Studio Closes")
      LockMainBtn.AccessibleDescription = TR$("AI works again until RapidR Studio closes")
      LockOffBtn.Visible = Flag(LockByPolicy = 0)
    CASE "unlocked"
      LockTitle.Caption = TR$("School Lock is unlocked until Studio closes")
      LockLead.Caption = TR$("AI works again until RapidR Studio closes. It is locked again the next time Studio starts.")
      IF LockByPolicy THEN
        LockFoot.Caption = TR$("Your school's IT set this for every account on this computer: only IT can turn it off.")
      ELSE
        LockFoot.Caption = TR$("To turn School Lock off for good, type the password and choose Turn School Lock Off.")
      END IF
      LockMainBtn.Caption = TR$("Lock &Again")
      LockMainBtn.AccessibleDescription = TR$("AI is turned off again now")
      LockOffBtn.Visible = Flag(LockByPolicy = 0)
  END SELECT
  LockPwLabel.Visible = Flag(Mode <> "unlocked" OR LockByPolicy = 0)
  LockPwEdit.Visible = LockPwLabel.Visible
  LockTitle.AccessibleName = LockTitle.Caption
  LockShowWait
  LayOutLockDialog
  LockFocusFirst
END SUB

' (RapidQ's sizes in the classic look, RapidR's elsewhere — the consent's
' and the save prompt's; the design guide's spacing)
SUB LayOutLockDialog
  DIM pad AS INTEGER, ic AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, bh AS INTEGER, th AS INTEGER
  DIM h AS INTEGER, lw AS INTEGER, fh AS INTEGER, mw AS INTEGER, ow AS INTEGER, cw AS INTEGER, need AS INTEGER
  bh = UiButtonH
  fh = UiCompactH
  IF ThemeName = "classic" THEN
    pad = 14: ic = 32: w = 460: th = 20
  ELSE
    pad = 20: ic = 56: w = 480: th = 24
  END IF
  LockDialog.Font.Name = UIFont.Name: LockDialog.Font.Size = UIFont.Size
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  ' (the buttons first: their words and 12 each side at least — the
  ' dialog as wide as they need in Studio's language)
  mw = UITextWidth(LockMainBtn.Caption) + 32
  IF mw < 96 THEN mw = 96
  ow = UITextWidth(LockOffBtn.Caption) + 32
  IF ow < 96 THEN ow = 96
  cw = UITextWidth(LockCancelBtn.Caption) + 32
  IF cw < 96 THEN cw = 96
  need = mw + 8 + cw
  IF LockOffBtn.Visible THEN need = need + 8 + ow
  IF need > w THEN w = need
  LockTitle.Font = SubFont
  LockTitle.Font.Color = TC("page.text")
  LockLead.Font.Color = TC("page.text")
  LockWhatLabel.Font = UIFont
  LockWhatLabel.Font.Bold = 1
  LockWhatLabel.Font.Color = TC("page.text")
  LockFoot.Font.Color = TC("page.dim")
  LockHint.Font.Color = TC("page.dim")
  LockLink.Font = LinkFont
  LockLink.Font.Color = TC("page.link")
  LockLink.Font.Underline = 1
  IF LockLogoLoaded = 0 THEN
    LockLogoBmp.LoadFromFile HomeDir + "ide/assets/rapidr-studio.svg"
    LockLogo.BMPHandle = LockLogoBmp
    LockLogoLoaded = 1
  END IF
  LockLogo.Left = pad: LockLogo.Top = pad: LockLogo.Width = ic: LockLogo.Height = ic
  x = pad + ic + 16
  y = pad
  h = AISetLinesHigh(LockTitle.Caption, w) * 13 \ 10
  IF h < th THEN h = th
  LockTitle.Left = x: LockTitle.Top = y: LockTitle.Width = w: LockTitle.Height = h
  y = y + h + 8
  h = AISetLinesHigh(LockLead.Caption, w)
  LockLead.Left = x: LockLead.Top = y: LockLead.Width = w: LockLead.Height = h
  y = y + h + 16
  IF LockMode = "set" THEN
    LockWhatLabel.Left = x: LockWhatLabel.Top = y: LockWhatLabel.Width = w: LockWhatLabel.Height = 18
    y = y + 22
    LockAIBox.Left = x: LockAIBox.Top = y: LockAIBox.Width = w: LockAIBox.Height = 22
    y = y + 24
    LockAppsBox.Left = x: LockAppsBox.Top = y: LockAppsBox.Width = w: LockAppsBox.Height = 22
    y = y + 24
    LockCoachBox.Left = x: LockCoachBox.Top = y: LockCoachBox.Width = w: LockCoachBox.Height = 22
    y = y + 22 + 16
  END IF
  ' (the labels' column as wide as the longer label, 120 at least)
  lw = UITextWidth(LockPwLabel.Caption)
  IF LockPw2Label.Visible AND UITextWidth(LockPw2Label.Caption) > lw THEN lw = UITextWidth(LockPw2Label.Caption)
  lw = lw + 12
  IF lw < 120 THEN lw = 120
  IF LockPwEdit.Visible THEN
    LockPwLabel.Left = x: LockPwLabel.Top = y + (fh - 18) \ 2: LockPwLabel.Width = lw - 4: LockPwLabel.Height = 18
    LockPwEdit.Left = x + lw: LockPwEdit.Top = y: LockPwEdit.Width = w - lw: LockPwEdit.Height = fh
    y = y + fh + 8
  END IF
  IF LockPw2Edit.Visible THEN
    LockPw2Label.Left = x: LockPw2Label.Top = y + (fh - 18) \ 2: LockPw2Label.Width = lw - 4: LockPw2Label.Height = 18
    LockPw2Edit.Left = x + lw: LockPw2Edit.Top = y: LockPw2Edit.Width = w - lw: LockPw2Edit.Height = fh
    y = y + fh + 8
  END IF
  ' (the hint, under the field: as tall as two lines always, so nothing
  ' moves as it changes; no field, no hint)
  LockHint.Visible = LockPwEdit.Visible
  IF LockPwEdit.Visible THEN
    h = AISetLinesHigh(LockHint.Caption, w - lw)
    IF h < 36 THEN h = 36
    LockHint.Left = x + lw: LockHint.Top = y: LockHint.Width = w - lw: LockHint.Height = h
    y = y + h + 8
  END IF
  h = AISetLinesHigh(LockFoot.Caption, w)
  LockFoot.Left = x: LockFoot.Top = y: LockFoot.Width = w: LockFoot.Height = h
  y = y + h + 4
  ' (a link's text sits 4 inside its rectangle: the text on the column's edge)
  LockLink.AutoSize = 0
  LockLink.Width = UITextWidth(LockLink.Caption) + 10: LockLink.Height = 24
  LockLink.Left = x - 4: LockLink.Top = y
  y = y + 24 + 16
  ' (the buttons at the right: Turn Off, the main one, Cancel — Cancel at
  ' the edge, as every Studio dialog has it)
  LockCancelBtn.Width = cw: LockCancelBtn.Height = bh: LockCancelBtn.Top = y: LockCancelBtn.Left = x + w - cw
  LockMainBtn.Width = mw: LockMainBtn.Height = bh: LockMainBtn.Top = y: LockMainBtn.Left = LockCancelBtn.Left - 8 - mw
  LockOffBtn.Width = ow: LockOffBtn.Height = bh: LockOffBtn.Top = y: LockOffBtn.Left = LockMainBtn.Left - 8 - ow
  LockDialog.ClientWidth = x + w + pad
  LockDialog.ClientHeight = y + bh + pad
END SUB

' (as the password is typed: its strength, the two the same)
SUB LockPwChanged
  DIM s AS INTEGER
  IF LockMode <> "set" THEN EXIT SUB
  s = Proj.SchoolLockStrength(LockPwEdit.Text)
  IF LockPw2Edit.Text <> "" AND LockPw2Edit.Text <> LockPwEdit.Text THEN
    LockHint.Caption = TR$("The two passwords differ.")
  ELSE
    SELECT CASE s
      CASE 0: LockHint.Caption = TR$("At least 6 characters.")
      CASE 1: LockHint.Caption = TR$("Weak: easy to guess. A longer one with numbers or symbols is safer.")
      CASE 2: LockHint.Caption = TR$("Fair. Longer, with capitals, numbers or symbols, is stronger.")
      CASE ELSE: LockHint.Caption = TR$("Strong.")
    END SELECT
  END IF
END SUB

' (the hint line said, and to a screen reader)
SUB LockSay(Text AS STRING)
  LockHint.Caption = Text
  Application.Announce Text
END SUB

FUNCTION LockTimeText(S AS INTEGER) AS STRING
  Result = Num(S \ 60) + ":" + RIGHT$("0" + Num(S MOD 60), 2)
END FUNCTION

' (a wait running: the countdown, Unlock and Turn Off waiting for it)
SUB LockShowWait
  DIM s AS INTEGER
  s = VAL(Proj.SchoolLock("wait"))
  IF s > 0 AND LockMode <> "set" THEN
    LockHint.Caption = TR$("Too many wrong passwords. Try again in {time}.", "time", LockTimeText(s))
    LockMainBtn.Enabled = Flag(LockMode = "unlocked")
    LockOffBtn.Enabled = 0
    LockTimer.Enabled = 1
  ELSE
    LockMainBtn.Enabled = 1
    LockOffBtn.Enabled = 1
    IF LockTimer.Enabled THEN
      LockTimer.Enabled = 0
      LockSay TR$("You can try the password again.")
    END IF
  END IF
END SUB

SUB LockTick
  LockShowWait
END SUB

' What a password did, said in the hint line (and to a screen reader);
' True when it was right.
FUNCTION LockTry(Outcome AS INTEGER) AS INTEGER
  DIM n AS INTEGER
  LockAnswer = Outcome
  SELECT CASE Outcome
    CASE 1
      LockHint.Caption = ""
      Result = 1
    CASE 0
      n = VAL(Proj.SchoolLock("left"))
      IF VAL(Proj.SchoolLock("wait")) > 0 THEN
        LockShowWait
        Application.Announce LockHint.Caption
      ELSE
        LockSay TRN$("That isn't the password. {n} more try before a one-minute wait.", "That isn't the password. {n} more tries before a one-minute wait.", n)
      END IF
    CASE -1
      LockShowWait
      Application.Announce LockHint.Caption
    CASE -2: LockSay TR$("Your school's IT set this lock: only IT can turn it off.")
    CASE ELSE: LockSay TR$("No password can unlock this lock: ask your school's IT.")
  END SELECT
END FUNCTION

' Lock / Unlock Until Studio Closes / Lock Again.
SUB LockMainClick
  DIM f AS STRING
  SELECT CASE LockMode
    CASE "set"
      IF LockAIBox.Checked THEN f = "ai"
      IF LockAppsBox.Checked THEN
        IF f <> "" THEN f = f + ","
        f = f + "ai_apps"
      END IF
      ' (Coach kept on: only when RAI itself isn't locked)
      IF LockCoachBox.Checked AND LockAIBox.Checked = 0 THEN
        IF f <> "" THEN f = f + ","
        f = f + "coach"
      END IF
      IF f = "" THEN
        LockSay TR$("Choose at least one feature to lock.")
        EXIT SUB
      END IF
      IF Proj.SchoolLockStrength(LockPwEdit.Text) = 0 THEN
        LockSay TR$("The password needs at least 6 characters.")
        LockPwEdit.SetFocus
        EXIT SUB
      END IF
      IF LockPwEdit.Text <> LockPw2Edit.Text THEN
        LockSay TR$("The two passwords differ: type the same one twice.")
        LockPw2Edit.SetFocus
        EXIT SUB
      END IF
      LockSetOn LockPwEdit.Text, f
      IF LockIsOn THEN LockDialog.Close
    CASE "locked"
      IF LockPwEdit.Text = "" THEN
        LockSay TR$("Type the password.")
        EXIT SUB
      END IF
      LockUnlockWith LockPwEdit.Text
      IF LockAnswer = 1 THEN
        LockDialog.Close
      ELSE
        LockPwEdit.Text = ""
        LockPwEdit.SetFocus
      END IF
    CASE "unlocked"
      LockAgain
      LockDialog.Close
  END SELECT
END SUB

' (AI in Studio locked: RAI is off, so Coach mode has nothing to keep —
' its box waits until AI in Studio is unchecked)
SUB LockBoxesChanged
  LockCoachBox.Enabled = Flag(LockAIBox.Checked = 0)
  IF LockAIBox.Checked THEN LockCoachBox.Checked = 0
END SUB

' Turn School Lock Off: the password again.
SUB LockOffClick
  IF LockPwEdit.Text = "" THEN
    LockSay TR$("Type the password to turn School Lock off.")
    LockPwEdit.SetFocus
    EXIT SUB
  END IF
  LockOffWith LockPwEdit.Text
  IF LockAnswer = 1 THEN
    LockDialog.Close
  ELSE
    LockPwEdit.Text = ""
    LockPwEdit.SetFocus
  END IF
END SUB

SUB LockCancelClick
  LockDialog.Close
END SUB

' (as the dialog shows, and after each mode change: the password's field,
' else the main button)
SUB LockFocusFirst
  IF LockPwEdit.Visible THEN LockPwEdit.SetFocus ELSE LockMainBtn.SetFocus
END SUB

SUB LockLinkClick
  OpenWebLink "https://github.com/iBobX/RapidR/blob/main/docs/manual/for-teachers.md"
END SUB

' ---- what the dialog's buttons do (the test steps too) ----------------------------------

SUB LockSetOn(Password AS STRING, Features AS STRING)
  IF Proj.SchoolLockTurnOn(Password, Features) = 0 THEN
    LockSay TR$("School Lock couldn't be turned on: {error}", "error", Proj.Error)
    EXIT SUB
  END IF
  KeepSchoolLock
  SchoolLockApply
END SUB

SUB LockUnlockWith(Password AS STRING)
  IF LockTry(Proj.SchoolLockUnlock(Password)) THEN SchoolLockApply
END SUB

SUB LockOffWith(Password AS STRING)
  IF LockTry(Proj.SchoolLockTurnOff(Password)) THEN
    KeepSchoolLock
    SchoolLockApply
  END IF
END SUB

SUB LockAgain
  Proj.SchoolLockRelock
  SchoolLockApply
END SUB

' (a test step lock:<what>[=<value>])
SUB SchoolLockStep(Stp AS STRING)
  DIM what AS STRING, arg AS STRING, k AS INTEGER, f AS STRING
  k = INSTR(Stp, "=")
  IF k > 0 THEN
    what = LEFT$(Stp, k - 1): arg = MID$(Stp, k + 1)
  ELSE
    what = Stp
  END IF
  SELECT CASE what
    CASE "set"
      ' (set=<password>[|<features>]: as Lock in the dialog — every feature
      ' unless named)
      f = FIELD$(arg, "|", 2)
      IF f = "" THEN f = "ai,ai_apps"
      LockMode = "set"
      LockSetOn FIELD$(arg, "|", 1), f
    CASE "unlock"
      LockMode = "locked"
      LockUnlockWith arg
      OutputLine "School Lock answered: " + Num(LockAnswer) + " — " + LockHint.Caption
    CASE "off"
      LockMode = "locked"
      LockOffWith arg
      OutputLine "School Lock answered: " + Num(LockAnswer) + " — " + LockHint.Caption
    CASE "again": LockAgain
    CASE "dialog": ShowSchoolLock
    CASE "state"
      OutputLine "School Lock state: on=" + Proj.SchoolLock("on") + " unlocked=" + Proj.SchoolLock("unlocked") + " policy=" + Proj.SchoolLock("policy") + " features=" + Proj.SchoolLock("features") + " ai=" + Num(LockOffAI) + " ai_apps=" + Num(LockOffApps) + " wait=" + Proj.SchoolLock("wait") + " left=" + Proj.SchoolLock("left") + " mcp=" + Num(Mcp.Listening)
      IF LockKeepsCoach THEN OutputLine "School Lock keeps Coach: 1"
      ' (the AI commands as the palette and the menu have them)
      k = CmdIndex("ai.newChat")
      IF k >= 0 THEN
        IF Cmd(k).Item >= 0 THEN OutputLine "School Lock commands: newchat palette=" + Num(Flag(Palette.CommandEnabled("ai.newChat") <> 0)) + " menu=" + Num(Flag(MI(Cmd(k).Item).Enabled <> 0)) + " hint=" + MI(Cmd(k).Item).Hint
      END IF
  END SELECT
END SUB
