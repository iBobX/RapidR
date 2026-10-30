<!-- translated-from: rich-text.md 819b0042d51f3e30 -->
# Texto enriquecido: RRichEdit

Un componente `RRichEdit` (nombre anterior `QRICHEDIT`) es un editor de texto
cuyo texto tiene formato: cada carácter tiene su fuente, tamaño, color, negrita,
cursiva, subrayado y tachado, y cada párrafo su alineación, sus sangrías y una
viñeta. Los programas le dan formato mediante `SelAttributes`, el usuario escribe
en él, y carga y guarda archivos de texto enriquecido (`.rtf`) que también abren
WordPad, Word, LibreOffice y TextEdit. Funciona igual en las compilaciones
nativas, en el RapidR Runtime y en el navegador.

![Un editor de texto enriquecido pequeño: una barra de herramientas con “B”, “I”, “U”, “Colour”, “Font”, alineación y viñetas sobre un documento con formato](../images/rich-text/editor.png)

El ejemplo de arriba es
[examples/gui/richtext.rr](../../../examples/gui/richtext.rr):
`rapidr run examples/gui/richtext.rr`.

## Dar formato al texto desde un programa

Selecciona el texto con `SelStart` y `SelLength` y luego establece
`SelAttributes`:

```basic
CREATE Form AS RForm
    Width = 400: Height = 200
    CREATE Notes AS RRichEdit
        Align = alClient
    END CREATE
END CREATE

Notes.Text = "Rich text: bold and red"
Notes.SelStart = 11: Notes.SelLength = 4      ' "bold"
Notes.SelAttributes.Bold = 1
Notes.SelStart = 20: Notes.SelLength = 3      ' "red"
Notes.SelAttributes.Color = &HFF
Form.ShowModal
```

`SelAttributes` tiene los miembros de una fuente: `Color`, `Name`, `Size`,
`Bold`, `Italic`, `Underline`, `StrikeOut` (y `AddStyles` / `DelStyles` con
`fsBold`, `fsItalic`, `fsUnderline`, `fsStrikeOut`). También puedes darle de una
vez una fuente `RFont` completa — cada valor de la fuente pasa a la selección:

```basic
DIM Title AS RFont
Title.Name = "Arial": Title.Size = 16: Title.Color = &H804000
Title.AddStyles(fsBold)
Notes.SelStart = 0: Notes.SelLength = 9
Notes.SelAttributes = Title
```

Las posiciones cuentan un salto de línea como dos caracteres, tal como los tiene
`Text` (CR LF), así que una posición que `INSTR` encuentra en `Text` selecciona
ese texto:

```basic
SUB Highlight(Word AS STRING)
    Notes.SelStart = INSTR(Notes.Text, Word) - 1
    Notes.SelLength = LEN(Word)
    Notes.SelAttributes.Color = &HFF0000
END SUB
```

### En el cursor: lo que se escribe a continuación

Sin nada seleccionado, `SelAttributes` es el formato de lo que se escribe o se
añade a continuación en el cursor — hasta que el cursor se mueve. Así un
registro puede dar color a cada línea que añade:

```basic
Log.SelStart = LEN(Log.Text)
Log.SelAttributes.Color = &H008000
Log.AddStrings "Conectado."
```

`AddStrings` añade sus líneas al final del texto y deja el cursor después de
ellas.

### Leer el formato

Leer un miembro de `SelAttributes` da el formato del último carácter de la
selección o, en un cursor, el formato que tendría el texto escrito. Una barra de
herramientas lo usa para mostrar el estado de la selección:

```basic
SUB ShowState
    BoldBtn.Down = Doc.SelAttributes.Bold
    ItalicBtn.Down = Doc.SelAttributes.Italic
END SUB
```

El color automático (el color de texto de la ventana, el que tiene el texto
hasta que se le da otro) se lee como `&H80000008`.

## Lo que escribe el usuario

El texto escrito toma el formato del carácter que tiene delante — al inicio de
un párrafo, el del carácter que tiene detrás. El texto escrito sobre una
selección toma el formato del primer carácter de la selección. `Ctrl+Z`
(`Cmd+Z` en macOS) deshace el último cambio y devuelve también sus formatos.

