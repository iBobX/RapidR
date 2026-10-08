' A $DEFINE matches its name in any case, as whole words outside strings
' (RC.EXE's output; RapidQ's RAPIDQ2.INC defines BOOLEAN and writes
' `AS boolean`, its gl.inc GLint and the OpenGL examples `AS glInt`).
$APPTYPE CONSOLE
$DEFINE GLint integer
$DEFINE Foo 5
DIM a AS glInt
a = 2.7
PRINT a
PRINT foo
PRINT FOO + 1
PRINT "foo Foo"
foox = 3
PRINT foox
