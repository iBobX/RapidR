<!-- translated-from: differences.md 1451b7e09d8b1eae -->
# Diferencias con RapidQ, y extensiones

<!-- legal -->
RapidR es una implementación original, escrita desde cero en Rust puro, y es
**compatible** con RapidQ: no es RapidQ ni una copia de él. La regla que sigue:
lo que hacen los programas escritos para RapidQ lo siguen haciendo; las
extensiones de RapidR son aditivas y nunca cambian lo que hace un programa
existente.

## Nombres

Los componentes de RapidR tienen sus propios nombres —`RForm`, `RButton`,
`RStringGrid` …— que usan RapidR Studio, los ejemplos y este manual.
**Los nombres de RapidQ (`QFORM`, `QBUTTON` …) se aceptan en todas partes, en
silencio, siempre**: un programa antiguo se ejecuta sin cambios, y los dos
nombres se pueden mezclar en un mismo programa. Cada nombre de RapidQ es su
nombre R (`QBUTTON` = `RButton`), salvo `QGAUGE` = `RProgressBar`,
`QOUTLINE` = `RTreeView` y `COMPORT` = `RComPort`
([la lista completa](../reference/classic-names.md)). Los componentes propios de
RapidR (`RPlot`, `RSQLite`, `RJson` …) tienen solo un nombre R. Las otras
extensiones de archivo de código fuente de RapidQ, `.rqw`, `.rqb` y `.rq`, se
leen como `.bas`. Para pasar un programa a los nombres de RapidR,
[impórtalo](importing-rapidq.md): una copia convertida, que se comprobó que
compila exactamente igual que el original, con un informe.

## Cómo se mantiene la compatibilidad

- **Comportamiento de referencia.** Cuando el manual deja una pregunta abierta,
  un programa de prueba la resuelve y RapidR sigue el resultado registrado. Eso
  resolvió el formato de números de la instrucción `PRINT`, su coma, las
  asignaciones a enteros, el redondeo, los operadores de bits, la división por
  cero, los casos límite de las funciones de cadenas de texto, los colores de los
  componentes y más
  ([docs/rapidq-ground-truth.md](../../rapidq-ground-truth.md)).
- **El conjunto de pruebas de conformidad** (más de 250 programas, cada uno con
  su salida esperada) se ejecuta en los tres entornos de ejecución: nativo,
  interpretado y en el navegador.
- **El corpus de ejemplos** (428 programas de ejemplo anteriores): RapidR
  compila 358 de ellos; la mayoría del resto llama directamente a DLL de Windows
  o usa OLE (más abajo). En Windows se ejecuta cada uno, interpretado y como
  compilación nativa, y sus ventanas se comparan con los resultados de
  referencia del conjunto de pruebas, y el estado de cada uno se enumera
  programa por programa en
  [rapidq-corpus-status.md](../../rapidq-corpus-status.md). Los programas que
  usan solo la API que compartían las versiones anteriores de Windows y de Linux
  también se ejecutan en macOS, Linux y la web: 114 de ellos se ejecutaron sin
  ningún problema en cada uno de macOS (interpretado y nativo) y Linux, y la web
  coincide con el escritorio
  ([rapidq-corpus-status-unix.md](../../rapidq-corpus-status-unix.md)).
- **La interfaz gráfica** se comprueba controlando las ventanas de cada programa
  de prueba y comparando sus píxeles y sus árboles de accesibilidad: nativo
  contra interpretado, el escritorio contra el navegador, a escala 1× y 2×.

## Lo que RapidR no hace

- **Llamadas a la API de Windows fuera de Windows.** Un programa que llama a las
  DLL de Windows (`DECLARE … LIB "user32"`, kernel32, gdi32, …, y `SENDMESSAGE`,
  `POSTMESSAGE`, `KILLMESSAGE` de RapidQ) hace esas llamadas **cuando se ejecuta
  en Windows**, de forma nativa e interpretada (mira la guía del lenguaje). En
  macOS, Linux y la web se compila y se ejecuta hasta la primera llamada de ese
  tipo, que lo detiene con un error que nombra la función y, cuando la hay, el
  equivalente portátil de RapidR (`GetTickCount` → `TIMER`). RapidR no emula
  Windows. Unas pocas llamadas de Windows que los programas de RapidQ hacen con
  frecuencia siguen funcionando en todas las plataformas porque tienen un
  significado claro: `Shell_NotifyIcon` (la bandeja del sistema), el registro
  (RRegistry), `joyGetPosEx` nombra RDXJoystick.
