' aicoach.inc: Coach mode (docs/learn-with-rai-plan.md §1, the manual's
' "Coach mode" in docs/manual/ai-assistant.md) — RAI teaches instead of
' writing the answer.
'
' The dial, always in RAI's pane under the model: Build (RAI writes the
' change and you apply it, as before) or Coach. Pair and Quiz me come
' later: the dial has room for them. It is kept per user (AIMode in
' Studio's settings), changeable any time, by keyboard (Tab to a segment,
' Enter or Space) and said to a screen reader.
'
' In Coach, RAI never writes the answer into your code: rapidr-studio's
' coach.rs keeps the hint ladder and the tools that change files are never
' offered to the model (refused if it calls one anyway: enforced in code).
' Each problem climbs, only when asked: 1 a nudge → 2 the idea → 3 the
' shape of the code → 4 the code, shown as text for you to type or copy.
' The strip under the chat says which hint you're on ("Hint 2 of 4"), with
' Next Hint and I Got It. A compiler error or a run-time error gets Help
' With This Error: what it means, why, and a hint to fix it yourself —
' the code only after the hints.
'
' School Lock can keep RAI in Coach ("Keep RAI in Coach mode"; the
' computer's policy coach_only = true): the dial shows Coach with the lock
' note, Build can't be chosen — and the tools hold it whatever the pane
' shows (AITools.Mode reads "coach").
'
' Languages: every text here through TR$ (Studio's language); the hints
' themselves follow RAI's answer language (AI.ReplyLanguage: "auto" today —
' the user's latest message, else Studio's). The buttons' own messages
' (Give me the next hint., Help me with this error: …) are in Studio's
' language, so a hint asked with them comes in it. HOOK for ES-REFERENCE's
' "RAI answers in: Studio's language / the language I write in": it sets
' AI.ReplyLanguage, and Coach follows with nothing to change here.
'
' Test steps (ai:coach:<what>[=<value>]): ai:coach:mode=build|coach (the
' dial clicked), ai:coach:next, ai:coach:gotit, ai:coach:error,
' ai:coach:state (the state in the Output).

DECLARE SUB AIDialBuildClick
DECLARE SUB AIDialCoachClick
DECLARE SUB AIDialPaint(Sender AS RCanvas)
DECLARE SUB AICoachNextClick
DECLARE SUB AICoachGotClick
DECLARE SUB AICoachErrorClick
DECLARE SUB AIDialPick(Mode AS STRING)

' (the user's choice on the dial: "build" or "coach"; School Lock may keep
' it on Coach whatever this says — AICoachNow)
DIM AIMode AS STRING
' (the dial: its track and two segments — each a focusable panel, a
' painted pill when chosen, its icon and its name)
DIM AIDialTrack AS RCanvas
DIM AIDialBuild AS RPanel
DIM AIDialBuildBg AS RCanvas
DIM AIDialBuildIcon AS RImage
DIM AIDialBuildBmp AS RBitmap
DIM AIDialBuildText AS RLabel
DIM AIDialCoach AS RPanel
DIM AIDialCoachBg AS RCanvas
DIM AIDialCoachIcon AS RImage
DIM AIDialCoachBmp AS RBitmap
DIM AIDialCoachText AS RLabel
' (School Lock keeps Coach on: a lock and a line beside the dial)
DIM AIDialLockIcon AS RImage
DIM AIDialLockBmp AS RBitmap
DIM AIDialLockText AS RLabel
' (the hint strip under the chat: the rung's chip, Next Hint, I Got It;
' Help With This Error when there is an error and no hint open)
DIM AICoachBar AS RPanel
DIM AICoachChip AS RPanel
DIM AICoachChipBg AS RCanvas
DIM AICoachChipIcon AS RImage
DIM AICoachChipBmp AS RBitmap
DIM AICoachChipText AS RLabel
DIM AICoachNext AS RButton
DIM AICoachGot AS RButton
DIM AICoachErr AS RButton
' (the last run's run-time error, as Output says it: "main.rr:12: …")
DIM AICoachRunError AS STRING
' (the state last announced: said once)
DIM AICoachSaid AS STRING

