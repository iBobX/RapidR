' focus.rr: OnEnter and OnExit (RapidR's, as Delphi's) on an edit and two
' buttons, each a place for a breakpoint (tests/studio_flows.mjs
' debug-events-focus-*, tools/macos/debug_walk.py's focus path): Tab and
' clicks move the focus; each handler adds to the trail what happened.
$THEME rapidr
DECLARE SUB NameEnter
DECLARE SUB NameExit
DECLARE SUB GreetEnter
DECLARE SUB GreetExit
DECLARE SUB Greet

DIM trail AS STRING, enters AS INTEGER, exits AS INTEGER

CREATE Form AS RForm
    Caption = "Focus"
    Width = 340: Height = 190
    CREATE NameEdit AS REdit
        Left = 16: Top = 16: Width = 200: Text = "Ada"
        OnEnter = NameEnter
        OnExit = NameExit
    END CREATE
    CREATE GreetButton AS RButton
        Caption = "Greet": Left = 16: Top = 52: Width = 90
        OnEnter = GreetEnter
        OnExit = GreetExit
        OnClick = Greet
    END CREATE
    CREATE OtherButton AS RButton
        Caption = "Other": Left = 120: Top = 52: Width = 90
    END CREATE
    CREATE Steps AS RLabel
        Left = 16: Top = 96: Width = 300
    END CREATE
END CREATE

SUB NameEnter
    enters = enters + 1: trail = trail + " +name"
END SUB

SUB NameExit
    exits = exits + 1: trail = trail + " -name"
END SUB

SUB GreetEnter
    enters = enters + 1: trail = trail + " +greet"
END SUB

SUB GreetExit
    exits = exits + 1: trail = trail + " -greet"
END SUB

SUB Greet
    trail = trail + " click"
    Steps.Caption = trail
END SUB

Form.ShowModal
PRINT "trail:"; trail
PRINT "enters"; enters; " exits"; exits
