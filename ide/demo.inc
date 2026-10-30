' RapidR Studio's side of a recorded demo (docs/video-pipeline.md).
'
' A demo script's `do` steps are Studio's own commands — the ones --do
' runs (file.saveAll, open:main.rr, run.stop…). The demo player
' (rapidr_ui_app::demo) writes each step's commands to
' <RAPIDR_DEMO_OUT>/inbox/do-<n>.txt; Studio, started with
' `--do demo:poll`, looks for the next one at each macro tick, runs its
' commands one a tick (quietly: no "> command" line in the Output, the
' video shows Studio as the user has it) and writes done-<n> when they
' have run. A test hook: RAPIDR_DEMO_OUT is read only in a development
' build or with the signed test-mode token (docs/security/test-mode.md).

DIM DemoSeq AS INTEGER
DIM DemoQueue AS STRING
DIM DemoMarker AS RFileStream

' demo:poll — the inbox looked at (and asked again at the next tick).
FUNCTION DemoStep(Stp AS STRING) AS INTEGER
  DIM folder AS STRING, f AS STRING, id AS STRING, k AS INTEGER
  IF Stp <> "demo:poll" THEN EXIT FUNCTION
  Result = 1
  folder = ENVIRON$("RAPIDR_DEMO_OUT")
  ' (asked again at the next tick, after what is queued: Studio's own --do
  ' steps go first)
  DoList = DoList + ",demo:poll"
  IF folder = "" THEN EXIT FUNCTION
  IF DemoQueue = "" THEN
    f = folder + "/inbox/do-" + LTRIM$(STR$(DemoSeq + 1)) + ".txt"
    IF FILEEXISTS(f) = 0 THEN EXIT FUNCTION
    DemoMarker.Open(f, 0)
    DemoQueue = DemoMarker.ReadLine
    DemoMarker.Close
    DemoSeq = DemoSeq + 1
    IF DemoQueue = "" THEN DemoQueue = "wait"
  END IF
  ' (one command a tick, as --do runs them)
  k = INSTR(DemoQueue, ",")
  IF k > 0 THEN
    id = LTRIM$(RTRIM$(LEFT$(DemoQueue, k - 1)))
    DemoQueue = MID$(DemoQueue, k + 1)
  ELSE
    id = LTRIM$(RTRIM$(DemoQueue))
    DemoQueue = ""
  END IF
  IF id <> "" AND id <> "wait" THEN
    IF LEFT$(id, 5) = "lang:" THEN
      SetLanguage MID$(id, 6)
    ELSEIF EditorStep(id) = 0 THEN
      IF SafetyStep(id) = 0 THEN
        IF PanelStep(id) = 0 THEN
          IF DebugStep(id) = 0 THEN RunCommand id
        END IF
      END IF
    END IF
  END IF
  IF DemoQueue = "" THEN
    DemoMarker.Open(folder + "/inbox/done-" + LTRIM$(STR$(DemoSeq)), 65535)
    DemoMarker.WriteStr("done", 4)
    DemoMarker.Close
  END IF
END FUNCTION