' ---- the state ------------------------------------------------------------------

' Coach now: the dial's, or School Lock's.
FUNCTION AICoachNow AS INTEGER
  Result = Flag(AITools.Mode = "coach")
END FUNCTION

' School Lock keeps RAI in Coach.
FUNCTION AICoachLocked AS INTEGER
  Result = Flag(AITools.CoachLocked <> 0)
END FUNCTION

' (a rung in words, for the chip's spoken name)
FUNCTION AICoachRungWhat(R AS INTEGER) AS STRING
  SELECT CASE R
    CASE 1: Result = TR$("a nudge")
    CASE 2: Result = TR$("the idea")
    CASE 3: Result = TR$("the shape of the code")
    CASE ELSE: Result = TR$("the code")
  END SELECT
END FUNCTION

' The error a learner would ask about: the program's first compiler error
' ("main.rr:12: …"), else the last run's run-time error; "" none.
FUNCTION AICoachErrorText AS STRING
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING
  IF ProjectOpen AND ProjectMain <> "" THEN
    text = ProblemsText(ProjectMain)
    n = TALLY(text, CHR$(10))
    FOR i = 1 TO n
      row = FIELD$(text, CHR$(10), i)
      IF FIELD$(row, CHR$(9), 1) = "error" THEN
        Result = ProjectRel(FIELD$(row, CHR$(9), 5)) + ":" + FIELD$(row, CHR$(9), 2) + ": " + FIELD$(row, CHR$(9), 4)
        EXIT FUNCTION
      END IF
    NEXT
  END IF
  Result = AICoachRunError
END FUNCTION

' (debug.inc: a run stopped at a run-time error; Run again forgets it)
SUB AICoachSawRunError(Text AS STRING)
  AICoachRunError = Text
  AICoachShow
END SUB

SUB AICoachRunStarted
  AICoachRunError = ""
END SUB

' ---- setting up -----------------------------------------------------------------

' (SetUpAI: the dial in the top strip, the hint strip above the box — made
' after the review bar, so it sits right under the chat)
SUB SetUpAIDial
  IF AIMode <> "coach" THEN AIMode = "build"
  AIDialTrack.Parent = AITop
  AIDialTrack.AccessibleHidden = 1
  AIDialTrack.OnPaint = AIDialPaint
  AIDialTrack.Tag = -1
  AIDialBuild.Parent = AITop
  AIDialBuild.BevelOuter = 0
  AIDialBuild.TabStop = 1
  AIDialBuild.OnClick = AIDialBuildClick
  AIDialBuildBg.Parent = AIDialBuild
  AIDialBuildBg.AccessibleHidden = 1
  AIDialBuildBg.Tag = 0
  AIDialBuildBg.OnPaint = AIDialPaint
  AIDialBuildBg.OnClick = AIDialBuildClick
  AIDialBuildIcon.Parent = AIDialBuild
  AIDialBuildIcon.Width = 16: AIDialBuildIcon.Height = 16
  AIDialBuildIcon.AccessibleHidden = 1
  AIDialBuildIcon.OnClick = AIDialBuildClick
  AIDialBuildText.Parent = AIDialBuild
  AIDialBuildText.Transparent = 1
  AIDialBuildText.AccessibleHidden = 1
  AIDialBuildText.Caption = TR$("Build")
  AIDialBuildText.OnClick = AIDialBuildClick
  AIDialCoach.Parent = AITop
  AIDialCoach.BevelOuter = 0
  AIDialCoach.TabStop = 1
  AIDialCoach.OnClick = AIDialCoachClick
  AIDialCoachBg.Parent = AIDialCoach
  AIDialCoachBg.AccessibleHidden = 1
  AIDialCoachBg.Tag = 1
  AIDialCoachBg.OnPaint = AIDialPaint
  AIDialCoachBg.OnClick = AIDialCoachClick
  AIDialCoachIcon.Parent = AIDialCoach
  AIDialCoachIcon.Width = 16: AIDialCoachIcon.Height = 16
  AIDialCoachIcon.AccessibleHidden = 1
  AIDialCoachIcon.OnClick = AIDialCoachClick
  AIDialCoachText.Parent = AIDialCoach
  AIDialCoachText.Transparent = 1
  AIDialCoachText.AccessibleHidden = 1
  AIDialCoachText.Caption = TR$("Coach")
  AIDialCoachText.OnClick = AIDialCoachClick
  AIDialLockIcon.Parent = AITop
  AIDialLockIcon.Width = 16: AIDialLockIcon.Height = 16
  AIDialLockIcon.AccessibleHidden = 1
  AIDialLockIcon.Visible = 0
  AIDialLockText.Parent = AITop
  AIDialLockText.AutoSize = 0: AIDialLockText.WordWrap = 0
  AIDialLockText.Visible = 0
