' RapidQ's own answers (RC.EXE): ENVIRON's string split at its first = (or a space), names in any case
$APPTYPE CONSOLE
ENVIRON "RQ_X=hello=1"
PRINT "["; ENVIRON$("RQ_X"); "]"
ENVIRON "RQ_Y two words"
PRINT "["; ENVIRON$("RQ_Y"); "]"
ENVIRON "RQ_X="
PRINT "["; ENVIRON$("RQ_X"); "]"
PRINT "["; ENVIRON$("rq_y"); "]"
ENVIRON "RQ_Z = sp"
PRINT "["; ENVIRON$("RQ_Z"); "]["; ENVIRON$("RQ_Z "); "]"
