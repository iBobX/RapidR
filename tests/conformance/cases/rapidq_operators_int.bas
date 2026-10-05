' The integer operators as RapidQ computes them — the .expected is RC.EXE's
' output (docs/rapidq-ground-truth.md): `\` rounds each operand as CINT does
' (INT(x + 0.5)) then truncates the quotient; MOD, AND, OR, XOR, NOT, SHL and
' SHR take 32-bit integers rounded half to even (beyond 32 bits:
' -2147483648); a shift count's low 5 bits count; &H80000000 … &HFFFFFFFF are
' negative; arithmetic is in floating point.
PRINT "a "; 7 \ 2; " "; -7 \ 2; " "; 7.9 \ 2; " "; -7.9 \ 2; " "; 7.5 \ 2; " "; -7.5 \ 2; " "; 6.5 \ 2; " "; 7 \ 2.5
PRINT "b "; 7.1 \ 1; " "; -7.1 \ 1; " "; 0.1 \ 1; " "; -0.1 \ 1; " "; 8.5 \ 2
PRINT "c "; 7 MOD 3; " "; -7 MOD 3; " "; 7 MOD -3; " "; 7.5 MOD 2; " "; 6.5 MOD 4; " "; -6.5 MOD 4; " "; 5.5 MOD 2.5; " "; 8.5 MOD 3
PRINT "d "; 5 AND 3; " "; 5 OR 2; " "; 7 XOR 2; " "; 2.7 AND 3; " "; 2.5 OR 0; " "; -2.5 OR 0; " "; NOT 0; " "; NOT 1; " "; NOT -1
PRINT "e "; 3000000000 AND 255; " "; 1E10 OR 0; " "; -1 AND 3000000000; " "; 7 MOD 3000000000
PRINT "f "; 5 SHL 2; " "; -8 SHR 1; " "; 1 SHL 31; " "; 1 SHL 32; " "; -1 SHR 28; " "; 2.5 SHL 1
PRINT "g "; &H80000001 SHL 1; " "; &HFFFFFFFF; " "; &H7FFFFFFF; " "; &HFF
PRINT "h "; (1 = 1); " "; (1 < 2); " "; (1 > 2); " "; (2 = 2) + 1
DIM i AS INTEGER
i = 2147483647
PRINT "i "; i + 1; " "; i * 2; " "; (i + 1) / 2; " "; i + 0.5; " "; -i - 2; " "; 65536 * 65536
PRINT "j "; i AND i; " "; NOT i; " "; i XOR -1
PRINT "k "; 3 INV 26; " "; 2 INV 4; " "; 0 INV 5; " "; 7 INV 1; " "; 5 INV 0
