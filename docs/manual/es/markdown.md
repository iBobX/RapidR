<!-- translated-from: markdown.md 73186c84654abbee -->
# Archivos Markdown y RMarkdownView

Markdown es el formato de texto sin formato de los archivos `README.md`, las
notas y los informes: `# Heading`, `**bold**`, `- list item`, `` `code` ``,
`[link](url)`. RapidR lo muestra **con formato**, como lo ve un lector — en
RapidR Studio, y en tus propios programas con el componente `RMarkdownView`.

## En RapidR Studio

![Un README.md abierto en RapidR Studio, mostrado con formato (Vista previa)](../images/markdown/preview.png)

**Abrir un archivo Markdown.** Un archivo que termina en `.md` (o `.markdown`)
se abre en **Vista previa**: encabezados, listas, tablas y código como están
hechos para leerse. Ábrelo desde el panel Proyecto (doble clic), con **Archivo >
Abrir proyecto o archivo…** (Ctrl+O; un archivo `.md` se abre solo, con o sin un
proyecto), o haciendo clic en un enlace a él en otro archivo Markdown. El
informe que RapidR escribe cuando importa un programa anterior
(`rapidr-import-report.md`) se abre de la misma manera.

![Un informe de importación en Vista previa: su resumen, una tabla y sus encabezados en Estructura](../images/markdown/import-report.png)

**Vista previa | Código fuente.** A la derecha de las pestañas de documentos, un
archivo Markdown tiene un interruptor, como el archivo de un formulario tiene
*Diseño | Código*:

| Segmento | Atajo | Muestra |
|---|---|---|
| **Vista previa** | Shift+F7 (Ver > Diseñador) | el texto con formato |
| **Código fuente** | F7 (Ver > Código) | el texto Markdown, para editarlo |
| **Lado a lado** (el último botón) | Ver > Diseñador y código lado a lado | los dos: lo que escribes en el código fuente se ve en la vista previa al instante |

F12 (Ver > Alternar formulario / código) cambia entre Vista previa y Código
fuente. Los cambios se guardan con **Archivo > Guardar** (Ctrl+S), como con el
código; la pestaña muestra un punto mientras un cambio no está guardado.

![El mismo README lado a lado: la vista previa a la izquierda, su código fuente a la derecha](../images/markdown/side-by-side.png)

**Leer.** Desplázate con la rueda del mouse, la barra de desplazamiento o las
teclas (Arriba / Abajo, Page Up / Page Down, Space, Home / End). Arrastra sobre
el texto para seleccionarlo; luego Ctrl+C (Cmd+C en una Mac) o **Edición >
Copiar** lo copia; Ctrl+A o **Edición > Seleccionar todo** selecciona todo. Las
celdas de una tabla se copian separadas por tabuladores, así que se pegan en una
hoja de cálculo como columnas.

**Enlaces.** Haz clic en un enlace:

- una dirección web (`https://…`, `mailto:…`) se abre en tu navegador;
- otro archivo (`notes.md`, `../docs/setup.md`) se abre en Studio — uno Markdown
  en Vista previa, en el encabezado que nombra el enlace (`notes.md#install`);
- un encabezado del mismo archivo (`#running-it`) se desplaza hasta allí.

Los enlaces también funcionan con el teclado. Con el foco en la vista previa,
**Tab** pasa al primer enlace y luego a cada uno de los siguientes — un anillo lo
marca, y la vista previa se desplaza hasta él — y sale de la vista previa después
del último; **Shift+Tab** va en el otro sentido. **Enter** o **Space** sigue el
enlace, como lo hace un clic. Lo mismo vale para un componente `RMarkdownView` en
tus propios programas (más abajo), donde `OnLinkClick` lo recibe.

**Estructura.** El panel Estructura enumera los encabezados del archivo,
anidados por nivel. Haz clic en uno: la vista previa se desplaza hasta él (en
Código fuente, el editor va a su línea).

El código fuente no se colorea como BASIC: un `'` en una oración es solo un
apóstrofo.

## En tus programas: RMarkdownView

El componente `RMarkdownView` muestra un texto Markdown con formato, igual en el
escritorio y en la web, con el tema del programa (claro, oscuro, contraste alto).
Los lectores de pantalla oyen sus encabezados (con su nivel), listas, tablas y
enlaces.

```basic
' alClient y alBottom vienen integrados: no hace falta incluir nada en un programa .rr

SUB LinkClicked (Url AS STRING)
  Status.Caption = "Hiciste clic en " + Url
END SUB

CREATE Form AS RForm
  Caption = "Léeme"
  Width = 640: Height = 480
  CREATE Doc AS RMarkdownView
    Align = alClient
    OnLinkClick = LinkClicked
  END CREATE
  CREATE Status AS RLabel
    Align = alBottom
  END CREATE
END CREATE

IF Doc.LoadFromFile("README.md") = 0 THEN
  Doc.Text = "# Sin README" + CHR$(10) + CHR$(10) + "*README.md* no se encontró."
END IF
Form.ShowModal
```

