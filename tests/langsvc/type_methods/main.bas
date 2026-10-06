TYPE TCounter
  Count AS INTEGER
  Delta AS INTEGER
  SUB Bump(times AS INTEGER)
    This.|Count = Count + Delta * times
  END SUB
  FUNCTION Twice() AS INTEGER
    Twice = Cou|nt * 2
  END FUNCTION
END TYPE

TYPE TLoud EXTENDS TCounter
  SUB Shout
    PRINT STR$(This.Count)
  END SUB
END TYPE

DIM c AS TCounter
DIM l AS TLoud
PRINT c.Cou|nt
c.|
c.Bump(|1)
l.|
'! 1 completion has Count Delta Bump Twice
'! 2 definition main.bas:2:3
'! 2 references main.bas:2:3 main.bas:5:10 main.bas:5:18 main.bas:8:13 main.bas:14:21 main.bas:20:9
'! 4 completion has Count Delta Bump Twice
'! 4 completion lacks Shout PRINT
'! 5 signature "SUB Bump(times AS INTEGER)" active 0
'! 6 completion has Count Bump Shout
'! 3 hover has "field of TYPE `TCounter`"
'! 3 rename Total main.bas:2:3 main.bas:5:10 main.bas:5:18 main.bas:8:13 main.bas:14:21 main.bas:20:9