- **Lo que un programa de Windows de 64 bits no puede hacer.** Los programas de
  RapidR son de 64 bits: una DLL de 32 bits que se distribuye con un programa
  antiguo no se puede cargar (el error lo dice; hace falta una compilación de
  64 bits de la DLL), el código máquina x86 que un programa escribe y ejecuta
  mediante `CallWindowProc` no se puede ejecutar, y un `TYPE` cuyos campos
  guardan punteros o identificadores (handles) como `LONG` no coincide con la
  disposición de 64 bits que Windows espera para esa estructura. Las `SUB` que se
  pasan a una DLL como funciones de devolución de llamada (callbacks) (`CODEPTR`
  para un procedimiento de ventana, una enumeración) todavía no se admiten. El
  `Handle` de un control es un número propio de RapidR (RapidR dibuja sus
  controles), así que los mensajes de Windows que se le envían no llegan a
  ninguna parte; el `Handle` de un formulario es su ventana real. En una pantalla
  de alta densidad de píxeles (high-DPI), las coordenadas que las funciones de
  Windows toman y dan son los píxeles de la pantalla, mientras que RapidQ (que no
  tiene en cuenta el DPI) obtenía coordenadas escaladas.
- **OLE / COM** (`QOLECONTAINER`, `QOLEOBJECT`): los programas que los declaran se
  compilan; sus métodos no hacen nada e imprimen una advertencia.
- Los `QD3DANIMATION` / `QD3DANIMATIONSET` no documentados de RapidQ (solo en su
  lista de palabras clave): los programas que los declaran se compilan y
  `QDXSCREEN.CreateAnimation` / `CreateAnimationSet` los aceptan, pero, como en
  RapidQ, no hay nada que hacer con ellos (RapidQ no les da claves (keys) ni nada
  que los reproduzca).
- La entrada y salida de puertos de la época de DOS.
- `ON ERROR` se acepta y se ignora (es de VB, no de RapidQ): un error de
  ejecución sigue terminando el programa.

## Diferencias deliberadas

Donde el comportamiento de RapidQ era un fallo o dependía de Windows, RapidR hace
en su lugar algo definido, y lo dice:

- Los valores hostiles nunca hacen fallar el entorno de ejecución: en su lugar
  hay un error de ejecución con su línea (“Division by zero” donde RapidQ lanzaba
  una excepción).
- Las funciones de memoria (`VARPTR`, `MEMCPY`, `MEMSET`, `MEMCMP`) son seguras
  para la memoria: las direcciones son vistas de los propios datos del programa;
  una dirección no válida es un error de ejecución.
- Un RDXJoystick sin ningún joystick lee “no conectado” en lugar de lanzar el
  error de índice de lista de RapidQ.
- `TextRect(Rect, x, y, S$, fc, bc)` dibuja: el texto recortado a `Rect`, que un
  fondo `bc` rellena primero (como hace la llamada de Windows que hay debajo). El
  propio TextRect de RapidQ detiene el programa en cada objeto probado (una
  infracción de acceso en un QBITMAP, un error de índice de lista en un QCANVAS).
- Las ventanas de todos los sistemas las dibuja el propio RapidR, con el aspecto
  propio de RapidR de forma predeterminada (el aspecto exacto de Windows de
  RapidQ con `$THEME Classic`; los tamaños y las posiciones son los de RapidQ en
  ambos), con menús, cuadros de diálogo y portapapeles reales del sistema donde
  los usuarios los esperan.