END SUB

SUB SetUpAICoachBar
  AICoachBar.Parent = AIPanel
  AICoachBar.Align = 2
  AICoachBar.Height = 40
  AICoachBar.BevelOuter = 0
  AICoachBar.Visible = 0
  AICoachBar.AccessibleName = TR$("Coach hints")
  ' (the rung's chip: info, soft; its parts hidden — the chip says it all)
  AICoachChip.Parent = AICoachBar
  AICoachChip.BevelOuter = 0
  AICoachChipBg.Parent = AICoachChip
  AICoachChipBg.Left = 0: AICoachChipBg.Top = 0
  AICoachChipBg.Tag = 0
  AICoachChipBg.OnPaint = ChipPaint
  AICoachChipBg.AccessibleHidden = 1
  AICoachChipIcon.Parent = AICoachChip
  AICoachChipIcon.Width = 16: AICoachChipIcon.Height = 16
  AICoachChipIcon.AccessibleHidden = 1
  AICoachChipText.Parent = AICoachChip
  AICoachChipText.Transparent = 1
  AICoachChipText.AccessibleHidden = 1
  AICoachNext.Parent = AICoachBar
  AICoachNext.OnClick = AICoachNextClick
  AICoachGot.Parent = AICoachBar
  AICoachGot.Caption = TR$("I &Got It")
  AICoachGot.AccessibleDescription = TR$("Ends this problem: your next question starts again at the first hint")
  AICoachGot.OnClick = AICoachGotClick
  AICoachErr.Parent = AICoachBar
  AICoachErr.Caption = TR$("&Help With This Error")
  AICoachErr.OnClick = AICoachErrorClick
END SUB

' (AIPanelFonts: the look's fonts and colours)
SUB AICoachFonts
  DIM f AS STRING
  AIDialTrack.Color = TC("face")
  AIDialBuild.Color = TC("window")
  AIDialCoach.Color = TC("window")
  AIDialBuildBg.Color = TC("window")
  AIDialCoachBg.Color = TC("window")
  AIDialBuildBmp.LoadIcon "build", 16
  AIDialBuildIcon.BMPHandle = AIDialBuildBmp
  AIDialCoachBmp.LoadIcon "hint", 16
  AIDialCoachIcon.BMPHandle = AIDialCoachBmp
  AIDialLockBmp.LoadIcon "lock", 16
  AIDialLockIcon.BMPHandle = AIDialLockBmp
  AIDialLockText.Font = DimFont
  AIDialLockText.Font.Color = TC("page.dim")
  AICoachBar.Color = TC("card")
  AICoachChip.Color = TC("card")
  AICoachChipBg.Color = TC("card")
  AICoachChipText.Font = UIFont
  AICoachChipText.Font.Color = TC("status.info.text")
  AICoachChipBmp.LoadIcon "hint", 16
  AICoachChipIcon.BMPHandle = AICoachChipBmp
  AICoachNext.Font = UIFont
  AICoachGot.Font = UIFont
  AICoachErr.Font = UIFont
  AIDialShow
END SUB

' ---- the dial -----------------------------------------------------------------------

