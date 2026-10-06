' RNUM (one implementation on every runtime: rapidr-value's datascience::num):
' creation, aggregates, element-wise math, arrays as operands, ordering,
' cumulative, search, the web's Get / Set / Push, properties, seeded random.
DIM a AS RNUM, b AS RNUM, c AS RNUM, r AS RNUM
a.Arange(0, 10, 2)
PRINT "arange: "; a.ToList; " size "; a.Size; " shape "; a.Shape
PRINT "sum "; a.Sum; " mean "; a.Mean; " min "; a.Min; " max "; a.Max; " ptp "; a.Ptp
PRINT "std "; FORMAT$("%.4f", a.Std); " var "; a.Var; " median "; a.Median
PRINT "argmin "; a.ArgMin; " argmax "; a.ArgMax
a.Arange(5, 0, -2)
PRINT "down: "; a.ToList
a.Linspace(0, 1, 5)
PRINT "linspace: "; a.ToList
a.Multiply(4)
a.Add(1)
PRINT "times 4 plus 1: "; a.ToList
b.FromList("10, 20, 30, 40, 50")
a.Add(b)
PRINT "plus b: "; a.ToList
PRINT "dot: "; a.Dot(b); " norm: "; FORMAT$("%.3f", b.Norm)
c.FromList("3,1,2,3,1")
c.Unique
PRINT "unique: "; c.ToList
c.FromList("4,2,6")
c.Sort
c.Reverse
PRINT "sorted down: "; c.ToList
c.Cumsum
PRINT "cumsum: "; c.ToList
c.Diff
PRINT "diff: "; c.ToList
c.FromList("1,4,9,16")
c.Sqrt
PRINT "sqrt: "; c.ToList; " searchsorted(2.5) "; c.SearchSorted(2.5)
c.Push(5)
c.Set(6, 7)
PRINT "push/set: "; c.ToList; " get(1) "; c.Get(1)
c.Slice(1, 4)
PRINT "slice: "; c.ToList; " any "; c.Any; " all "; c.All
c.Data = "-1.5, 0, 2.5"
c.Clip(-1, 1)
PRINT "clip: "; c.ToList
c.Data = "1.25, 2.75"
c.Round(1)
PRINT "round: "; c.ToList; " ndim "; c.NDim; " dtype "; c.DType
c.Print
RANDOMIZE 7
r.Rand(3)
first$ = r.ToList
RANDOMIZE 7
r.Rand(3)
PRINT "seeded random repeats: "; IIF(first$ = r.ToList, "yes", "no")
r.RandInt(1, 6, 50)
PRINT "dice in range: "; IIF(r.Min >= 1 AND r.Max <= 6, "yes", "no")
