' JPEG pictures wherever a bitmap goes ($RESOURCE, QBITMAP.BMPHandle):
' its size and colors (a JPEG is lossy: near, not exact).
$RESOURCE PHOTO AS "picture_files/two_colors.jpg"
DIM B AS QBITMAP
B.BMPHandle = PHOTO
PRINT B.Width; B.Height
FUNCTION Near(c AS LONG, r AS INTEGER, g AS INTEGER, b AS INTEGER) AS INTEGER
  DIM cr AS INTEGER, cg AS INTEGER, cb AS INTEGER
  cr = c AND 255
  cg = (c \ 256) AND 255
  cb = (c \ 65536) AND 255
  Near = (ABS(cr - r) < 12) AND (ABS(cg - g) < 12) AND (ABS(cb - b) < 12)
END FUNCTION
PRINT Near(B.Pixel(2, 4), 200, 120, 40); Near(B.Pixel(13, 4), 230, 230, 230)
