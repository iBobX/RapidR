' mcpapp.inc: AI apps' eyes and hands on the program Studio runs —
' app_layout and app_screenshot (looking), app_click, app_type, app_key,
' app_drag and app_scroll (using it), app_wait — through the program's
' own runtime (RProgramSession.App; rapidr_ui_app's eyes, hands and link):
' pictures of the program's own windows only, input into its own windows
' only, shown by a virtual pointer that glides to what it clicks; never
' the system's mouse, keyboard or screen, so nothing else on the computer
' can be seen or touched.
'
' Permission (the AI Apps pane's mode): looking is "read", allowed in
' every mode; using the program is "drive", which counts as running it —
' Read only refuses it, Ask before changes and Allow edits ask once per
' run of the program (Allow Once lets the AI app use the rest of this run,
' Allow for This Session every run), Allow everything doesn't ask.
' Paused in the debugger, the program can be looked at but not used: the
' program's runtime says where it is paused and to continue it first.
'
' Included by mcp.inc after its globals (mcp.inc lists its tools,
' decides, asks and runs them: the lines marked "mcpapp.inc" there).

DECLARE SUB McpAppTools
DECLARE SUB McpAppSetUp
DECLARE SUB McpAppRun(Tool AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE SUB McpAppArg(CallId AS INTEGER, Arg AS STRING, Key AS STRING)
DECLARE SUB McpAppReplied(Id AS INTEGER, Ok AS INTEGER, Text AS STRING, Picture AS STRING)
DECLARE FUNCTION McpAppQ(Text AS STRING) AS STRING

' (the calls waiting for the program's answer: the session's request id,
' the AI app's call, what it was)
CONST MCPAPPWAITS = 32
DIM McpAppId(MCPAPPWAITS) AS INTEGER
DIM McpAppCall(MCPAPPWAITS) AS INTEGER
DIM McpAppTool(MCPAPPWAITS) AS STRING
DIM McpAppClient(MCPAPPWAITS) AS STRING
DIM McpAppWhat(MCPAPPWAITS) AS STRING
DIM McpAppCount AS INTEGER
' (the AI app's call whose request is being sent: an answer that comes at
' once — the program isn't running — is its, before its id is known)
DIM McpAppSending AS INTEGER
' (the run in which the user allowed an AI app to use the program — its
' RunStarted: Allow Once lasts the run)
DIM McpAppRunAllowed AS DOUBLE

' A description with ` for each double quote (a BASIC string can't hold one).
FUNCTION McpAppQ(Text AS STRING) AS STRING
  Result = REPLACESUBSTR$(Text, "`", CHR$(34))
END FUNCTION

' The tools, said for small models: what each does, an example call.
SUB McpAppTools
  Mcp.AddTool "app_layout", McpAppQ("What the running program shows now, in words: each of its windows, then each control's name, type, caption or text, and place (at x,y size w×h, inside the window), and what a person would see wrong (text cut off, controls on top of each other). Call it before app_click. Example: {} for every window, or {`Window`: `Form1`}."), "[Window AS STRING]", "read"
  Mcp.AddTool "app_screenshot", McpAppQ("A picture (PNG) of the running program's windows — only the program's own, never the screen or other apps. Example: {} for all its windows side by side; {`Window`: `Form1`, `Scale`: `1x`} for one window at its own pixel size; Frame: false without its title bar; Cursor: true with the pointer an AI app moves."), "[Window AS STRING], [Scale AS STRING], [Frame AS BOOLEAN], [Cursor AS BOOLEAN]", "read"
  Mcp.AddTool "app_click", McpAppQ("Clicks in the running program as a person would (the user sees a pointer do it). Name the control by Caption (the text on it), by Name, by the Label beside it, or give a point (x, y from app_layout). Example: {`Caption`: `7`}; also {`Name`: `btnSave`}, {`Label`: `Name`}, {`Window`: `Form1`, `X`: 30, `Y`: 55}. Double: true double-clicks; Button: `right` right-clicks. Says what it clicked — or, when nothing matches, what there is."), "[Caption AS STRING], [Name AS STRING], [Label AS STRING], [Window AS STRING], [X AS INTEGER], [Y AS INTEGER], [Double AS BOOLEAN], [Button AS STRING]", "change"
  Mcp.AddTool "app_type", McpAppQ("Types text into the running program: clicks the control first, then types it key by key. Example: {`Label`: `Name`, `Text`: `Ada`} or {`Name`: `txtSearch`, `Text`: `cats`}; without a control, into what has the keyboard."), "Text AS STRING, [Label AS STRING], [Name AS STRING], [Caption AS STRING], [Window AS STRING]", "change"
  Mcp.AddTool "app_key", McpAppQ("Presses keys in the running program: Enter, Tab, Escape, Backspace, Up, Down, F5, Ctrl+S, Shift+Tab … several separated by spaces. Example: {`Keys`: `Ctrl+A Backspace`}. Name, Caption or Label gives that control the keyboard first."), "Keys AS STRING, [Name AS STRING], [Caption AS STRING], [Label AS STRING]", "change"
  Mcp.AddTool "app_drag", McpAppQ("Drags in the running program: presses on a control (or at X, Y), moves to ToX, ToY in the same window, lets go. Example: {`Name`: `Slider1`, `ToX`: 180, `ToY`: 20}."), "ToX AS INTEGER, ToY AS INTEGER, [Name AS STRING], [Caption AS STRING], [Label AS STRING], [Window AS STRING], [X AS INTEGER], [Y AS INTEGER]", "change"
  Mcp.AddTool "app_scroll", McpAppQ("Turns the mouse wheel over a control of the running program: Notches down (3 by default), negative for up. Example: {`Name`: `List1`, `Notches`: 5}."), "[Name AS STRING], [Caption AS STRING], [Label AS STRING], [Window AS STRING], [X AS INTEGER], [Y AS INTEGER], [Notches AS INTEGER]", "change"
  Mcp.AddTool "app_wait", McpAppQ("Lets the running program work on its own: Ms milliseconds (its timers, a download), or with Idle: true until it has nothing left to run (at most Ms). Example: {`Ms`: 500}."), "[Ms AS INTEGER], [Idle AS BOOLEAN]", "read"
