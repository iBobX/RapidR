' QDXJOYSTICK with no joystick (docs/directx-plan.md §2.4): RapidQ's
' members (Update, IsLeft / IsRight / IsUp / IsDown, Button(n)) and
' RapidR's (Connected, Name, X … V, Buttons, POV) read "nothing there" and
' at rest — never an error (RapidQ's own raised EStringListError).
$APPTYPE CONSOLE
DIM J AS QDXJOYSTICK
J.Update
PRINT J.IsLeft; J.IsRight; J.IsUp; J.IsDown; J.Button(1); J.Button(32)
PRINT J.Connected; "["; J.Name; "]"; J.Index
PRINT J.X; " "; J.Y; " "; J.Z; " "; J.R; " "; J.U; " "; J.V; " "; J.Buttons; " "; J.POV
J.Index = 3
J.Update
PRINT J.Index; J.Connected; J.Button(0); J.Button(33)
