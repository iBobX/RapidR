' mcpapp.inc: AI apps' eyes and hands on the program Studio runs —
' app_layout and app_screenshot (looking), app_click, app_type, app_key,
' app_drag and app_scroll (using it), app_wait — through the program's
' own runtime (RProgramSession.App; rapidr_ui_app's eyes, hands and link):
' pictures of the program's own windows only, input into its own windows
' only, shown by a virtual pointer that glides to what it clicks; never
' the system's mouse, keyboard or screen, so nothing else on the computer
' can be seen or touched.
'
' Permission (the AI Apps pane's mode): looking at the program (its
' layout, a picture: it can show private data) and using it are "drive",
' which counts as running it — Read only refuses it, Ask before changes
' and Allow edits ask once per run of the program (Allow Once covers
' pictures, clicks and typing until this run ends; Allow for This Session
' every run), Allow everything doesn't ask. app_wait only lets time pass:
' "read".
' Paused in the debugger, the program can be looked at but not used: the
' program's runtime says where it is paused and to continue it first.
'
' The tools themselves — names, descriptions, schemas — are RapidR
' Studio's one tool set (crates/rapidr-studio/src/toolset.rs), the same
' RAI uses; the request to the program is built once, for both
' (McpTools.AppRequest). Included by mcp.inc after its globals (mcp.inc
' lists the tools — once the user confirmed the first-time question's
' version 2, which names them —, decides, asks and runs them: the lines
' marked "mcpapp.inc" there).

DECLARE SUB McpAppSetUp
DECLARE SUB McpAppRun(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
DECLARE SUB McpAppReplied(Id AS INTEGER, Ok AS INTEGER, Text AS STRING, Picture AS STRING)

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

SUB McpAppSetUp
  Session.OnApp = McpAppReplied
END SUB

' A call allowed: the request to the program; its answer comes as
' Session's OnApp (McpAppReplied).
SUB McpAppRun(Tool AS STRING, Arguments AS STRING, CallId AS INTEGER, Client AS STRING, What AS STRING)
  DIM id AS INTEGER, n AS INTEGER, k AS INTEGER, req AS STRING
  IF McpAppCount >= MCPAPPWAITS THEN
    Mcp.Reply CallId, "Too many calls to the program are waiting for its answer: wait for them first.", 1
    EXIT SUB
  END IF
  ' (the program's request, the same RAI's calls make)
  req = McpTools.AppRequest(Tool, Arguments)
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
  id = Session.App(req)
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
' removed) —, listed in the pane and the audit log: the action and "ok"
' or "error" only, never the program's sentence (it quotes what the
' program shows), the picture or what was typed.
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
  IF Ok <> 0 THEN McpLogLine client, tool, what, "", "ok" ELSE McpLogLine client, tool, what, "", "error"
END SUB