- Los documentos impresos con `Printer` / `LPRINT` son PDF que se envían a la
  impresora del sistema (mira [Componentes](components.md#the-global-objects)).
- **Una cadena de texto con un carácter por encima de 255 es texto, no bytes.**
  Las cadenas de RapidQ son bytes, y las de RapidR también lo son siempre que
  todos los caracteres sean de hasta 255: `WriteStr(CHR$(200), 1)` escribe el
  byte 200 y un archivo binario copiado con `ReadStr` / `WriteStr` es exacto.
  RapidQ no tenía ningún carácter por encima de 255; RapidR sí (`"中文"`,
  `"😀"`), y un flujo escribe una cadena así entera como UTF-8, donde un conteo de
  bytes la habría cortado por la mitad de un carácter. `WriteStr(S, n)` cuenta
  caracteres, así que `WriteStr(S, LEN(S))` escribe todo `S`. Los cuadros de
  texto y el editor de código nunca guardan un `?` en lugar de un carácter que
  falta en la codificación de su archivo (guardan UTF-8 en su lugar), y
  `WriteText` / `ReadAllText` existen a propósito para texto
  (mira [Texto y bytes](language.md#text-and-bytes-accents-chinese-emoji)).
- **El cuadro de diálogo de fuente nombra un color del sistema.** Una fuente que
  nunca estableció un color (un `RFontDialog` nuevo, las fuentes de la mayoría de
  los componentes) tiene `clWindowText`. El cuadro de diálogo de RapidQ lo
  muestra como “Custom”; el de RapidR muestra “Black” (`clWindow`, “White”), y
  presionar Aceptar sin elegir otro color conserva `clWindowText`, así que el
  texto sigue al tema:

  ```basic
  FontDlg.GetFont(Label1.Font)     ' la lista muestra Black
  IF FontDlg.Execute THEN FontDlg.SetFont(Label1.Font)
  ```

## Extensiones

Todo lo de aquí es propio de RapidR; un programa que lo usa ya no se compila con
el compilador de RapidQ.

**Lenguaje**

- La compilación condicional de VB `#If` / `#ElseIf` / `#Else` / `#End If`,
  `#Const`.
- `i++`, `i--`, `+=` y compañía, `CBOOL`, `Public` / `Private` / `Global` de VB
  (se aceptan).
- `$THEME`; `$APPTYPE WEB`; los scripts `#!/usr/bin/env rapidr`.
- Los bloques de Rust `RUSTSTART … RUSTEND` en las compilaciones nativas.

**Componentes y miembros**

- Componentes solo de RapidR: `RSQLITE`, `RJSON`, `RHTTP`, `RSERVERSOCKET`, `RHTTPSERVER` ([Redes](network.md)),
  `RNUM`, `RDATAFRAME`, `RPLOT` y los componentes de datos vinculados `RCSV`,
  `RDATASOURCE`, `RDBGRID` ([Ciencia de datos](data-science.md)), `RAI`
  ([El asistente de IA y RAI](ai-assistant.md)), `RMARKDOWNVIEW`,
  `RCODEEDITOR`, `RDESIGNSURFACE`, `RMEMO`, `RUPDOWN`, `RDATETIMEPICKER`,
  `RTOOLBAR`, `RSECRETS` ([Guardar secretos](secrets.md)), los componentes web
  ([referencia](../reference/components.md)).
- En componentes existentes: `DataSource` y `DataField` en `REdit` y `RLabel`;
  `Paragraph` y `TextRTF` en `RRichEdit` ([Texto enriquecido](rich-text.md));
  `Form.OnDropFiles`.
- En todos los componentes: `Anchors`, restricciones de tamaño, `AccessibleName` /
  `AccessibleDescription`, `Scale` / `OnScaleChanged`; `Application.Theme`
  ([Componentes](components.md#rapidrs-additions-to-every-component)).
- Enlace de parámetros de SQL (`Query sql, values…`, `AddParam`): sin inyección de
  SQL.
- Imágenes SVG dondequiera que RapidQ toma un mapa de bits; dibujadas con
  `currentColor` o con los tokens de color de RapidR, toman los colores del tema,
  y `RImageList.AddSVGFile` las añade al tamaño de la lista
  ([Tus propios íconos SVG](svg-icons.md)). El nombre de un `$RESOURCE` va
  dondequiera que va un archivo de imagen (`RImage.Load RES`).
- Conexiones cifradas a MySQL: `SSLMode`, `SSLCA` y `Encrypted` de RMySQL
  ([Bases de datos](databases.md#encrypted-connections)).
- Los ejes, botones y eventos de RDXJoystick; QMIDI sin un sintetizador del
  sistema.

**Entornos de ejecución y herramientas**

- Tres entornos de ejecución a partir de un mismo código fuente: ejecutables
  nativos, el intérprete y el navegador.
<!-- legal -->
- RapidR Studio: un diseñador de formularios, un editor de código con
  autocompletado e información flotante, un depurador, datos vinculados en vivo
  en el diseñador, un asistente de IA (desactivado hasta que lo actives), en el
  escritorio y en el navegador.
- Dibujo de alta densidad, accesibilidad para lectores de pantalla y temas para
  todos los programas, incluidos los antiguos.
- Los programas se dibujan con el aspecto propio de RapidR de forma
  predeterminada (el de RapidR Studio: claro, oscuro o de contraste alto según
  esté configurado el sistema, con Inter como fuente predeterminada);
  `$THEME Classic` dibuja el aspecto exacto de Windows de RapidQ. Los tamaños, las
  posiciones y los colores que un programa lee son los de RapidQ en todos los
  aspectos ([Temas](components.md#themes)).
- El RapidR Runtime con tipos de archivo; ejecutables independientes que no
  necesitan Rust.
<!-- legal -->
- Cada compilación lleva consigo sus avisos de terceros.

## Planeado

La hoja de ruta ([ROADMAP.md](../../../ROADMAP.md)) enumera lo que viene a
continuación, entre otras cosas: el manual, Studio y los ejemplos en español,
portugués y chino además de inglés; `$THEME native` (el aspecto de cada
sistema); RapidR ejecutándose en un ESP32; modelos de IA locales y un componente
de chat listo para usar; más fuentes de datos (Excel, Parquet, PostgreSQL …);
`$UNSAFEMEM` y llamar a bibliotecas nativas en macOS y Linux como RapidQ llama a
las DLL; dispositivos móviles. Ninguna de estas cosas está en la 2.119.0.
