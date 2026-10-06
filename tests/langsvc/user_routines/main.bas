DIM total AS INTEGER
SUB Greet(who AS STRING, times AS INTEGER)
    DIM i AS INTEGER
    FOR |i = 1 TO times
        PRINT "Hi "; who
        total = total + 1
    NEXT
    |
END SUB
FUNCTION Twice(n AS INTEGER) AS INTEGER
    Twice = n * 2
END FUNCTION
Greet "Ann", |2
PRINT Twice(|3)
Gr|eet "Bo", 1
PRINT to|tal
|
'! 1 rename who error "already the name"
'! 2 completion has who times i total Greet Twice LEFT$ PRINT
'! 3 signature "SUB Greet(who AS STRING, times AS INTEGER)" active 1
'! 4 signature "FUNCTION Twice(n AS INTEGER) AS INTEGER" active 0
'! 5 definition main.bas:2:5
'! 5 references main.bas:2:5 main.bas:13:1 main.bas:15:1
'! 5 hover has "SUB Greet(who AS STRING, times AS INTEGER)"
'! 6 hover has "DIM total AS INTEGER" "global variable"
'! 6 rename count main.bas:1:5 main.bas:6:9 main.bas:6:17 main.bas:16:7
'! 6 rename Twice error "already the name"
'! 7 completion has total Greet Twice
'! 7 completion lacks who times i
'! diagnostics none
