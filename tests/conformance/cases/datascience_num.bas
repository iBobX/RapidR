' RNUM on the one data-science model (rapidr_value::datascience): every
' member gives the same result in native builds, the interpreter and the
' browser. (Random members: only what doesn't depend on the numbers drawn.)
DIM a AS RNum
DIM b AS RNum
DIM c AS RNum

' --- Creation ---
a.arange 0, 10, 1
PRINT a.tolist
a.arange 0, 1, 0.25
PRINT a.tolist
a.arange 5, 0, -2
PRINT a.tolist
a.linspace 0, 1, 5
PRINT a.tolist
a.zeros 3
PRINT a.tolist
a.ones 2
PRINT a.tolist
a.full 3, 7.5
PRINT a.tolist
a.fromlist "4, 1, 3, 1, 5"
PRINT a.tolist
a.create 2
a.set 3, 9
a.push 11
PRINT a.tolist; " "; a.get(3); " "; a.at(9)

' --- Properties ---
a.fromlist "1,2,3,4"
PRINT a.Size; " "; a.Length; " "; a.Shape; " "; a.NDim; " "; a.DType; " "; a.Data
a.Data = "5,6,7"
PRINT a.Count; " "; a.Data

' --- Aggregation ---
a.fromlist "2,4,4,4,5,5,7,9"
PRINT a.sum; " "; a.mean; " "; a.avg; " "; a.min; " "; a.max
PRINT a.std; " "; a.var; " "; a.median; " "; a.argmin; " "; a.argmax; " "; a.count; " "; a.ptp
PRINT a.Sum(); " "; a.Mean()

' --- Element-wise math (in place) ---
a.fromlist "1,4,9"
a.sqrt
PRINT a.tolist
a.square
PRINT a.tolist
a.negative
PRINT a.tolist
a.abs
PRINT a.tolist
a.fromlist "-2.5,0,3.7"
a.sign
PRINT a.tolist
a.fromlist "-2.5,0.4,3.7"
a.floor
PRINT a.tolist
a.fromlist "-2.5,0.4,3.7"
a.ceil
PRINT a.tolist
a.fromlist "1.2345,2.5551"
a.round 2
PRINT a.tolist
a.fromlist "1,10,100"
a.log10
PRINT a.tolist
a.fromlist "1,2,8"
a.log2
PRINT a.tolist
a.fromlist "0"
a.exp
PRINT a.tolist
a.cos
PRINT a.tolist
a.fromlist "2,4"
a.reciprocal
PRINT a.tolist

' --- Arithmetic with another array or a number ---
a.fromlist "1,2,3"
b.fromlist "10,20,30"
a.add b
PRINT a.tolist
a.subtract 1
PRINT a.tolist
a.multiply "b"
PRINT a.tolist
a.divide 10
PRINT a.tolist
a.power 2
PRINT a.tolist
a.mod 7
PRINT a.tolist
a.clip 2, 5
PRINT a.tolist
c.fromlist "1,2"
a.add c
PRINT a.tolist

' --- Ordering / manipulation ---
a.fromlist "3,1,2,3"
a.sort
PRINT a.tolist
a.reverse
PRINT a.tolist
a.unique
PRINT a.tolist
a.append b
PRINT a.tolist
a.slice 1, 4
PRINT a.tolist
a.shuffle
PRINT a.count; " "; a.sum
a.reshape 3

' --- Cumulative ---
a.fromlist "1,2,3,4"
a.cumsum
PRINT a.tolist
a.fromlist "1,2,3,4"
a.cumprod
PRINT a.tolist
a.diff
PRINT a.tolist

' --- Dot product / linear algebra ---
a.fromlist "3,4"
b.fromlist "1,2"
PRINT a.dot("b"); " "; a.norm
a.normalize
PRINT a.tolist

' --- Boolean / search ---
a.fromlist "0,2,0,3"
PRINT a.any; " "; a.all
a.where
PRINT a.tolist
a.fromlist "1,3,5,7"
PRINT a.searchsorted(4); " "; a.searchsorted(0); " "; a.searchsorted(9)

' --- Random (instance methods) ---
a.rand 4
PRINT a.count; " "; a.min >= 0; " "; a.max < 1
a.randn 5, 10, 2
PRINT a.count
a.uniform 2, 3, 6
PRINT a.count; " "; a.min >= 2; " "; a.max < 3
a.randint 1, 3, 50
a.unique
PRINT a.tolist
a.fromlist "4,4,4"
PRINT a.choice
a.choice 5
PRINT a.tolist

' --- Output ---
a.fromlist "1,2.5"
a.print
PRINT a.tostring
a.clear
PRINT a.count; " ["; a.tolist; "]"