| Miembro | Qué hace |
|---|---|
| `Text` | el texto Markdown que se muestra (establécelo para mostrar otro) |
| `LoadFromFile(File)` | muestra un archivo; True cuando se leyó |
| `OnLinkClick(Url)` | se hizo clic en un enlace; `Url` es su destino como está escrito |
| `OpenLinks` | True (el valor predeterminado): un enlace web se abre solo en el navegador |
| `CodeLanguage`, `OnCodeClick(Word, OfType)` | el lenguaje del `inline code` del texto, y un clic en una de sus palabras: mira más abajo |
| `ScrollTo(Heading)` | desplaza un encabezado hasta arriba, por su texto o su ancla (`"#running-it"`) |
| `Headings` | los encabezados, una línea cada uno: nivel, texto, ancla, línea, separados por tabuladores |
| `PlainText`, `SelText` | el texto como se muestra (sin las marcas), y la parte seleccionada |
| `SelectAll`, `Clear`, `LinkCount`, `EmptyText` | como dicen sus nombres |

La lista completa está en la [referencia](../reference/members.md#rmarkdownview).

**Qué se muestra.** Markdown CommonMark, con las tablas de GitHub y
`~~strikethrough~~`: encabezados, párrafos, **negrita**, *cursiva*, código en
línea, enlaces, listas con viñetas y numeradas (anidadas), citas en bloque,
bloques de código (coloreados según su lenguaje: mira más abajo), tablas (con las
columnas alineadas como dice la fila `:---:`) y líneas horizontales. El HTML
dentro del texto se omite, y una imagen muestra su descripción en cursiva en
lugar de la imagen.

### Bloques de código en color

Un bloque de código se colorea como el editor de código de RapidR colorea su
lenguaje, con el tema del programa: las palabras clave, los nombres, las cadenas
de texto, los números y los comentarios, cada uno con su propio color, que sigue
al tema cuando este cambia. El lenguaje es la palabra que sigue a las ```` ``` ````
de apertura:

| Delimitador | Se colorea como |
|---|---|
| ```` ```sql ```` | SQL: las palabras de todos los dialectos |
| ```` ```sqlite ````, ```` ```mysql ```` (o `mariadb`) | SQL con las palabras de ese dialecto (`PRAGMA`, `AUTO_INCREMENT` …) |
| ```` ```basic ```` (o `rapidr`, `rr`, `bas`, `vb` …) | el BASIC de RapidR |
| ```` ```json ````, ```` ```html ````, ```` ```css ````, ```` ```javascript ```` (`js`), ```` ```toml ````, ```` ```csv ````, ```` ```rust ````, ```` ```markdown ```` | ese lenguaje |
| cualquier otra palabra, o ninguna | texto simple, con la fuente de código |

Así, un programa que muestra el SQL que ejecutó — el historial de una
herramienta de bases de datos, las consultas de una IA — lo muestra como lo haría
el editor:

![Un RMarkdownView: una consulta SQL coloreada, su respuesta como tabla, el BASIC que la ejecutó](../images/markdown/sql-code.png)

![Lo mismo con el tema oscuro: los colores lo siguen](../images/markdown/sql-code-dark.png)

El ejemplo es
[examples/gui/markdown_code.rr](../../../examples/gui/markdown_code.rr) (el botón
**Theme** cambia el aspecto). Funciona igual en el escritorio y en la web.

### Conteos de añadidos y quitados: `<ins>` y `<del>`

Los conteos de una comparación de diferencias — **+11** **−0** — pueden
mostrarse con los colores de la comparación: `<ins>+11</ins> <del>−0</del>`. El
texto entre `<ins>` es añadido (el verde del tema), el texto entre `<del>` es
quitado (su rojo; no tachado: los signos también lo dicen), ambos con la fuente de
código para que los dígitos queden alineados, y cada uno se hace legible (4.5:1,
7:1 en contraste alto) sobre el fondo de la vista en cada aspecto. Un `title` es
lo que dice un lector de pantalla en lugar del signo:
`<ins title="11 lines added">+11</ins>`. Cualquier otro HTML se omite.

### Código en línea y sus palabras: `CodeLanguage`

`` `Inline code` `` es monoespaciado sobre un fondo con tinte y por lo demás
simple. Establece **`CodeLanguage = "rapidr"`** y también se colorea como el
editor colorea el código de RapidR — `PRINT`, `RListBox`, `Items.AddItems` — y
las palabras del código de RapidR en la vista, en línea o en un bloque de código,
responden al mouse: donde el programa tiene un servicio de lenguaje (el propio
chat del asistente de RapidR Studio tiene uno), el puntero sobre una palabra que
conoce es una mano, dejarlo sobre ella muestra su ayuda de una línea como
sugerencia, y un clic dispara **`OnCodeClick(Word, OfType)`**: la palabra como
está escrita y, para un miembro después de un punto, el tipo de componente al que
pertenece (`"RListBox"`; `""` para cualquier otra palabra). El chat de Studio
abre allí la palabra en el panel Ayuda. Una vista sin `CodeLanguage`, o un
programa sin servicio de lenguaje, se comporta como antes.

El componente `RMarkdownView` es propio de RapidR.