' The track: a soft rounded field; a segment: the pill when chosen (the
' accent's edge, the soft selection inside), the window's ground when not.
SUB AIDialPaint(Sender AS RCanvas)
  DIM w AS INTEGER, h AS INTEGER, chosen AS INTEGER
  w = Sender.Width: h = Sender.Height
  Sender.FillRect 0, 0, w, h, Sender.Color
  IF Sender.Tag < 0 THEN
    Sender.RoundRect 0, 0, w, h, h, h, TC("border")
    Sender.RoundRect 1, 1, w - 1, h - 1, h - 2, h - 2, TC("window")
    EXIT SUB
  END IF
  IF Sender.Tag = 1 THEN chosen = AICoachNow ELSE chosen = Flag(AICoachNow = 0)
  IF chosen THEN
    Sender.RoundRect 0, 0, w, h, h, h, TC("accent")
    Sender.RoundRect 1, 1, w - 1, h - 1, h - 2, h - 2, TC("selected")
  END IF
END SUB

' The dial as the mode is now: the chosen segment's pill, the names and
' what each does for a screen reader, the lock's note.
SUB AIDialShow
  DIM coach AS INTEGER, locked AS INTEGER
  coach = AICoachNow
  locked = AICoachLocked
  IF coach THEN
    AIDialBuild.AccessibleName = TR$("Build")
    AIDialCoach.AccessibleName = TR$("Coach, chosen")
  ELSE
    AIDialBuild.AccessibleName = TR$("Build, chosen")
    AIDialCoach.AccessibleName = TR$("Coach")
  END IF
  AIDialBuild.AccessibleDescription = TR$("RAI writes changes for you to review and apply")
  AIDialCoach.AccessibleDescription = TR$("RAI gives hints one step at a time and never writes the answer into your code")
  AIDialBuildText.Font = UIFont
  AIDialCoachText.Font = UIFont
  AIDialBuildText.Font.Bold = Flag(coach = 0)
  AIDialCoachText.Font.Bold = coach
  IF coach THEN
    AIDialBuildText.Font.Color = TC("page.dim")
    AIDialCoachText.Font.Color = TC("selected_text")
  ELSE
    AIDialBuildText.Font.Color = TC("selected_text")
    AIDialCoachText.Font.Color = TC("page.dim")
  END IF
  ' (School Lock keeps Coach: Build can't be chosen, the reason beside it)
  AIDialBuild.Enabled = Flag(locked = 0)
  AIDialBuildText.Enabled = Flag(locked = 0)
  AIDialLockIcon.Visible = locked
  AIDialLockText.Visible = locked
  IF locked THEN
    IF LockByPolicy THEN AIDialLockText.Caption = TR$("Your school keeps RAI in Coach mode.") ELSE AIDialLockText.Caption = TR$("Your teacher keeps RAI in Coach mode.")
    AIDialBuild.Hint = AIDialLockText.Caption
    AIDialBuild.ShowHint = 1
  ELSE
    AIDialBuild.ShowHint = 0
  END IF
  AI.InsertLinks = Flag(coach = 0)
  AIDialTrack.Repaint
  AIDialBuildBg.Repaint
  AIDialCoachBg.Repaint
  AICoachShow
END SUB

SUB AIDialBuildClick
  AIDialPick "build"
END SUB

SUB AIDialCoachClick
  AIDialPick "coach"
END SUB

' The dial turned: kept at once, the tools told (the ladder starts over),
' said once.
SUB AIDialPick(Mode AS STRING)
  DIM was AS INTEGER
  was = AICoachNow
  IF Mode = "build" AND AICoachLocked THEN
    AIStatus.SimpleText = AIDialLockText.Caption
    Application.Announce AIDialLockText.Caption
    EXIT SUB
  END IF
  AIMode = Mode
  AITools.Mode = AIMode
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("AIMode", AIMode)
  AIDialShow
  AIChat.EmptyText = AIEmptyText
  IF AICoachNow <> was THEN
    IF AICoachNow THEN
      AIStatus.SimpleText = TR$("Coach mode: RAI gives hints one step at a time. You write the code.")
    ELSE
      AIStatus.SimpleText = TR$("Build mode: RAI writes changes for you to review and apply.")
    END IF
    Application.Announce AIStatus.SimpleText
  END IF