END SUB

SUB McpAppSetUp
  Session.OnApp = McpAppReplied
END SUB

' An argument of the AI app's call, when given, into the request (McpJ).
SUB McpAppArg(CallId AS INTEGER, Arg AS STRING, Key AS STRING)
  DIM v AS STRING
  v = Mcp.Argument(Arg, CallId)
  IF v <> "" THEN McpJ.Set Key, v
END SUB

' A call allowed: the request to the program; its answer comes as
' Session's OnApp (McpAppReplied).
SUB McpAppRun(Tool AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM id AS INTEGER, n AS INTEGER, k AS INTEGER
  IF McpAppCount >= MCPAPPWAITS THEN
    Mcp.Reply CallId, "Too many calls to the program are waiting for its answer: wait for them first.", 1
    EXIT SUB
  END IF
  McpJ.Parse "{}"
  McpJ.Set "do", MID$(Tool, 5)
  McpAppArg CallId, "Window", "window"
  McpAppArg CallId, "Name", "name"
  McpAppArg CallId, "Caption", "caption"
  McpAppArg CallId, "Label", "label"
  McpAppArg CallId, "X", "x"
  McpAppArg CallId, "Y", "y"
  McpAppArg CallId, "Text", "text"
  McpAppArg CallId, "Keys", "keys"
  McpAppArg CallId, "ToX", "to_x"
  McpAppArg CallId, "ToY", "to_y"
  McpAppArg CallId, "Notches", "dy"
  McpAppArg CallId, "Ms", "ms"
  McpAppArg CallId, "Idle", "idle"
  McpAppArg CallId, "Double", "double"
  McpAppArg CallId, "Button", "button"
  McpAppArg CallId, "Scale", "scale"
  McpAppArg CallId, "Frame", "frame"
  McpAppArg CallId, "Cursor", "cursor"
  n = McpAppCount
  McpAppId(n) = 0
  McpAppCall(n) = CallId
  McpAppTool(n) = Tool
  McpAppClient(n) = Client
  McpAppWhat(n) = What
  McpAppCount = n + 1
  ' (an answer at once — the program isn't running — comes before App
  ' returns: McpAppReplied knows it by McpAppSending)
  McpAppSending = CallId
  id = Session.App(McpJ.Stringify)
  McpAppSending = 0
  FOR k = 0 TO McpAppCount - 1
    IF McpAppCall(k) = CallId AND McpAppId(k) = 0 THEN
      McpAppId(k) = id
      IF Tool <> "app_layout" AND Tool <> "app_screenshot" THEN Mcp.Progress CallId, What
    END IF
  NEXT
END SUB

' The program answered request Id (or didn't in time, or ended): the AI
' app's call answered — a picture as an image (its temporary file then
' removed) —, listed in the pane and the audit log.
SUB McpAppReplied(Id AS INTEGER, Ok AS INTEGER, Text AS STRING, Picture AS STRING)
  DIM k AS INTEGER, found AS INTEGER, callId AS INTEGER, tool AS STRING, client AS STRING, what AS STRING
  found = -1
  FOR k = 0 TO McpAppCount - 1
    IF McpAppId(k) = Id AND found < 0 THEN found = k
  NEXT
  IF found < 0 AND McpAppSending <> 0 THEN
    FOR k = 0 TO McpAppCount - 1
      IF McpAppCall(k) = McpAppSending AND McpAppId(k) = 0 AND found < 0 THEN found = k
    NEXT
  END IF
  IF found < 0 THEN EXIT SUB
  callId = McpAppCall(found): tool = McpAppTool(found): client = McpAppClient(found): what = McpAppWhat(found)
  FOR k = found TO McpAppCount - 2
    McpAppId(k) = McpAppId(k + 1)
    McpAppCall(k) = McpAppCall(k + 1)
    McpAppTool(k) = McpAppTool(k + 1)
    McpAppClient(k) = McpAppClient(k + 1)
    McpAppWhat(k) = McpAppWhat(k + 1)
  NEXT
  McpAppCount = McpAppCount - 1
  IF Ok <> 0 AND Picture <> "" THEN
    Mcp.ReplyImage callId, Picture, Text
    IF LEFT$(Picture, 5) <> "data:" THEN KILL Picture
  ELSE
    IF Ok <> 0 THEN Mcp.Reply callId, Text, 0 ELSE Mcp.Reply callId, Text, 1
  END IF
  IF Ok <> 0 THEN McpLogLine client, tool, what, "", "ok" ELSE McpLogLine client, tool, what, "", "error: " + LEFT$(Text, 80)
END SUB
