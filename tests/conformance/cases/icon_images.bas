' Icons as pictures: QIMAGELIST.AddICOHandle / InsertICOHandle (an icon is
' scaled to the list's size, never cut into pieces), GetICO, and a QBITMAP
' loading an .ICO; see-through parts kept (Pixel reads what's drawn).
$RESOURCE DISC AS "picture_files/rr_disc.ico"
DIM L AS QIMAGELIST
L.Width = 8 : L.Height = 8
L.AddICOHandle DISC
L.InsertICOHandle 0, DISC
PRINT L.Count
DIM B AS QBITMAP
B.Width = 8 : B.Height = 8
B.FillRect(0, 0, 8, 8, &HFFFFFF)
L.Draw(B, 0, 0, 1)
PRINT HEX$(B.Pixel(4, 4)); " "; HEX$(B.Pixel(0, 0))
DIM Icon AS QBITMAP
Icon.BMP = L.GetICO(0)
PRINT Icon.Width; Icon.Height
DIM Whole AS QBITMAP
Whole.BMPHandle = DISC
PRINT Whole.Width; " "; HEX$(Whole.Pixel(8, 8))