END SUB

' (the mode from the settings, or School Lock changed: the tools and the dial)
SUB AIDialApply
  IF AIMode <> "coach" THEN AIMode = "build"
  AITools.Mode = AIMode
  AIDialShow
END SUB

' (LayOutAIPanel: the dial's row at Top; its height — with the lock's line
' when it shows and doesn't fit beside the dial)
FUNCTION LayOutAIDial(Top AS INTEGER, W AS INTEGER) AS INTEGER
  DIM ch AS INTEGER, sw AS INTEGER, x AS INTEGER, tw AS INTEGER, dw AS INTEGER, lw AS INTEGER
  ch = UiCompactH
  ' (each segment: 12, the icon, 6, the name, 12 — the wider name sets both)
  tw = UITextWidth(AIDialBuildText.Caption)
  IF UITextWidth(AIDialCoachText.Caption) > tw THEN tw = UITextWidth(AIDialCoachText.Caption)
  sw = 12 + 16 + 6 + tw + 4 + 12
  IF sw < 88 THEN sw = 88
  dw = 2 + sw + 2 + sw + 2
  x = 6
  AIDialTrack.Left = x: AIDialTrack.Top = Top: AIDialTrack.Width = dw: AIDialTrack.Height = ch
  AIDialBuild.Left = x + 2: AIDialBuild.Top = Top + 2: AIDialBuild.Width = sw: AIDialBuild.Height = ch - 4
  AIDialCoach.Left = x + 2 + sw + 2: AIDialCoach.Top = Top + 2: AIDialCoach.Width = sw: AIDialCoach.Height = ch - 4
  ' (the pill inside the segment: 1 in, so the focus ring shows around it)
  AIDialBuildBg.Left = 1: AIDialBuildBg.Top = 1: AIDialBuildBg.Width = sw - 2: AIDialBuildBg.Height = ch - 6
  AIDialCoachBg.Left = 1: AIDialCoachBg.Top = 1: AIDialCoachBg.Width = sw - 2: AIDialCoachBg.Height = ch - 6
  AIDialBuildIcon.Left = 12: AIDialBuildIcon.Top = (ch - 4 - 16) \ 2
  AIDialCoachIcon.Left = 12: AIDialCoachIcon.Top = (ch - 4 - 16) \ 2
  AIDialBuildText.Left = 34: AIDialBuildText.Top = (ch - 4 - 18) \ 2: AIDialBuildText.Width = sw - 34 - 6: AIDialBuildText.Height = 18
  AIDialCoachText.Left = 34: AIDialCoachText.Top = (ch - 4 - 18) \ 2: AIDialCoachText.Width = sw - 34 - 6: AIDialCoachText.Height = 18
  Result = ch
  IF AIDialLockText.Visible = 0 THEN EXIT FUNCTION
  ' (the lock's line: beside the dial when it fits, else under it)
  lw = UITextWidth(AIDialLockText.Caption) + 4
  IF x + dw + 8 + 16 + 4 + lw <= W - 6 THEN
    AIDialLockIcon.Left = x + dw + 8: AIDialLockIcon.Top = Top + (ch - 16) \ 2
    AIDialLockText.Left = x + dw + 8 + 20: AIDialLockText.Top = Top + (ch - 16) \ 2: AIDialLockText.Width = W - 6 - (x + dw + 28): AIDialLockText.Height = 16
  ELSE
    AIDialLockIcon.Left = x + 2: AIDialLockIcon.Top = Top + ch + 4
    AIDialLockText.Left = x + 22: AIDialLockText.Top = Top + ch + 4: AIDialLockText.Width = W - 6 - (x + 22): AIDialLockText.Height = 16
    Result = ch + 22
  END IF
END FUNCTION

' ---- the hint strip ------------------------------------------------------------------

