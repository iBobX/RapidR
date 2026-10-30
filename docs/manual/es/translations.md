<!-- translated-from: translations.md 110aa7e679c71d7f -->
# Tu programa en todos los idiomas

Un programa de RapidR puede mostrar su texto en inglés, español, portugués,
chino o cualquier otro idioma. Escribes el programa en un idioma (normalmente
inglés), envuelves su texto en `TR$` y pones las traducciones en pequeños
archivos de texto junto al programa. Nada más cambia: un programa sin archivos
de traducción muestra su propio texto, exactamente como antes.

## Un ejemplo completo

`hello.rr`:

```basic
' Ejecútalo en español:  rapidr run hello.rr es
IF COMMAND$(1) <> "" THEN Application.Language = COMMAND$(1)

PRINT "Idioma: "; Application.Language
PRINT TR$("Save file")
PRINT TR$("Open", "menu")
PRINT TR$("Hello, {name}!", "name", "Ana")
PRINT TRN$("{n} file", "{n} files", 1)
PRINT TRN$("{n} file", "{n} files", 5)
```

`lang/es.po`, en una carpeta llamada `lang` junto al programa:

```po
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

msgid "Save file"
msgstr "Guardar archivo"

msgctxt "menu"
msgid "Open"
msgstr "Abrir"

msgid "Hello, {name}!"
msgstr "¡Hola, {name}!"

msgid "{n} file"
msgid_plural "{n} files"
msgstr[0] "{n} archivo"
msgstr[1] "{n} archivos"
```

`rapidr run hello.rr es` imprime:

```text
Idioma: es
Guardar archivo
Abrir
¡Hola, Ana!
1 archivo
5 archivos
```

`rapidr run hello.rr pt` (sin `pt.po`) imprime el propio texto en inglés del
programa. `rapidr run hello.rr es-AR` imprime el español: el español de
Argentina busca primero en `es-AR.po` y luego en `es.po`.

El mismo programa está en
[examples/translations](../../../examples/translations/), con archivos en
español, portugués y chino; su fila del lienzo dibuja el saludo con `TextOut`,
así que el chino está en una etiqueta y en un lienzo:

![El ejemplo de traducciones en chino](../images/translations/languages-chinese.png)

## Texto en el código

| Escribes | Qué da |
|---|---|
| `TR$("Save file")` | la traducción de “Save file”, o el propio “Save file” |
| `TR$("Open", "menu")` | la traducción de “Open” *como elemento de menú*: un **contexto** distingue dos significados de la misma palabra (el `msgctxt` del archivo PO) |
| `TR$("Hello, {name}!", "name", user$)` | marcadores de posición por nombre: `{name}` se convierte en el valor. Quien traduce puede moverlo a cualquier parte de la oración |
| `TR$("Open {file}", "menu", "file", f$)` | un contexto y marcadores de posición juntos |
| `TRN$("{n} file", "{n} files", count)` | la forma correcta para `count` en el idioma (una forma en chino, dos en español y portugués, tres en polaco); `{n}` es la cantidad |

Cómo distingue `TR$` un contexto de los marcadores de posición: después del
texto, una cantidad **impar** de argumentos empieza con el contexto; una
cantidad par son solo marcadores de posición (un nombre y luego su valor).
`TRN$` los toma de la misma manera después de la cantidad.

`{{` y `}}` dan las llaves mismas (`TR$("Use {{ and }}")` muestra
`Use { and }`). Un `{name}` que nadie dio se queda como está escrito.

## El idioma

- `Application.Language` es el idioma del sistema hasta que el programa lo
  establece: en macOS y Windows, la configuración del sistema; en Linux, la
  variable `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` o `LANG`; en un navegador, el
  idioma del navegador.
- `Application.Language = "pt-BR"` lo cambia: el siguiente `TR$` habla portugués
  de Brasil. Cualquier escritura sirve (`pt_br` se lee de vuelta como `pt-BR`);
  `""` vuelve al idioma del sistema.
