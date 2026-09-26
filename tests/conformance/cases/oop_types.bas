' xfail: codegen — native builds don't compile OOP TYPEs yet (a clear compile error; in progress)
' TYPE with fields (incl. an array field), SUB/FUNCTION methods using This and bare field
' names, a CONSTRUCTOR, inheritance (base constructor first) and instances passed to SUBs.
TYPE TCounter
  Count AS INTEGER
  Increment AS INTEGER
  Names(2) AS STRING
  SUB Bump
    This.Count = This.Count + This.Increment
  END SUB
  FUNCTION Twice() AS INTEGER
    Twice = Count * 2
  END FUNCTION
  CONSTRUCTOR
    Increment = 5
  END CONSTRUCTOR
END TYPE

TYPE TLoud EXTENDS TCounter
  SUB Shout
    PRINT "count is " + STR$(This.Count)
  END SUB
  CONSTRUCTOR
    Count = 100
  END CONSTRUCTOR
END TYPE

SUB UseIt(obj AS TCounter)
  obj.Bump
  PRINT "via param " + STR$(obj.Count)
END SUB

DIM c AS TCounter
c.Bump
c.Bump
PRINT STR$(c.Count)
PRINT STR$(c.Twice())
c.Names(1) = "x,y"
PRINT c.Names(1) + "|" + c.Names(2)
DIM l AS TLoud
l.Bump
l.Shout
UseIt c
