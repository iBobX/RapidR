' notify.inc: Studio's notifications (lane NOTIFY; the rules are in
' docs/design-guidelines.md, "Notifications").
'
'   - Notify (RNotify): the toasts — small messages that say what happened
'     and get out of the way. They sit in the corner of the editor area, at
'     the left of the editor's scroll bar and above its horizontal one
'     (Dock.DocumentLeft … and SetArea), three at most, merged by key, with
'     the time their words need (a person's setting: Tools > Settings);
'   - the bell at the status bar's right end: the number of messages not
'     looked at yet; it opens the history (Ctrl+Shift+N does too), where
'     every message is kept with its time and its action again;
'   - ListPicker (an RCommandPalette): a searchable list of files — what
'     "Show" opens on "5 files were reloaded", and the list of open tabs;
'   - a question the person must answer is not a toast: the strip on its
'     editor (RNotifyBar: external.inc).
' What a toast says is never a secret: its words are scrubbed as the Output
' pane's are before they are shown or kept.

DECLARE SUB NotifyAction(Id AS INTEGER, Index AS INTEGER, Key AS STRING)
DECLARE SUB NotifyClosed(Id AS INTEGER, Reason AS STRING, Key AS STRING)
DECLARE SUB NotifyUnseen(Count AS INTEGER)
DECLARE SUB NotifyEntered
DECLARE SUB NotifyLeft
DECLARE SUB BellClick
DECLARE SUB ListPicked(Id AS STRING)
DECLARE SUB LayOutNotify
DECLARE SUB BellShow
DECLARE SUB BellPlace
DECLARE SUB BellColour
DECLARE SUB OpenNotifications
DECLARE FUNCTION Toast(Text AS STRING, Kind AS STRING, Action AS STRING, Key AS STRING) AS INTEGER

' (the keyboard is in the toasts: Next Area reaches them, Escape leaves)
DIM NotifyFocused AS INTEGER
DIM BellBmp AS RBitmap
' (tests: notify:press=when waited this many steps for a toast)
DIM NotifyWaits AS INTEGER
' (what a list is for: "files" — what a batch updated —, "tabs")
DIM ListPurpose AS STRING
'' (Settings > General > Notifications stay: 0 as long as the words need,
' 1 longer, 2 until closed — NotifyStay, decl.inc)

' The toasts: a see-through overlay over the whole window that takes the
' mouse only where a toast is.
CREATE Notify AS RNotify
  Align = 0
  OnAction = NotifyAction
  OnClosed = NotifyClosed
  OnUnseenChange = NotifyUnseen
  OnEnter = NotifyEntered
  OnExit = NotifyLeft
END CREATE

' The bell at the status bar's right end: the bell and, when there are
' messages not looked at, their number in a soft pill.
CREATE BellBox AS RPanel
  BevelOuter = 0
  Cursor = -21
  OnClick = BellClick
  CREATE BellChip AS RCanvas
    Tag = 0
    OnPaint = ChipPaint
    OnClick = BellClick
    AccessibleHidden = 1
    Visible = 0
  END CREATE
  CREATE BellIcon AS RImage
    Width = 12: Height = 12
    OnClick = BellClick
    AccessibleHidden = 1
  END CREATE
  CREATE BellCount AS RLabel
    Transparent = 1
    OnClick = BellClick
    AccessibleHidden = 1
    Visible = 0
  END CREATE
END CREATE

' The files a toast's Show lists, and the tabs' list: a searchable card.
CREATE ListPicker AS RCommandPalette
  Placeholder = "Search"
  OnCommand = ListPicked
END CREATE

SUB SetUpNotify
  ' (the list button at the right end of the document tabs: every open tab,
  ' searchable, hanging from the button)
  Dock.OnDocumentList = OpenTabsList
  Notify.Parent = Studio
  BellBox.Parent = Status
  BellChip.Parent = BellBox
  BellIcon.Parent = BellBox
  BellCount.Parent = BellBox
  ListPicker.Parent = Studio
  LayOutNotify
  BellShow
END SUB

SUB ApplyNotifyChrome
  Notify.Font = UIFont
  Notify.AccessibleName = TR$("Notifications")
  ListPicker.Font = UIFont
  ListPicker.Placeholder = TR$("Search")
  BellColour
  BellCount.Font = StatusFont
  BellCount.Font.Color = TC("status.info.text")
  ApplyNotifyStay
  BellShow
END SUB

' (Tools > Settings: how long toasts stay — the guide's WCAG 2.2.1)
SUB ApplyNotifyStay
  SELECT CASE NotifyStay
    CASE 1: Notify.TimeScale = 200
    CASE 2: Notify.TimeScale = 0
    CASE ELSE: Notify.TimeScale = 100
  END SELECT
END SUB

' Where the toasts sit: the window's client area is the overlay, the
' toasts' corner is the editor area's — at the left of the editor's scroll
' bar (17) and its minimap (90, when it shows), above its horizontal bar.
SUB LayOutNotify
  DIM d AS INTEGER, mini AS INTEGER, w AS INTEGER, h AS INTEGER
  Notify.Left = 0: Notify.Top = 0
  Notify.Width = Studio.ClientWidth: Notify.Height = Studio.ClientHeight
  w = Dock.DocumentWidth
  h = Dock.DocumentHeight
  IF w > 0 AND h > 0 THEN
    mini = 0
    d = ActiveCode
    IF d >= 0 THEN
      IF CodeDoc(d).ShowMinimap THEN mini = 90
    END IF
    Notify.SetArea Dock.Left + Dock.DocumentLeft, Dock.Top + Dock.DocumentTop, w - 17 - mini, h - 17
  ELSE
    Notify.SetArea 0, 0, 0, 0
  END IF
  BellPlace
END SUB

' A toast: Text (already in Studio's language), its Kind ("info",
' "success", "warning", "error"), one or two actions ("Show", "Run It|Undo")
' and its Key (a toast with the same key on screen takes the new words).
' Its number.
FUNCTION Toast(Text AS STRING, Kind AS STRING, Action AS STRING, Key AS STRING) AS INTEGER
  LayOutNotify
  Result = Notify.Show(Text, Kind, Action, -1, Key)
END FUNCTION

' Up to three names of a list of paths (a line each), then "+N".
FUNCTION NotifyNames(Paths AS STRING) AS STRING
  DIM n AS INTEGER, i AS INTEGER
  n = TALLY(Paths, CHR$(10))
  FOR i = 1 TO n
    IF i <= 3 THEN
      IF i > 1 THEN Result = Result + ", "
      Result = Result + FileTitle(FIELD$(Paths, CHR$(10), i))
    END IF
  NEXT
  IF n > 3 THEN Result = Result + " +" + LTRIM$(STR$(n - 3))
END FUNCTION

' ---- the events ----------------------------------------------------------------------------

' A toast's action (or one in the history): by its key, the feature that
' said it does it.
SUB NotifyAction(Id AS INTEGER, Index AS INTEGER, Key AS STRING)
  IF ExtToastAction(Key, Index) THEN EXIT SUB
  IF AIChatToastAction(Key, Index) THEN EXIT SUB
  IF AIMadeToastAction(Key, Index) THEN EXIT SUB
END SUB

' A toast went. The keyboard was on it: back to the editor.
SUB NotifyClosed(Id AS INTEGER, Reason AS STRING, Key AS STRING)
  IF NotifyFocused AND Notify.Count = 0 AND Notify.HistoryOpen = 0 THEN
    IF ActiveCode >= 0 THEN FocusCodeSoon ActiveCode
  END IF
END SUB

SUB NotifyUnseen(Count AS INTEGER)
  BellShow
END SUB

SUB NotifyEntered
  NotifyFocused = 1
END SUB

SUB NotifyLeft
  NotifyFocused = 0
END SUB

' ---- the bell ----------------------------------------------------------------------------------

' (the room the bell takes at the status bar's right end)
FUNCTION BellRoom AS INTEGER
  Result = 44
END FUNCTION

' The bell on the bar's colour of the moment: while a program runs, is paused or
' stopped on an error the bar is that state's colour (ApplyState), so the bell's box
' takes it and its icon is the light one, as the bar's words are, never a light patch
' on a coloured bar. (The classic and high contrast looks keep their bar.)
SUB BellColour
  BellBox.Color = Status.Color
  BellChip.Color = Status.Color
  IF Status.Color <> TC("statusbar") THEN
    BellBmp.LoadIcon "notifications", 12, "rapidr high contrast"
  ELSE
    BellBmp.LoadIcon "notifications", 12
  END IF
  BellIcon.BMPHandle = BellBmp
  BellChip.Repaint
END SUB

SUB BellShow
  DIM n AS INTEGER, s AS STRING, w AS INTEGER
  n = Notify.UnseenCount
  BellColour
  IF n > 0 THEN
    IF n > 9 THEN s = "9+" ELSE s = LTRIM$(STR$(n))
    BellCount.Caption = s
    BellChip.Visible = 1
    BellCount.Visible = 1
    BellBox.AccessibleName = TRN$("Notifications: {n} new", "Notifications: {n} new", n)
  ELSE
    BellChip.Visible = 0
    BellCount.Visible = 0
    BellBox.AccessibleName = TR$("Notifications")
  END IF
  BellPlace
  BellChip.Repaint
END SUB

' The bell and its pill on the status bar's centre line, at the far right.
SUB BellPlace
  DIM w AS INTEGER, h AS INTEGER, cw AS INTEGER, ch AS INTEGER
  StatusMeasure.Font.Name = StatusFont.Name: StatusMeasure.Font.Size = StatusFont.Size
  w = BellRoom - 6
  h = Status.Height - 2
  IF h > UiCompactH THEN h = UiCompactH
  IF h < 16 THEN h = 16
  ' (left of the status bar's size grip)
  BellBox.Left = Studio.ClientWidth - w - 20: BellBox.Top = (Status.Height - h) \ 2
  BellBox.Width = w: BellBox.Height = h
  BellIcon.Left = 4: BellIcon.Top = (h - 12) \ 2
  IF BellChip.Visible THEN
    cw = StatusMeasure.TextWidth(BellCount.Caption) + 10
    IF cw < 16 THEN cw = 16
    ch = 14
    IF ch > h THEN ch = h
    BellChip.Left = 20: BellChip.Top = (h - ch) \ 2: BellChip.Width = cw: BellChip.Height = ch
    BellCount.Left = 20 + (cw - StatusMeasure.TextWidth(BellCount.Caption)) \ 2
    BellCount.Top = (h - BellCount.Height) \ 2
  END IF
END SUB

' The history: the card above the bell, hanging from its corner.
SUB BellClick
  OpenNotifications
END SUB

SUB OpenNotifications
  LayOutNotify
  IF Notify.HistoryOpen THEN
    Notify.HideHistory
  ELSE
    Notify.ShowHistory Studio.ClientWidth - 8, Status.Top - 6
  END IF
  BellShow
END SUB

' ---- the files' list ---------------------------------------------------------------------------

' What a list picked: a file to show (what a batch updated, an open tab).
SUB ListPicked(Id AS STRING)
  IF Id <> "" THEN OpenDocument Id
  IF ActiveCode >= 0 THEN FocusCodeSoon ActiveCode
END SUB

' The list of files (a line each, full paths) in the searchable card.
SUB ShowFileList(Paths AS STRING)
  DIM n AS INTEGER, i AS INTEGER, p AS STRING
  ListPicker.Clear
  n = TALLY(Paths, CHR$(10))
  FOR i = 1 TO n
    p = FIELD$(Paths, CHR$(10), i)
    IF p <> "" THEN ListPicker.AddCommand p, ProjectRel(p), "", "", FileIconName(p)
  NEXT
  ListPicker.Show
END SUB

' The open tabs: every open document in the searchable card under the strip's
' list button (Right, Bottom: its corner in the dock manager's pixels).
' Typing narrows it, Enter shows the tab, Escape closes.
SUB OpenTabsList(Right AS INTEGER, Bottom AS INTEGER)
  DIM d AS INTEGER, t AS STRING
  ListPicker.Clear
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      t = ProjectRel(DocFile(d))
      IF DocDirty(d) THEN t = t + "  " + CHR$(8226)
      ListPicker.AddCommand DocFile(d), t, "", "", FileIconName(DocFile(d))
    END IF
  NEXT
  ListPicker.ShowAt Dock.Left + Right, Dock.Top + Bottom + 4
END SUB

' ---- the commands ----------------------------------------------------------------------------

FUNCTION NotifyCommand(Id AS STRING) AS INTEGER
  Result = 1
  SELECT CASE Id
    CASE "view.notifications": OpenNotifications
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (a test step: notify:<what>=<value>)
'   notify:say=<text>[~kind[~actions[~key]]]   a toast
'   notify:press=<id>             the action of toast <id>, as a click would
'   notify:history                the bell clicked
'   notify:tabs                   the list button of the document tabs clicked
'   notify:report                 what Studio shows, in the Output: [notify] …
'   notify:bell                   the bell's colours against the status bar's: [bell] …
SUB NotifyStep(S AS STRING)
  DIM k AS INTEGER, what AS STRING, v AS STRING, t AS STRING, i AS INTEGER
  k = INSTR(S, "=")
  IF k > 0 THEN
    what = LEFT$(S, k - 1): v = MID$(S, k + 1)
  ELSE
    what = S
  END IF
  SELECT CASE what
    CASE "say"
      ' (Text~kind~Action1|Action2~key: a toast of any kind, for the walks)
      k = Toast(FIELD$(v, "~", 1), FIELD$(v, "~", 2), FIELD$(v, "~", 3), FIELD$(v, "~", 4))
    CASE "press"
      ' (notify:press=<number>, notify:press=last — the newest toast on
      ' screen — or notify:press=last:2 — its second action —: as a click on
      ' the action would)
      i = 1
      k = INSTR(v, ":")
      IF k > 0 THEN
        i = VAL(MID$(v, k + 1))
        v = LEFT$(v, k - 1)
      END IF
      IF v = "" OR v = "last" THEN
        IF Notify.Count > 0 THEN k = Notify.Press(VAL(Notify.Id(Notify.Count - 1)), i)
      ELSEIF v = "when" THEN
        ' (notify:press=when: waits, a minute at most, until a toast is on
        ' screen, then presses it)
        IF Notify.Count > 0 THEN
          NotifyWaits = 0
          k = Notify.Press(VAL(Notify.Id(Notify.Count - 1)), i)
        ELSEIF NotifyWaits < 60 THEN
          NotifyWaits = NotifyWaits + 1
          DoList = "notify:press=when," + DoList
        END IF
      ELSE
        k = Notify.Press(VAL(v), i)
      END IF
    CASE "retire"
      ' (notify:retire=<key>~<reason>: its actions can no longer work)
      k = Notify.Retire(FIELD$(v, "~", 1), FIELD$(v, "~", 2))
    CASE "history": OpenNotifications
    CASE "tabs": OpenTabsList Dock.DocumentLeft + Dock.DocumentWidth - 4, Dock.DocumentTop + 30
    CASE "report"
      t = "[notify] count=" + LTRIM$(STR$(Notify.Count)) + " unseen=" + LTRIM$(STR$(Notify.UnseenCount)) + " history=" + LTRIM$(STR$(Notify.HistoryCount))
      FOR i = 0 TO Notify.Count - 1
        t = t + " | " + Notify.Kind(i) + ":" + Notify.Text(i)
      NEXT
      OutputLine t
    CASE "bell"
      ' (the bell's box and pill are the bar's colour of the moment, a state's too)
      t = "[bell] state=" + RunState + " box="
      IF BellBox.Color = Status.Color THEN t = t + "bar" ELSE t = t + "other"
      t = t + " pill="
      IF BellChip.Color = Status.Color THEN t = t + "bar" ELSE t = t + "other"
      OutputLine t
  END SELECT
END SUB
