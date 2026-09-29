' Builtins as the RapidQ manual defines them: INSERT$, FORMAT$ (Pascal
' Format), STRF$ (FloatToStrF), and RANDOMIZE giving a repeatable sequence
PRINT INSERT$("hi", "Hello", 3)
PRINT INSERT$("open error: ", "x", 1)
PRINT FORMAT$("Location: %s %4d %-10.4g|", "Any", 1234, 55.39)
PRINT FORMAT$("%d %d %0:d %d", 10, 20)
PRINT FORMAT$("%.5d|%5d|%-5d|%x|%.4x", 42, 42, 42, 255, 255)
PRINT FORMAT$("%.2f %.1f %n %m", 3.14159, 2.25, 1234567.891, -2)
PRINT FORMAT$("%e|%g|%.3s|100%%", 1234.5, 0.5, "abcdef")
PRINT STRF$(99.9934, 0, 4, 4)
PRINT STRF$(12345678, 3, 8, 0)
PRINT STRF$(3.14159, 2, 2, 4)
PRINT STRF$(1234.5, 1, 4, 2)
RANDOMIZE 42
a = RND(1000): b = RND(1000): c = RND
RANDOMIZE 42
IF a = RND(1000) AND b = RND(1000) AND c = RND THEN PRINT "same sequence" ELSE PRINT "different"
PRINT RND(1) = 0