El texto copiado (o cortado) de un editor de texto enriquecido y pegado en otro
conserva sus formatos — con las teclas, con el menú del clic derecho, o con
`CopyToClipboard` / `PasteFromClipboard` de un programa. Pegado en un campo de
texto sin formato, o en otra aplicación, es solo el texto.

## Font y Alignment

`Font` es el formato del texto que nunca recibió formato: mientras nada del
control tenga formato, establecer `Font` cambia todo el texto. Cuando una parte
del texto ya tiene su propio formato, el resto conserva la fuente que muestra, y
`Font` solo cambia el final del texto (lo que se escribe en una última línea
vacía).

`Alignment` (`taLeftJustify`, `taCenter`, `taRightJustify`) alinea los párrafos
que no tienen alineación propia; el texto que se establece con `Text` después la
recibe como propia.

Estas son las reglas clásicas: los programas anteriores dan formato y guardan
exactamente como siempre.

## Párrafos (los de RapidR)

`Paragraph` da formato a los párrafos en que está la selección (en un cursor, su
propio párrafo):

| Miembro | Qué hace |
|---|---|
| `Paragraph.Alignment` | `taLeftJustify`, `taCenter` o `taRightJustify` |
| `Paragraph.LeftIndent` | qué tan adentro empiezan las líneas, en píxeles |
| `Paragraph.FirstIndent` | cuánto más adentro empieza la primera línea (menos que 0: una sangría francesa) |
| `Paragraph.RightIndent` | qué tan adentro desde la derecha terminan las líneas |
| `Paragraph.Numbering` | 1: una viñeta antes del párrafo; 0: ninguna |

```basic
Doc.SelStart = INSTR(Doc.Text, "Compras") - 1
Doc.Paragraph.Alignment = taCenter
Doc.SelStart = INSTR(Doc.Text, "pan") - 1
Doc.SelLength = 20                       ' pan, leche, huevos …
Doc.Paragraph.Numbering = 1
```

## Archivos: texto enriquecido y texto sin formato

`SaveToFile` escribe el texto con sus formatos como RTF, y `LoadFromFile` vuelve a
leer el RTF — cualquier RTF que escriba un procesador de textos; un archivo que
no es RTF se carga como su texto sin formato. Con `PlainText = 1` ambos
funcionan con texto sin formato (líneas que terminan en CR LF). `SaveToStream` y
`LoadFromStream` hacen lo mismo con un flujo.

```basic
Doc.SaveToFile "letter.rtf"        ' con sus formatos
Doc.PlainText = 1
Doc.SaveToFile "letter.txt"        ' solo el texto
```

`TextRTF` (el de RapidR) es el mismo RTF como cadena de texto: léelo para guardar
una nota con formato en una base de datos, establécelo para mostrar una.

## Lectores de pantalla

Un lector de pantalla lee el texto del editor de texto enriquecido con sus
formatos: VoiceOver, NVDA y Narrator informan la fuente, el tamaño, la negrita,
la cursiva, el subrayado y el color del texto que está bajo el cursor (con sus
comandos para informar del formato), y la descripción del control dice el
formato en el cursor (“At the caret: Arial 12 point, bold, red”). En el
navegador la descripción es el `aria-description` del campo.

## Compatibilidad

Los miembros de `QRICHEDIT` de los programas anteriores se comportan como
siempre (el conjunto de pruebas de compatibilidad los cubre): `SelAttributes` y
sus miembros, `SelFont` (otro nombre de `SelAttributes = Font`), `Font`,
`Alignment`, `PlainText`, `SelText`, `AddStrings`, `Line`, `Clear`,
`LoadFromFile` / `SaveToFile` y los flujos (los archivos de texto enriquecido de
esos programas, byte por byte). El ejemplo `richedit/syntax` del conjunto
(resaltado de sintaxis) muestra sus colores como siempre.

Diferencias que quedan: los miembros clásicos `LineCount` y `Line(i)` cuentan las
líneas tal como se ajustan en la pantalla cuando `WordWrap` está activado, los de
RapidR cuentan párrafos (como los clásicos con `WordWrap = 0`); después de
`SelectAll`, el `SelLength` clásico cuenta el final del texto como dos
caracteres más; un documento cargado desde RTF se guarda con el encabezado de
RapidR (el clásico conserva el del archivo cargado).