- Respaldo: `es-AR` → `lang/es-AR.po` → `lang/es.po` → el propio texto del
  programa. El chino nombrado solo por su región también toma el archivo de su
  escritura (`zh-CN` lee `zh-Hans.po`, `zh-TW` lee `zh-Hant.po`).
- Cuando el idioma cambia, los textos, las sugerencias y los nombres accesibles
  que son el texto completo de un catálogo —`Caption = TR$("Save file")`— se
  traducen otra vez por sí solos, en la siguiente vuelta del ciclo de eventos.
- `Application.OnLanguageChange` se dispara justo después. El texto que hiciste
  con valores rellenados (`TR$("Line {n}", "n", 5)`), los elementos de lista y
  las pestañas son cadenas de texto simples: vuelve a hacerlos ahí.

```basic
SUB LanguageChanged
    ' (Caption = TR$("Save file") cambió por sí solo; este tiene un valor)
    CountLabel.Caption = TRN$("{n} file", "{n} files", FileCount)
END SUB
Application.OnLanguageChange = LanguageChanged
```

- Una lista para que las personas elijan su idioma: `Application.Languages`
  enumera los idiomas que tienen tus catálogos, una etiqueta de idioma por línea
  (`es`, `pt-BR`), y `Application.LanguageName(tag)` da el nombre propio de cada
  uno (`Español`, `Português (Brasil)`, `中文（简体）`):

```basic
DIM all AS STRING, i AS INTEGER, tag AS STRING
all = Application.Languages
FOR i = 1 TO TALLY(all, CHR$(10)) + 1
    tag = FIELD$(all, CHR$(10), i)
    IF tag <> "" THEN LanguageBox.AddItems Application.LanguageName(tag)
NEXT
```

## Los archivos de traducción

- Un archivo por idioma, con el nombre del idioma: `lang/es.po`,
  `lang/pt-BR.po`, `lang/zh-Hans.po` (`es_AR.po` con un guion bajo también
  sirve).
- Dónde: en una carpeta `lang` junto al archivo del programa que ejecutas
  (`rapidr run hello.rr`, Ejecutar de RapidR Studio), junto a un ejecutable
  compilado, o junto a los archivos de una página web.
  `Application.LanguageFolder = "locale"` los lee en su lugar de otra carpeta
  junto al programa (en la web, junto a la página, con su propio `index.txt`).