' The strip as the ladder is now: Coach with a hint open — its chip, Next
' Hint (Show the Code after the third), I Got It —; Coach with none open
' and an error — Help With This Error —; else hidden.
SUB AICoachShow
  DIM rung AS INTEGER, err AS STRING, said AS STRING, show AS INTEGER
  IF AICoachNow = 0 THEN
    IF AICoachBar.Visible THEN AICoachBar.Visible = 0
    AICoachSaid = ""
    EXIT SUB
  END IF
  rung = AITools.CoachRung
  IF rung > 0 THEN
    AICoachChipText.Caption = TR$("Hint {n} of 4", "n", Num(rung))
    AICoachChip.AccessibleName = TR$("Hint {n} of 4: {what}", "n", Num(rung), "what", AICoachRungWhat(rung))
    IF rung >= 3 THEN AICoachNext.Caption = TR$("&Show the Code") ELSE AICoachNext.Caption = TR$("&Next Hint")
    IF rung >= 3 THEN
      AICoachNext.AccessibleDescription = TR$("The fourth hint: RAI shows the code, for you to type or copy in yourself")
    ELSE
      AICoachNext.AccessibleDescription = TR$("RAI gives the next hint: a little more each time")
    END IF
    AICoachChip.Visible = 1
    AICoachNext.Visible = Flag(rung < 4)
    AICoachGot.Visible = 1
    AICoachErr.Visible = 0
    show = 1
    said = AICoachChip.AccessibleName
  ELSE
    err = AICoachErrorText
    AICoachChip.Visible = 0
    AICoachNext.Visible = 0
    AICoachGot.Visible = 0
    AICoachErr.Visible = Flag(err <> "")
    AICoachErr.AccessibleDescription = TR$("RAI says what the error means, why it happens, and gives a hint to fix it yourself: {error}", "error", err)
    show = Flag(err <> "")
  END IF
  IF AICoachBar.Visible <> show THEN AICoachBar.Visible = show
  LayOutAICoach
  AICoachEnable
  ' (a new hint, said once; the parts of the chip repainted)
  AICoachChipBg.Repaint
  IF said <> "" AND said <> AICoachSaid THEN
    AICoachSaid = said
    Application.Announce said
  END IF
END SUB

' (Next Hint and the others only while RAI can be asked)
SUB AICoachEnable
  DIM can AS INTEGER
  can = Flag(AI.Busy = 0 AND AIReady <> 0 AND LockOffAI = 0)
  AICoachNext.Enabled = can
  AICoachErr.Enabled = can
END SUB

' The strip's parts for its width: the chip at the left; I Got It at the
' right edge, Next Hint before it — compact, on one centre line, 8 apart.
' A pane too narrow for all three: the buttons on a second row, at the
' right (the chip never runs under a button).
SUB LayOutAICoach
  DIM w AS INTEGER, ch AS INTEGER, y AS INTEGER, by AS INTEGER, x AS INTEGER, bw AS INTEGER, cw AS INTEGER
  DIM gw AS INTEGER, nw AS INTEGER, need AS INTEGER, hh AS INTEGER
  w = AIPanel.ClientWidth
  IF w < 200 THEN w = 200
  ch = UiCompactH
  y = (40 - ch) \ 2
  ' (the chip: 8, the icon, 6, the words, 12)
  AICoachChipText.AutoSize = 1
  cw = 8 + 16 + 6 + AICoachChipText.Width + 12
  gw = UITextWidth(TR$("I Got It")) + 24
  IF gw < 72 THEN gw = 72
  nw = UITextWidth(REPLACESUBSTR$(AICoachNext.Caption, "&", "")) + 24
  IF nw < 72 THEN nw = 72
  need = 8 + cw + 8
  IF AICoachGot.Visible THEN need = need + gw + 8
  IF AICoachNext.Visible THEN need = need + nw + 8
  by = y
  hh = 40
  IF AICoachChip.Visible AND need > w THEN
    by = 40 - 4
    hh = 40 - 4 + ch + 8
  END IF
  IF AICoachBar.Height <> hh THEN AICoachBar.Height = hh
  x = w - 8
  IF AICoachGot.Visible THEN
    x = x - gw
    AICoachGot.Left = x: AICoachGot.Top = by: AICoachGot.Width = gw: AICoachGot.Height = ch
    x = x - 8
  END IF
  IF AICoachNext.Visible THEN
    x = x - nw
    AICoachNext.Left = x: AICoachNext.Top = by: AICoachNext.Width = nw: AICoachNext.Height = ch
  END IF
  IF AICoachErr.Visible THEN
    bw = UITextWidth(REPLACESUBSTR$(AICoachErr.Caption, "&", "")) + 24
    IF bw > w - 16 THEN bw = w - 16
    AICoachErr.Left = 8: AICoachErr.Top = y: AICoachErr.Width = bw: AICoachErr.Height = ch
  END IF
  AICoachChip.Left = 8: AICoachChip.Top = y: AICoachChip.Width = cw: AICoachChip.Height = ch
  AICoachChipBg.Width = cw: AICoachChipBg.Height = ch
  AICoachChipIcon.Left = 8: AICoachChipIcon.Top = (ch - 16) \ 2
  AICoachChipText.Left = 30: AICoachChipText.Top = (ch - AICoachChipText.Height) \ 2
