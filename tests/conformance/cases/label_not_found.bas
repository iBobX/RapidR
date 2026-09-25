' xfail: codegen — native builds refuse labels/GOTO/GOSUB with a clear compile error (state-machine lowering planned)
PRINT "start"
GOTO Nowhere
SUB Other()
There:
END SUB
GOSUB There