- Una compilación los lleva consigo: `rapidr build` (nativo o `--interp`), las
  compilaciones web y Compilar de RapidR Studio copian los archivos `.po` de la
  carpeta `lang` a lo que crean — `Contents/Resources/lang` en una aplicación de
  macOS, `lang/` junto a un `.exe` o en `usr/bin` de un AppDir de Linux, `lang/`
  junto a una página web. Una aplicación compilada habla todos los idiomas que
  tradujiste, dondequiera que se inicie
  ([Compilar aplicaciones](building-apps.md#the-programs-data-files-go-with-it)).
- Siguen siendo archivos simples en lo que distribuyes, así que el usuario puede
  añadir un idioma sin volver a compilar: pon `lang/fr.po` junto a los demás
  archivos de la aplicación. En la web, añade también su nombre a
  `lang/index.txt`: una página pide solo los archivos que están listados allí,
  así que un idioma para el que no tiene archivo no cuesta ninguna solicitud.
- El formato es PO, el formato de texto simple que las herramientas de
  traducción y quienes traducen ya conocen. RapidR lo lee con su propio lector
  pequeño:
  - `msgid` es el texto del programa, `msgstr` su traducción; `msgctxt`, un
    contexto; `msgid_plural` y `msgstr[0]`, `msgstr[1]` …, las formas del plural.
  - Un texto largo puede ir en varias líneas de `"…"`, que se unen; `\n` es un
    salto de línea, `\t` un tabulador, `\"` una comilla, `\\` una barra invertida.
  - Las líneas con `#` son comentarios. Las entradas marcadas con `#, fuzzy` (una
    traducción que todavía necesita revisión) no se usan, como hacen otras
    herramientas.
  - La primera entrada (`msgid ""`) es el encabezado. Su línea `Plural-Forms`
    dice cómo cuenta el idioma:

    | Idioma | Plural-Forms |
    |---|---|
    | Español, inglés, alemán, portugués de Portugal | `nplurals=2; plural=(n != 1);` |
    | Francés, portugués de Brasil | `nplurals=2; plural=(n > 1);` |
    | Chino, japonés | `nplurals=1; plural=0;` |
    | Polaco | `nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 \|\| n%100>=20) ? 1 : 2);` |

    Sin una, se usa la regla del inglés.
- Una carpeta, un archivo o una entrada que falta nunca es un error: el programa
  muestra su propio texto. Una línea que RapidR no puede leer se omite; el resto
  del archivo sigue funcionando. Guarda los archivos como UTF-8. Un archivo que
  no es UTF-8 (uno que un programa o un editor más antiguo guardó como
  Windows-1252, una `é` como un solo byte) se lee como Windows-1252, así que
  ninguna letra se convierte en `?` ni en `�`; un `?` que ya está *en* el archivo
  (chino que una herramienta antigua perdió al escribirlo) no puede volver, así
  que escribe el archivo otra vez. Para escribir un catálogo desde un programa,
  usa `WriteText` (o `WriteStr(Text, LEN(Text))`) en un `RFileStream`: mira
  [Texto y bytes](language.md#text-and-bytes-accents-chinese-emoji).
- El chino, el japonés y el coreano se muestran por igual en etiquetas, botones
  y lienzos: el escritorio usa la fuente CJK del sistema y la web las fuentes
  Noto que vienen con la compilación; no hay nada que instalar.
- Un programa de consola imprime las traducciones tal como son cuando su archivo
  es un archivo `.rr` (o cualquier archivo que tenga una letra no inglesa). Un
  archivo `.bas` de letras inglesas simples se toma como un programa clásico,
  cuya consola muestra la página de códigos antigua: `¡` y `ó` se mostrarían allí
  como otros signos. Las ventanas y sus títulos no se ven afectados.

## Herramientas para quienes traducen

`rapidr i18n` crea y comprueba los archivos (el lector y el escritor propios de
RapidR, nada que instalar):

```
rapidr i18n extract main.rr forms.rr -o lang/messages.pot   cada texto de TR$ y TRN$: la plantilla
rapidr i18n update lang/messages.pot lang/es.po lang/pt-BR.po   el archivo de cada idioma al día con ella
rapidr i18n check lang/es.po                                 cada traducción conserva su {name}, %1, `code`, & y "..."
rapidr i18n pseudo lang/messages.pot -o lang/xx.po           el seudoidioma: cada texto con acentos, un tercio más largo
```

- **El seudoidioma `xx`.** `Application.Language = "xx"` con `lang/xx.po` muestra
  cada texto que traducen los catálogos como `[Šàvé ƒíĺé·····]`: cualquier inglés
  simple que quede en la pantalla es texto que nunca pasó por `TR$`, y cualquier
  texto cortado es un lugar donde no cabrá un idioma más largo.
- **Palabras clave.** Una rutina que guarda el inglés y lo traduce cuando lo
  muestra (`TR$(Title)`, una variable) nombra su argumento para la plantilla con
  un comentario en tu código fuente: `' i18n-keywords: AddRow:2` (el segundo
  argumento de `AddRow` es un texto que se traduce; `AddRow:2@menu`, en el
  contexto `menu`).
- **Marcado para revisión.** `#, review` en una entrada dice que una persona
  todavía tiene que revisarla (un borrador); mientras tanto se usa. `#, fuzzy` no
  se usa.

RapidR Studio se traduce de la misma manera: [Studio en tu
idioma](studio-languages.md).

## Todavía no

- Formularios que traducen sus propios textos sin código (es lo siguiente).
- Números, fechas y dinero al estilo de cada idioma (`FORMAT$` y las demás
  funciones se quedan exactamente como están).
- Un editor de traducciones en RapidR Studio.
