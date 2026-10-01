' Custom events (manual 10.9): an AS EVENT(Template) field given a SUB holds its pointer; CALLFUNC fires it; 0 until given one.
DECLARE SUB Ready_Template (N AS LONG)
TYPE TWorker EXTENDS QOBJECT
  OnReady AS EVENT(Ready_Template)
  Id AS LONG
  SUB Work
    IF This.OnReady > 0 THEN CALLFUNC(This.OnReady, This.Id)
  END SUB
END TYPE
SUB Ready (N AS LONG)
  PRINT N; " is ready."
END SUB
DIM W AS TWorker
W.Id = 7
W.Work
W.OnReady = Ready
W.Work
