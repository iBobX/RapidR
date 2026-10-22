' A parameter or a variable named like a statement keyword RapidQ leaves
' free (Open, Close, Input, Kill, Seek, Write) never changes a string
' literal: "Open" prints Open after, before and inside the SUB or FUNCTION,
' called or not, beside a local DIM or a variable of that name (the literal
' was read as the parameter, empty outside its SUB: Database Studio's Open
' button went blank). RC.EXE prints exactly this; it refuses Input and Kill
' as names of the main program's variables, so those two aren't DIMmed there.
SUB ShowOpen(Open AS STRING)
  PRINT "Open: sub [" + Open + "] Open " + "Open"
END SUB
PRINT "Open"
PRINT "Open it, " + "Open" + "!"; " "; "open"; " "; "OPEN"
FUNCTION GetOpen(Open AS STRING) AS STRING
  GetOpen = "Open" + "=" + Open
END FUNCTION
PRINT "Open"
SUB UnusedOpen(Open AS STRING)
  PRINT Open
END SUB
PRINT "Open" + "Open"
FUNCTION UnusedFOpen(Open AS STRING) AS STRING
  UnusedFOpen = Open
END FUNCTION
s$ = "Open": PRINT s$; " "; "xOpenx"
ShowOpen "a"
PRINT GetOpen("b")
SUB ShowClose(Close AS STRING)
  PRINT "Close: sub [" + Close + "] Close " + "Close"
END SUB
PRINT "Close"
PRINT "Close it, " + "Close" + "!"; " "; "close"; " "; "CLOSE"
FUNCTION GetClose(Close AS STRING) AS STRING
  GetClose = "Close" + "=" + Close
END FUNCTION
PRINT "Close"
SUB UnusedClose(Close AS STRING)
  PRINT Close
END SUB
PRINT "Close" + "Close"
FUNCTION UnusedFClose(Close AS STRING) AS STRING
  UnusedFClose = Close
END FUNCTION
s$ = "Close": PRINT s$; " "; "xClosex"
ShowClose "a"
PRINT GetClose("b")
SUB ShowInput(Input AS STRING)
  PRINT "Input: sub [" + Input + "] Input " + "Input"
END SUB
PRINT "Input"
PRINT "Input it, " + "Input" + "!"; " "; "input"; " "; "INPUT"
FUNCTION GetInput(Input AS STRING) AS STRING
  GetInput = "Input" + "=" + Input
END FUNCTION
PRINT "Input"
SUB UnusedInput(Input AS STRING)
  PRINT Input
END SUB
PRINT "Input" + "Input"
FUNCTION UnusedFInput(Input AS STRING) AS STRING
  UnusedFInput = Input
END FUNCTION
s$ = "Input": PRINT s$; " "; "xInputx"
ShowInput "a"
PRINT GetInput("b")
SUB ShowKill(Kill AS STRING)
  PRINT "Kill: sub [" + Kill + "] Kill " + "Kill"
END SUB
PRINT "Kill"
PRINT "Kill it, " + "Kill" + "!"; " "; "kill"; " "; "KILL"
FUNCTION GetKill(Kill AS STRING) AS STRING
  GetKill = "Kill" + "=" + Kill
END FUNCTION
PRINT "Kill"
SUB UnusedKill(Kill AS STRING)
  PRINT Kill
END SUB
PRINT "Kill" + "Kill"
FUNCTION UnusedFKill(Kill AS STRING) AS STRING
  UnusedFKill = Kill
END FUNCTION
s$ = "Kill": PRINT s$; " "; "xKillx"
ShowKill "a"
PRINT GetKill("b")
SUB ShowSeek(Seek AS STRING)
  PRINT "Seek: sub [" + Seek + "] Seek " + "Seek"
END SUB
PRINT "Seek"
PRINT "Seek it, " + "Seek" + "!"; " "; "seek"; " "; "SEEK"
FUNCTION GetSeek(Seek AS STRING) AS STRING
  GetSeek = "Seek" + "=" + Seek
END FUNCTION
PRINT "Seek"
SUB UnusedSeek(Seek AS STRING)
  PRINT Seek
END SUB
PRINT "Seek" + "Seek"
FUNCTION UnusedFSeek(Seek AS STRING) AS STRING
  UnusedFSeek = Seek
END FUNCTION
s$ = "Seek": PRINT s$; " "; "xSeekx"
ShowSeek "a"
PRINT GetSeek("b")
SUB ShowWrite(Write AS STRING)
  PRINT "Write: sub [" + Write + "] Write " + "Write"
END SUB
PRINT "Write"
PRINT "Write it, " + "Write" + "!"; " "; "write"; " "; "WRITE"
FUNCTION GetWrite(Write AS STRING) AS STRING
  GetWrite = "Write" + "=" + Write
END FUNCTION
PRINT "Write"
SUB UnusedWrite(Write AS STRING)
  PRINT Write
END SUB
PRINT "Write" + "Write"
FUNCTION UnusedFWrite(Write AS STRING) AS STRING
  UnusedFWrite = Write
END FUNCTION
s$ = "Write": PRINT s$; " "; "xWritex"
ShowWrite "a"
PRINT GetWrite("b")
SUB Locals
  DIM Open AS STRING
  DIM Close AS STRING
  DIM Input AS STRING
  DIM Kill AS STRING
  DIM Seek AS STRING
  DIM Write AS STRING
  Open = "o": Close = "c": Input = "i": Kill = "k": Seek = "s": Write = "w"
  PRINT Open; Close; Input; Kill; Seek; Write; " "; "Open"; " "; "Close"; " "; "Input"; " "; "Kill"; " "; "Seek"; " "; "Write"
END SUB
Locals
DIM Open AS INTEGER
Open = 1
PRINT Open; " "; "Open"; " "; "Open"; Open
DIM Close AS INTEGER
Close = 2
PRINT Close; " "; "Close"; " "; "Close"; Close
DIM Seek AS INTEGER
Seek = 3
PRINT Seek; " "; "Seek"; " "; "Seek"; Seek
DIM Write AS INTEGER
Write = 4
PRINT Write; " "; "Write"; " "; "Write"; Write