END SUB

' (a question to RAI from the strip, as if typed: shown in the chat)
SUB AICoachAsk(Text AS STRING)
  IF AI.Busy THEN EXIT SUB
  AISendNow Text, 0
  AICoachShow
END SUB

SUB AICoachNextClick
  DIM r AS INTEGER
  IF AI.Busy OR AICoachNow = 0 THEN EXIT SUB
  ' (a method's call, then the rung it left: a bare member read would be a
  ' property's)
  AITools.CoachNext
  r = AITools.CoachRung
  IF r >= 4 THEN AICoachAsk TR$("Show me the code.") ELSE AICoachAsk TR$("Give me the next hint.")
END SUB

SUB AICoachGotClick
  AITools.CoachReset
  AICoachSaid = ""
  AICoachShow
  AIStatus.SimpleText = TR$("Nice work. Ask about the next thing when you're ready.")
  Application.Announce AIStatus.SimpleText
  AIInput.SetFocus
END SUB

SUB AICoachErrorClick
  DIM e AS STRING
  e = AICoachErrorText
  IF e = "" OR AI.Busy OR AICoachNow = 0 THEN EXIT SUB
  AITools.CoachForError
  AICoachAsk TR$("Help me with this error: {error}", "error", e)
END SUB

' (ai.inc's AISendNow: a question typed in Coach — a new problem on the
' first hint, or the open one's)
SUB AICoachTyped
  IF AICoachNow THEN AITools.CoachAsk
END SUB

' (a new chat: no hint open)
SUB AICoachNewChat
  AITools.CoachReset
  AICoachSaid = ""
  AICoachShow
END SUB

' (what the empty chat says in Coach)
FUNCTION AICoachEmptyText AS STRING
  Result = TR$("Coach mode: ask about your code, or about what you want to make. RAI gives one hint at a time — a nudge, the idea, the shape of the code — and shows code only when you ask a fourth time. You write the code yourself.")
END FUNCTION

' ---- test steps ---------------------------------------------------------------------

' ai:coach:<what>[=<value>]: True when Verb was one.
FUNCTION AICoachStep(Verb AS STRING, Arg AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Verb
    CASE "coach:mode": AIDialPick Arg
    CASE "coach:next": AICoachNextClick
    CASE "coach:gotit": AICoachGotClick
    CASE "coach:error": AICoachErrorClick
    CASE "coach:focus"
      ' (the keyboard on a segment, as Tab leaves it)
      IF Arg = "build" THEN AIDialBuild.SetFocus ELSE AIDialCoach.SetFocus
    CASE "coach:state"
      OutputLine "Coach state: mode=" + AITools.Mode + " chosen=" + AIMode + " locked=" + Num(AICoachLocked) + " rung=" + Num(AITools.CoachRung) + " error=" + Num(AITools.CoachError) + " bar=" + Num(AICoachBar.Visible) + " next=" + Num(AICoachNext.Visible) + " insert=" + Num(AI.InsertLinks) + " proposals=" + Num(AITools.ProposalCount) + " review=" + Num(AIReviewBar.Visible) + " errtext=" + AICoachErrorText
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION
