' QDOCKFORM for RapidR: a panel that sits docked at an edge of its form
' (or at an alternative place) and can be torn off into a window of its
' own and docked back — by the program (Dock, Close) or by dragging its
' title or grip. RapidQ programs know it from RAPIDQ2.INC (the manual's
' Appendix A); this is RapidR's own code for the same members, styles and
' behaviour, built on QPANEL, QCANVAS and QFORM with no Windows calls
' (docs/desktop-host-plan.md, "RapidQ's remaining UI objects"). A program
' that names QDOCKFORM gets it — RAPIDQ2.INC's own (`$IFNDEF __QDF_INC`)
' then stays out.
$IFNDEF __QDF_INC
$DEFINE __QDF_INC
DECLARE SUB RapidR_DockForm_OnDock (Docked AS INTEGER, Alt AS INTEGER)
DECLARE SUB RapidR_DockForm_OnPaint
DECLARE SUB RapidR_DockForm_OnClose
' (Windows' classic colours: shadow, face, highlight)
CONST RAPIDR_DF_SHADOW = &H808080
CONST RAPIDR_DF_FACE = &HC0C0C0
CONST RAPIDR_DF_LIGHT = &HFFFFFF

TYPE QDOCKFORM EXTENDS QPANEL
  PRIVATE:
    MouseDown AS INTEGER
    PrevX AS INTEGER
    PrevY AS INTEGER
    DockX AS INTEGER
    DockY AS INTEGER
    AltX AS INTEGER
    AltY AS INTEGER
    PressX AS INTEGER
    PressY AS INTEGER
    ByProg AS INTEGER
    ClosePressed AS INTEGER
  PUBLIC:
    TitleColor AS LONG
    ST1_TitleBackground AS LONG
    DragCursor AS INTEGER
    AltAlign AS INTEGER
    Sizeable AS INTEGER
    Style AS INTEGER
    UndockedHeight AS INTEGER
    UndockedWidth AS INTEGER
    Caption AS STRING PROPERTY SET RapidR_SetCaption
    Docked AS INTEGER PROPERTY SET RapidR_ReadOnly
    UndockedForm AS QFORM
    AltPanel AS QPANEL
    Panel AS QPANEL
    Canvas AS QCANVAS
    Client AS QPANEL
    ' (made last: over the canvas, as the library's)
    CloseButton AS QCANVAS
    DockStyle AS INTEGER PROPERTY SET RapidR_SetDockStyle
    UnDockStyle AS INTEGER PROPERTY SET RapidR_SetUnDockStyle
    Locked AS INTEGER PROPERTY SET RapidR_SetLock
    CanClose AS INTEGER PROPERTY SET RapidR_SetClose
    Closed AS INTEGER PROPERTY SET RapidR_ReadOnly
    AltDock AS INTEGER PROPERTY SET RapidR_ReadOnly
    OnDock AS EVENT (RapidR_DockForm_OnDock)
    OnPaint AS EVENT (RapidR_DockForm_OnPaint)
    OnClose AS EVENT (RapidR_DockForm_OnClose)

  PROPERTY SET RapidR_ReadOnly (Dummy AS INTEGER)
    ' (read-only)
  END PROPERTY

  PROPERTY SET RapidR_SetCaption (NewCaption AS STRING)
    QDOCKFORM.Caption = NewCaption
    QDOCKFORM.UndockedForm.Caption = NewCaption
    QDOCKFORM.AccessibleName = NewCaption
    QDOCKFORM.Canvas.Repaint
  END PROPERTY

  PROPERTY SET RapidR_SetDockStyle (NewStyle AS INTEGER)
    QDOCKFORM.DockStyle = NewStyle
    QDOCKFORM.Canvas.Repaint
  END PROPERTY

  PROPERTY SET RapidR_SetUnDockStyle (NewStyle AS INTEGER)
    ' (6 isn't an undocked style: the library draws 1 instead)
    IF NewStyle = 6 THEN NewStyle = 1
    QDOCKFORM.UnDockStyle = NewStyle
    QDOCKFORM.Canvas.Repaint
  END PROPERTY

  PROPERTY SET RapidR_SetLock (NewLock AS INTEGER)
    IF NewLock > 0 THEN QDOCKFORM.Locked = 1 ELSE QDOCKFORM.Locked = 0
    QDOCKFORM.Canvas.Repaint
  END PROPERTY

  PROPERTY SET RapidR_SetClose (NewClose AS INTEGER)
    IF NewClose > 0 THEN QDOCKFORM.CanClose = 1 ELSE QDOCKFORM.CanClose = 0
    QDOCKFORM.Canvas.Repaint
  END PROPERTY

  ' The style shown now.
  FUNCTION RapidR_Shown AS INTEGER
    IF QDOCKFORM.Docked THEN Result = QDOCKFORM.DockStyle ELSE Result = QDOCKFORM.UnDockStyle
  END FUNCTION

  ' Whether the style drags by its title (else by a grip at its left).
  FUNCTION RapidR_Titled AS INTEGER
    DIM s AS INTEGER
    s = QDOCKFORM.RapidR_Shown
    Result = (s < 3 OR s > 9)
  END FUNCTION

  ' Puts the panel into its floating window, that window's corner at
  ' (WX, WY) and its size UndockedWidth x UndockedHeight.
  SUB RapidR_Float (WX AS DOUBLE, WY AS DOUBLE)
    QDOCKFORM.Docked = 0
    QDOCKFORM.Visible = 0
    QDOCKFORM.AltPanel.Visible = 0
    QDOCKFORM.UndockedForm.Left = WX
    QDOCKFORM.UndockedForm.Top = WY
    QDOCKFORM.UndockedForm.Width = QDOCKFORM.UndockedWidth
    QDOCKFORM.UndockedForm.Height = QDOCKFORM.UndockedHeight
    QDOCKFORM.Panel.Parent = QDOCKFORM.UndockedForm
    QDOCKFORM.UndockedForm.Show
  END SUB

  ' Puts the panel back into a dock — Spot 2 the alternative place, else
  ' its own — and lets the floating window go. Tidy also hides the place
  ' it doesn't use (a dock asked for by the program can come from the
  ' other place; a dragged window always comes from floating).
  SUB RapidR_Settle (Spot AS INTEGER, Tidy AS INTEGER)
    QDOCKFORM.Docked = 1
    IF Spot = 2 THEN
      QDOCKFORM.AltPanel.Visible = 1
      IF Tidy THEN QDOCKFORM.Visible = 0
      QDOCKFORM.Panel.Parent = QDOCKFORM.AltPanel
    ELSE
      IF Tidy THEN QDOCKFORM.AltPanel.Visible = 0
      QDOCKFORM.Visible = 1
      QDOCKFORM.Panel.Parent = QDOCKFORM
    END IF
    ' (the window's OnClose below knows this close is no user's)
    QDOCKFORM.ByProg = 1
    QDOCKFORM.UndockedForm.Close
  END SUB

  ' Where = 1 docks it at home; 2 at the alternative place when Style is
  ' set (home otherwise); any other value floats it in its own window,
  ' centred on the screen. A closed form comes back this way, AltDock then
  ' left as it was; an open one's AltDock tells where it went.
  SUB Dock (Where AS INTEGER)
    DIM spot AS INTEGER
    spot = 0
    IF Where = 1 THEN spot = 1
    IF Where = 2 THEN
      IF QDOCKFORM.Style > 0 THEN spot = 2 ELSE spot = 1
    END IF
    IF QDOCKFORM.Closed THEN
      QDOCKFORM.Closed = 0
    ELSEIF spot = 2 THEN
      QDOCKFORM.AltDock = 1
    ELSE
      QDOCKFORM.AltDock = 0
    END IF
    IF spot = 0 THEN
      QDOCKFORM.RapidR_Float((Screen.Width - QDOCKFORM.UndockedWidth) / 2, (Screen.Height - QDOCKFORM.UndockedHeight) / 2)
    ELSE
      QDOCKFORM.RapidR_Settle(spot, 1)
    END IF
    QDOCKFORM.Canvas.Repaint
  END SUB

  ' Hides the form (Dock shows it again) and tells the program (OnClose).
  ' Floating or at the alternative place, it goes home first.
  SUB Close
    IF QDOCKFORM.Docked = 0 OR QDOCKFORM.AltDock = 1 THEN
      ' (Dock seeing Closed set leaves AltDock alone, and clears Closed)
      QDOCKFORM.Closed = 1
      QDOCKFORM.Dock(1)
    END IF
    QDOCKFORM.Closed = 1
    QDOCKFORM.Visible = 0
    IF QDOCKFORM.OnClose THEN CALLFUNC QDOCKFORM.OnClose
  END SUB

  ' The close button: Windows' caption close box, 16 x 14.
  SUB RapidR_DrawClose (Flat AS INTEGER)
    QDOCKFORM.CloseButton.FillRect(0, 0, 16, 14, RAPIDR_DF_FACE)
    IF QDOCKFORM.ClosePressed THEN
      QDOCKFORM.CloseButton.Line(0, 0, 15, 0, RAPIDR_DF_SHADOW)
      QDOCKFORM.CloseButton.Line(0, 0, 0, 13, RAPIDR_DF_SHADOW)
      QDOCKFORM.CloseButton.Line(15, 0, 15, 13, RAPIDR_DF_LIGHT)
      QDOCKFORM.CloseButton.Line(0, 13, 16, 13, RAPIDR_DF_LIGHT)
    ELSEIF Flat = 0 THEN
      QDOCKFORM.CloseButton.Line(0, 0, 15, 0, RAPIDR_DF_LIGHT)
      QDOCKFORM.CloseButton.Line(0, 0, 0, 13, RAPIDR_DF_LIGHT)
      QDOCKFORM.CloseButton.Line(15, 0, 15, 13, 0)
      QDOCKFORM.CloseButton.Line(0, 13, 16, 13, 0)
      QDOCKFORM.CloseButton.Line(14, 1, 14, 12, RAPIDR_DF_SHADOW)
      QDOCKFORM.CloseButton.Line(1, 12, 15, 12, RAPIDR_DF_SHADOW)
    END IF
    ' (the cross)
    QDOCKFORM.CloseButton.Line(4, 3, 11, 10, 0)
    QDOCKFORM.CloseButton.Line(5, 3, 12, 10, 0)
    QDOCKFORM.CloseButton.Line(11, 3, 4, 10, 0)
    QDOCKFORM.CloseButton.Line(12, 3, 5, 10, 0)
  END SUB

  ' A grip dot: a 2 x 2 shadow square with a highlight under its corner.
  SUB RapidR_Dot (X AS INTEGER, Y AS INTEGER)
    QDOCKFORM.Canvas.FillRect(X + 1, Y + 1, X + 3, Y + 3, RAPIDR_DF_LIGHT)
    QDOCKFORM.Canvas.FillRect(X, Y, X + 2, Y + 2, RAPIDR_DF_SHADOW)
  END SUB

  ' A column of three dots, 4 pixels apart, from (X, Y) down.
  SUB RapidR_Dots (X AS INTEGER, Y AS INTEGER)
    DIM dy AS INTEGER
    FOR dy = 0 TO 8 STEP 4
      QDOCKFORM.RapidR_Dot(X, Y + dy)
    NEXT dy
  END SUB

  ' A 3-D edge round (0, 0)-(W, H): Lit on the top and left sides, Dark on
  ' the bottom and right; Shade adds the inner shadow of a raised window.
  SUB RapidR_Edges (W AS INTEGER, H AS INTEGER, Lit AS INTEGER, Dark AS INTEGER, Shade AS INTEGER)
    QDOCKFORM.Canvas.Line(0, 0, W, 0, Lit)
    QDOCKFORM.Canvas.Line(0, 0, 0, H, Lit)
    QDOCKFORM.Canvas.Line(0, H - 1, W, H - 1, Dark)
    QDOCKFORM.Canvas.Line(W - 1, 0, W - 1, H, Dark)
    IF Shade THEN
      QDOCKFORM.Canvas.Line(1, H - 2, W - 1, H - 2, RAPIDR_DF_SHADOW)
      QDOCKFORM.Canvas.Line(W - 2, 1, W - 2, H - 1, RAPIDR_DF_SHADOW)
    END IF
  END SUB

  ' A title bar as Windows draws an active (or inactive) caption.
  SUB RapidR_CaptionBar (X1 AS INTEGER, Y1 AS INTEGER, X2 AS INTEGER, Y2 AS INTEGER, Active AS INTEGER)
    DIM th AS INTEGER
    IF Active THEN
      QDOCKFORM.Canvas.FillRect(X1, Y1, X2, Y2, &H800000)
    ELSE
      QDOCKFORM.Canvas.FillRect(X1, Y1, X2, Y2, RAPIDR_DF_SHADOW)
    END IF
    th = QDOCKFORM.Canvas.TextHeight(QDOCKFORM.Caption)
    QDOCKFORM.Canvas.TextOut(X1 + 3, Y1 + (Y2 - Y1 - th) / 2, QDOCKFORM.Caption, IIF(Active, RAPIDR_DF_LIGHT, RAPIDR_DF_FACE), -1)
  END SUB

  ' Where the Client sits for the style shown: below a title, or right of
  ' a grip (none when docked and locked).
  SUB RapidR_PlaceClient (S AS INTEGER)
    DIM pw AS INTEGER, ph AS INTEGER, gap AS INTEGER
    pw = QDOCKFORM.Panel.Width
    ph = QDOCKFORM.Panel.Height
    IF QDOCKFORM.RapidR_Titled THEN
      QDOCKFORM.Client.Left = 5
      QDOCKFORM.Client.Top = 25
      QDOCKFORM.Client.Width = pw - 10
      QDOCKFORM.Client.Height = ph - 30
      EXIT SUB
    END IF
    QDOCKFORM.Client.Top = 3
    QDOCKFORM.Client.Height = ph - 6
    IF QDOCKFORM.Docked = 1 AND QDOCKFORM.Locked THEN
      QDOCKFORM.Client.Left = 3
      QDOCKFORM.Client.Width = pw - 4
      EXIT SUB
    END IF
    ' (the grip's width, and what the right side keeps)
    IF S = 5 AND QDOCKFORM.Docked = 0 THEN gap = 14 ELSE gap = 15
    QDOCKFORM.Client.Left = gap
    IF S = 5 THEN
      QDOCKFORM.Client.Width = pw - 17
    ELSEIF QDOCKFORM.Docked THEN
      QDOCKFORM.Client.Width = pw - 16
    ELSE
      QDOCKFORM.Client.Width = pw - 20
    END IF
  END SUB

  ' The border a floating panel draws round itself, by style.
  SUB RapidR_FloatFrame (S AS INTEGER, W AS INTEGER, H AS INTEGER)
    DIM rings AS INTEGER, r AS INTEGER
    IF S = 3 OR S = 5 OR S = 10 THEN
      QDOCKFORM.RapidR_Edges(W, H, RAPIDR_DF_LIGHT, 0, 1)
      EXIT SUB
    END IF
    rings = 0
    IF S = 0 OR S = 2 THEN rings = 1
    IF S = 1 OR S = 4 THEN rings = 2
    FOR r = 0 TO rings - 1
      QDOCKFORM.Canvas.Rectangle(r, r, W - r, H - r, RAPIDR_DF_SHADOW)
    NEXT r
  END SUB

  ' A titled style's head: the inner frame, then its title in the style's
  ' way (a band, a dotted rule, a caption bar, or three ruled lines).
  SUB RapidR_Head (S AS INTEGER, W AS INTEGER, H AS INTEGER, Active AS INTEGER)
    DIM tw AS INTEGER, ty AS INTEGER, x AS INTEGER, y AS INTEGER, mid AS INTEGER
    tw = QDOCKFORM.Canvas.TextWidth(QDOCKFORM.Caption)
    ty = 5 + (18 - QDOCKFORM.Canvas.TextHeight(QDOCKFORM.Caption)) / 2
    IF (S <> 2 AND S <> 10) OR QDOCKFORM.Docked THEN QDOCKFORM.Canvas.Rectangle(2, 2, W - 2, H - 2, RAPIDR_DF_SHADOW)
    IF S = 10 THEN
      QDOCKFORM.RapidR_CaptionBar(4, 4, W - 4 - (QDOCKFORM.Docked = 0), 24, Active)
    ELSEIF S = 1 OR S = 2 THEN
      IF S = 1 AND Active THEN QDOCKFORM.Canvas.FillRect(5, 5, W - 5, 23, QDOCKFORM.ST1_TitleBackground)
      QDOCKFORM.Canvas.TextOut(7, ty, QDOCKFORM.Caption, QDOCKFORM.TitleColor, -1)
      IF S = 2 AND Active THEN
        FOR x = tw + 11 TO W - 7 STEP 4
          QDOCKFORM.RapidR_Dots(x, 10)
        NEXT x
      END IF
    ELSE
      ' (0, and the styles past 10: three rules behind the centred caption)
      FOR y = 12 TO 16 STEP 2
        QDOCKFORM.Canvas.Line(3, y, W - 4, y, RAPIDR_DF_SHADOW)
      NEXT y
      mid = (W - tw) / 2
      QDOCKFORM.Canvas.FillRect(mid - 5, 12, mid + tw + 5, 17, RAPIDR_DF_FACE)
      QDOCKFORM.Canvas.TextOut(mid, ty, QDOCKFORM.Caption, QDOCKFORM.TitleColor, -1)
    END IF
  END SUB

  ' A grip style's look: its edges when docked, then the grip itself down
  ' the left side (three grooves, dots, a raised bar, or hatching).
  SUB RapidR_Grip (S AS INTEGER, W AS INTEGER, H AS INTEGER, Active AS INTEGER)
    DIM x AS INTEGER, y AS INTEGER
    IF QDOCKFORM.Docked THEN
      IF S = 5 THEN
        QDOCKFORM.RapidR_Edges(W, H, RAPIDR_DF_LIGHT, RAPIDR_DF_SHADOW, 0)
      ELSE
        QDOCKFORM.Canvas.Line(0, 0, W, 0, RAPIDR_DF_SHADOW)
        QDOCKFORM.Canvas.Line(0, H - 1, W, H - 1, RAPIDR_DF_LIGHT)
      END IF
    END IF
    IF Active = 0 THEN EXIT SUB
    IF S = 3 THEN
      FOR x = 5 TO 9 STEP 2
        QDOCKFORM.Canvas.Line(x, 3, x, H - 3, RAPIDR_DF_SHADOW)
      NEXT x
    ELSEIF S = 4 THEN
      FOR y = 4 TO H - 4 STEP 12
        QDOCKFORM.RapidR_Dots(5, y)
      NEXT y
    ELSEIF S = 5 THEN
      ' (a bar two pixels wide, lit at the left and top, shaded at the right
      ' and bottom)
      QDOCKFORM.Canvas.Line(5, 3, 7, 3, RAPIDR_DF_LIGHT)
      QDOCKFORM.Canvas.Line(5, 4, 5, H - 4, RAPIDR_DF_LIGHT)
      QDOCKFORM.Canvas.Line(7, 3, 7, H - 4, RAPIDR_DF_SHADOW)
      QDOCKFORM.Canvas.Pset(6, H - 4, RAPIDR_DF_SHADOW)
    ELSE
      FOR y = 4 TO H - 4 STEP 2
        QDOCKFORM.Canvas.Line(5, y, 8, y, RAPIDR_DF_SHADOW)
      NEXT y
    END IF
  END SUB

  ' Paints the panel for the style shown: the Client's place, the floating
  ' frame, the head or the grip, the close button; then the program's
  ' OnPaint.
  EVENT Canvas.OnPaint
    DIM w AS INTEGER, h AS INTEGER, s AS INTEGER, active AS INTEGER
    w = QDOCKFORM.Canvas.Width
    h = QDOCKFORM.Canvas.Height
    s = QDOCKFORM.RapidR_Shown
    ' (a docked, locked panel shows no handle)
    active = (QDOCKFORM.Locked = 0 OR QDOCKFORM.Docked = 0)
    QDOCKFORM.Canvas.FillRect(0, 0, w, h, RAPIDR_DF_FACE)
    QDOCKFORM.Canvas.Font.AddStyles(0)
    QDOCKFORM.RapidR_PlaceClient(s)
    IF QDOCKFORM.Docked = 0 THEN QDOCKFORM.RapidR_FloatFrame(s, w, h)
    IF QDOCKFORM.RapidR_Titled THEN
      QDOCKFORM.RapidR_Head(s, w, h, active)
    ELSE
      QDOCKFORM.RapidR_Grip(s, w, h, active)
    END IF
    ' (the close button: the titled styles but the past-10 ones)
    IF QDOCKFORM.CanClose AND QDOCKFORM.RapidR_Titled AND s <= 10 THEN
      QDOCKFORM.CloseButton.Left = w - 22
      QDOCKFORM.CloseButton.Top = 7
      QDOCKFORM.CloseButton.Width = 16
      QDOCKFORM.CloseButton.Height = 14
      QDOCKFORM.CloseButton.Visible = 1
      IF s <> 1 AND s <> 10 THEN QDOCKFORM.Canvas.FillRect(w - 26, 6, w - 3, 22, RAPIDR_DF_FACE)
      QDOCKFORM.RapidR_DrawClose(s <> 10)
    ELSE
      QDOCKFORM.CloseButton.Visible = 0
    END IF
    IF QDOCKFORM.OnPaint <> 0 THEN CALLFUNC QDOCKFORM.OnPaint
  END EVENT

  ' (the close button: pressed down while the mouse holds it; a release
  ' still on it closes the form)
  EVENT CloseButton.OnMouseDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    QDOCKFORM.ClosePressed = 1
    QDOCKFORM.RapidR_DrawClose(QDOCKFORM.RapidR_Shown <> 10)
  END EVENT

  EVENT CloseButton.OnMouseUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    QDOCKFORM.ClosePressed = 0
    QDOCKFORM.RapidR_DrawClose(QDOCKFORM.RapidR_Shown <> 10)
    IF X < 0 OR Y < 0 OR X >= 16 OR Y >= 14 THEN EXIT EVENT
    QDOCKFORM.Close
  END EVENT

  ' Whether (X, Y) on the canvas is where a drag starts: the title bar of
  ' a titled style, else the grip strip down the left side (above Bottom).
  FUNCTION RapidR_OnHandle (X AS INTEGER, Y AS INTEGER, Bottom AS INTEGER) AS INTEGER
    Result = 0
    IF X <= 4 OR Y <= 3 THEN EXIT FUNCTION
    IF QDOCKFORM.RapidR_Titled THEN
      IF X < QDOCKFORM.Canvas.Width - 4 AND Y < 24 THEN Result = 1
    ELSE
      IF X < 15 AND Y < Bottom THEN Result = 1
    END IF
  END FUNCTION

  ' Whether both distances are less than Reach.
  FUNCTION RapidR_Near (DX AS INTEGER, DY AS INTEGER, Reach AS INTEGER) AS INTEGER
    Result = 0
    IF ABS(DX) < Reach AND ABS(DY) < Reach THEN Result = 1
  END FUNCTION

  ' A left press on the title (or grip) of a form that isn't Locked starts
  ' a drag. Docked, it also notes on the screen where its window's corner
  ' would stand at home (DockX, DockY) and at the alternative place
  ' (AltX, AltY), for the moves to compare with.
  EVENT Canvas.OnMouseDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    DIM baseX AS INTEGER, baseY AS INTEGER
    IF Button <> 0 OR QDOCKFORM.Locked THEN EXIT EVENT
    IF QDOCKFORM.RapidR_OnHandle(X, Y, QDOCKFORM.Canvas.Height - 1) = 0 THEN EXIT EVENT
    QDOCKFORM.MouseDown = 1
    QDOCKFORM.PrevX = X
    QDOCKFORM.PrevY = Y
    QDOCKFORM.PressX = Screen.MouseX
    QDOCKFORM.PressY = Screen.MouseY
    IF QDOCKFORM.Docked = 0 THEN EXIT EVENT
    QDOCKFORM.RapidR_MeasureAlt
    ' (the form's client corner on the screen: the mouse, less the point
    ' in the canvas, less where the panel that holds it now sits)
    baseX = Screen.MouseX - X
    baseY = Screen.MouseY - Y
    IF QDOCKFORM.AltDock THEN
      baseX = baseX - QDOCKFORM.AltPanel.Left
      baseY = baseY - QDOCKFORM.AltPanel.Top
    ELSE
      baseX = baseX - QDOCKFORM.Left
      baseY = baseY - QDOCKFORM.Top
    END IF
    QDOCKFORM.DockX = baseX + QDOCKFORM.Left
    QDOCKFORM.DockY = baseY + QDOCKFORM.Top
    QDOCKFORM.AltX = baseX + QDOCKFORM.AltPanel.Left
    QDOCKFORM.AltY = baseY + QDOCKFORM.AltPanel.Top
  END EVENT

  EVENT Canvas.OnMouseUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    IF Button = 0 THEN QDOCKFORM.MouseDown = 0
  END EVENT

  ' With no drag on, the cursor shows where one can start (DragCursor).
  ' During a drag the window's corner follows the mouse: a docked form
  ' pulled more than 30 pixels from every place it may dock (its home; its
  ' alternative with Style 1) floats there; a floating one brought within
  ' 29 pixels of its home — or, with Style 1, its alternative — docks
  ' there. Either ends the drag and tells the program (OnDock).
  EVENT Canvas.OnMouseMove (X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    DIM wx AS INTEGER, wy AS INTEGER, shape AS INTEGER
    IF QDOCKFORM.MouseDown = 0 THEN
      shape = 0
      IF QDOCKFORM.Locked = 0 THEN
        IF QDOCKFORM.RapidR_OnHandle(X, Y, 32767) THEN shape = QDOCKFORM.DragCursor
      END IF
      QDOCKFORM.Canvas.Cursor = shape
      EXIT EVENT
    END IF
    wx = Screen.MouseX - QDOCKFORM.PrevX
    wy = Screen.MouseY - QDOCKFORM.PrevY
    IF QDOCKFORM.Docked = 0 THEN
      QDOCKFORM.UndockedForm.Left = wx
      QDOCKFORM.UndockedForm.Top = wy
      IF QDOCKFORM.RapidR_Near(wx - QDOCKFORM.DockX, wy - QDOCKFORM.DockY, 29) THEN
        QDOCKFORM.MouseDown = 0
        QDOCKFORM.AltDock = 0
        QDOCKFORM.RapidR_Settle(1, 0)
        QDOCKFORM.Canvas.Repaint
        IF QDOCKFORM.OnDock THEN CALLFUNC(QDOCKFORM.OnDock, 1, 0)
      ELSEIF QDOCKFORM.Style = 1 THEN
        IF QDOCKFORM.RapidR_Near(wx - QDOCKFORM.AltX, wy - QDOCKFORM.AltY, 29) THEN
          QDOCKFORM.MouseDown = 0
          QDOCKFORM.AltDock = 1
          QDOCKFORM.RapidR_Settle(2, 0)
          QDOCKFORM.Canvas.Repaint
          IF QDOCKFORM.OnDock THEN CALLFUNC(QDOCKFORM.OnDock, 1, 1)
        END IF
      END IF
      EXIT EVENT
    END IF
    ' (docked: still close to a place it may dock at, it stays)
    IF QDOCKFORM.RapidR_Near(wx - QDOCKFORM.DockX, wy - QDOCKFORM.DockY, 31) THEN EXIT EVENT
    IF QDOCKFORM.Style = 1 THEN
      IF QDOCKFORM.RapidR_Near(wx - QDOCKFORM.AltX, wy - QDOCKFORM.AltY, 31) THEN EXIT EVENT
    END IF
    QDOCKFORM.MouseDown = 0
    QDOCKFORM.AltDock = 0
    QDOCKFORM.RapidR_Float(wx, wy)
    QDOCKFORM.Canvas.Repaint
    IF QDOCKFORM.OnDock THEN CALLFUNC(QDOCKFORM.OnDock, 0, 0)
  END EVENT

  ' The floating window closing: a close by Dock or a drag passes; the
  ' user's (its close box — a borderless window has none — or the
  ' system's) is refused unless CanClose, else it closes the form: OnClose
  ' the first time, then Closed.
  EVENT UndockedForm.OnClose (Action AS INTEGER)
    IF QDOCKFORM.ByProg = 0 THEN
      IF QDOCKFORM.CanClose THEN
        IF QDOCKFORM.Closed = 0 THEN
          IF QDOCKFORM.OnClose THEN CALLFUNC QDOCKFORM.OnClose
        END IF
        QDOCKFORM.Closed = 1
      ELSE
        Action = 0
      END IF
    END IF
    QDOCKFORM.ByProg = 0
  END EVENT

  ' Lets the form lay out the hidden alternative panel, to read where it
  ' would sit: shown for an instant with no depth (its height for a top or
  ' bottom panel, else its width), then hidden with its depth back.
  SUB RapidR_MeasureAlt
    DIM upright AS INTEGER, depth AS INTEGER
    IF QDOCKFORM.AltPanel.Visible THEN EXIT SUB
    upright = 1
    IF QDOCKFORM.AltPanel.Align = 1 OR QDOCKFORM.AltPanel.Align = 2 THEN upright = 0
    IF upright THEN
      depth = QDOCKFORM.AltPanel.Width
      QDOCKFORM.AltPanel.Width = 0
    ELSE
      depth = QDOCKFORM.AltPanel.Height
      QDOCKFORM.AltPanel.Height = 0
    END IF
    QDOCKFORM.AltPanel.Visible = 1
    QDOCKFORM.AltPanel.Visible = 0
    IF upright THEN QDOCKFORM.AltPanel.Width = depth ELSE QDOCKFORM.AltPanel.Height = depth
  END SUB

  CONSTRUCTOR
    ' (a new one: docked at the left, 170 x 400 when it floats, dragged
    ' with the four-arrow cursor, a grey style 1 title)
    Docked = 1
    UndockedWidth = 170
    UndockedHeight = 400
    DragCursor = -21
    TitleColor = 0
    ST1_TitleBackground = RAPIDR_DF_SHADOW
    BevelOuter = 0
    Align = 3
    ' (inside: Panel, which moves between the docks and the window, holds
    ' the canvas the look is drawn on, the program's Client over it, and
    ' the close button, hidden until a style shows it)
    Panel.Parent = QDOCKFORM
    Panel.BevelOuter = 0
    Panel.Align = 5
    Canvas.Parent = QDOCKFORM.Panel
    Canvas.Align = 5
    Client.Parent = QDOCKFORM.Panel
    Client.BevelOuter = 2
    Client.Left = 5
    Client.Top = 25
    Client.Width = QDOCKFORM.Panel.Width - 10
    Client.Height = QDOCKFORM.Panel.Height - 30
    CloseButton.Parent = QDOCKFORM.Panel
    CloseButton.Visible = 0
    CloseButton.Width = 16
    CloseButton.Height = 14
    ' (the alternative place, hidden, at the right until the program
    ' parents it elsewhere; the floating window without a border)
    AltPanel.Parent = QDOCKFORM
    AltPanel.Align = 4
    AltPanel.Visible = 0
    AltPanel.BevelOuter = 0
    UndockedForm.BorderStyle = 0
  END CONSTRUCTOR
END TYPE
$ENDIF
