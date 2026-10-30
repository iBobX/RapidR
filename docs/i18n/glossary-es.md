# Spanish glossary: español neutro for RapidR

**Status: DRAFT 1 (2026-10-29), waiting for Robert's approval. No translation starts until it is approved.**

This is the one list of words every Spanish translation of RapidR follows: RapidR Studio's screens, RAI, the compiler's messages, the manual and the website (master-plan M5). It is written in English so it can be reviewed against the English product; the Spanish is in the middle column of each table.

"Español neutro" here means Latin-American Spanish that a person in Ciudad de México, Bogotá, Lima, Santiago, Buenos Aires or Montevideo reads without noticing an accent. The test for every word: would a 12-year-old in any of those cities stop and wonder? If a word is regional, take the plainer one, or the one most countries share.

How to read the tables:

- **Q1 … Q10** in a note point to an open question in [section 4](#4-open-questions-for-robert). The Spanish shown is my recommendation; if Robert picks the other option, only those rows change.
- **⚑** marks a word I am less sure of than the rest and where a native check is most welcome. Robert can overrule any row when he approves.
- Menu and button words are written the way they appear in the menu (sentence case, see [1.4](#14-capitalisation)).
- A string with `&`, `<` or `>` is in code font so the table shows it as written.

Where the words come from: `ide/commands.inc` (every menu and command title), the captions in `ide/*.inc`, the manual (`README.md`, `getting-started.md`, `ai-apps.md`, `versions.md`, `debugging.md`, `translations.md`), `docs/versions-plan.md` (the Simple words), `docs/design-guidelines.md`, `docs/ai-ux-plan.md`, `docs/learn-with-rai-plan.md`, `docs/local-ai-plan.md` and `crates/rapidr-lang/data/components/*.toml` (the Toolbox group names, the property categories). Section 5 lists them.

---

> **Decisions (Robert, 2026-10-29):** Q1 build = **compilar** · Q2 app = **aplicación** · Q3 array = **arreglo** · Q4 event handler = **controlador** de eventos · Q5 Watch / Immediate / Outline = **Observar / Inmediato / Estructura** · Q8 add = **añadir** · Q9 library = **biblioteca** · Q6 step commands short: **Entrar en la llamada / Pasar por encima / Salir de la llamada** · Q7 Pro Git words: **English Git commands with Spanish nouns** · Q10 manual code samples: **comments and shown text translated**, code words never. Every row below follows these; where a row still says *agregar*, *app*, *crear* or *manejador*, the decision wins.

## 1. Style guide

### 1.1 Voice

- **Tú, always.** One person, spoken to directly. Never *vos* (elegí, hacé, podés, andá), never *vosotros* (elegid, pulsad), never *usted* (elija, puede). A plural *you* is rare in a UI; when a teacher's note really needs one, rephrase ("Cada persona…") or, as the last resort, use *ustedes*, which is the plural of tú in Latin America (⚑ Robert to confirm).
- **Buttons, menu items, check boxes, dialog titles and chips use the infinitive**: Guardar, Abrir, Permitir que las apps de IA usen RapidR Studio. Never the imperative on a control ("Guarda").
- **Sentences that tell the person what to do use the tú imperative**: "Elige un modelo", "Haz clic en Guardar", "Presiona F5", "Escribe lo que buscas". Learn the irregular ones: *haz, ten, pon, ve, sal, sé, di, ven*. ("Ten en cuenta" for "Keep in mind", "Ve a Herramientas > Configuración".) With a pronoun, the accent follows the stress: "Inténtalo de nuevo", "Cámbialo".
- **Questions to the person**: "¿Quieres guardar los cambios?" (never "¿Desea…?").
- **Who speaks.** Studio speaks about itself by its name, in the third person, without an article: "Studio guardó una copia". RAI speaks in the first person with tú: "No puedo ejecutar el programa solo con el código". The English has no "please", no "thanks", no exclamation marks and no emoji; Spanish has none either (the only "¡" is a greeting: "¡Hola, mundo!").
- **Friendly and short, like the English.** One idea per sentence. Say what happens, not how it is built. Prefer a plain verb to a noun phrase ("No se pudo guardar" over "Se produjo un error de guardado"). Avoid *debe/deberá*; say what the person can do. Gerunds are fine for progress ("Probando la clave…"), not for instructions.
- **No gender in the person.** *Tú* avoids it, so keep it that way: "Te damos la bienvenida" (not "Bienvenido/a"), "¿Quieres continuar?" (not "¿Estás seguro/a?"). No "@", no "x", no "e" endings, no "los/las" pairs. The Welcome page's title is the noun: "Bienvenida".
- **Plain words, as the design guide asks** (`docs/design-guidelines.md`, "Words"): no jargon where a word will do; no other product's name; a third party is named only where the person must recognise it (an AI provider, GitHub).

### 1.2 Neutral words: say this, not that

| Say | Not | Why |
|---|---|---|
| archivo | fichero | *fichero* is Spain's |
| computadora | ordenador, computador | *ordenador* is Spain's; *computador* is Chile's and Colombia's |
| hacer clic, doble clic, clic derecho | pulsar, cliquear, dar clic | RAE writes *clic* (plural *clics*); *hacer clic en* |
| presionar (a key, a physical button) | pulsar, oprimir | *pulsar* is Spain's; *oprimir* is stiff |
| mouse | ratón | *mouse* is what Mexico to Argentina say |
| ícono | icono | Robert's rule (both are correct); plural *íconos* |
| carpeta | directorio | *directorio* never in a screen |
| descargar | bajar | |
| agregar | añadir | Spain-leaning, understood everywhere; Q8 |
| configuración | ajustes, preferencias | *ajustes* feels Spain's; *preferencias* is a macOS word |
| enlace | vínculo, link | |
| contraseña (password) | clave | *clave* is reserved for the **API key** (clave de API); never use it for a password |
| correo electrónico, correo | mail, email | |
| navegador | browser | |
| captura de pantalla | pantallazo | |
| en línea | online | |
| ejecutar | correr | |
| guardar | grabar | *grabar* is for sound and video |
| centavo | céntimo | *céntimo* is Spain's |
| sitio web | web (as a noun) | |
| internet (lowercase) | Internet | RAE 2010 |
| ahora, enseguida | ahorita, ya mismo, vale, okey, chévere | no slang, no regional interjections, no "ok" |
| presiona, elige, haz | pulsa, escoge, coge | |

### 1.3 Words with two jobs: one Spanish word each

The English uses one word for several things. These choices are final unless Robert changes them; use `msgctxt` (see `docs/manual/translations.md`) when the same English word needs two Spanish ones.

| English word | Spanish | When |
|---|---|---|
| Delete / Remove / Clear | Eliminar / Quitar / Borrar | **Eliminar** takes something away for good (a chat, a file). **Quitar** takes it out of a list and keeps it (Remove an include folder). **Borrar** empties a list or a log (Clear Recent, Clear Activity). |
| Turn on / Enable | Activar / Habilitar | **Activar** (and *Desactivar*) is a feature or switch (AI apps). **Habilitado / deshabilitado** is a control that is or isn't greyed out. |
| Accept / OK | Aceptar | Same word on purpose: the OK button is *Aceptar*, and so is "Accept" a change. If one surface ever has both, the change's button reads *Conservar*. |
| Update / Refresh | Actualizar | The ↻ button and a software update both read *Actualizar*. |
| Window / Form | Ventana / Formulario | **Ventana**: any window, the Window menu, the program's running window. **Formulario**: an `RForm` as the designer and the code treat it. |
| Layout | Disposición | Where panes sit (Reset Layout) and a property group. **Diseño** is only the Design view and the designer. |
| Key | Clave / Tecla | **Clave de API** is the secret. **Tecla** is a keyboard key. |
| Back | Atrás / Volver | `< Back` in a wizard is *Atrás*. Returning from the Chats list, or to an older version, is *Volver*. |
| Edit | Edición / Editar | The menu is *Edición*; the verb (Edit Breakpoint) is *Editar*. |
| Format | Formato / Dar formato | The designer's menu is *Formato*; Format Document is *Dar formato al documento* (*formatear* is what you do to a disk). |
| Tag | Marca / Etiqueta | In Simple, "Mark this version" makes a **marca**. In Pro it is an **etiqueta (tag)**. *Tag* the property is code. |
| Set | Establecer | Set Value, Set Next Statement. |
| Look Again | Buscar de nuevo | The button that scans for local AI servers. |
| Case, Word (search toggles) | Mayúsculas y minúsculas, Palabra completa | Regex stays **Regex**. |
| Cursor / Pointer | Cursor / Puntero | **Cursor** is the text caret. **Puntero** is the mouse pointer (the AI's virtual pointer is a *puntero virtual*). |

### 1.4 Capitalisation

**Sentence case everywhere**: menus, commands, buttons, dialog titles, pane and tab names, column headers, chips, list items, headings. Capitalise the first word and proper nouns only.

- "Guardar como...", "Permitir en esta sesión", "Apps de IA", "Puntos de interrupción", "Opciones del proyecto", "Línea de tiempo".
- This is the natural form in Spanish software, and it **replaces the English Title Case rule** of the design guide on purpose: Spanish does not capitalise every word. The other rules carry over (verb first; `...` when more is asked; a field's label ends with `:`; one primary action).
- Stay capitalised: RapidR, RapidR Studio, Studio, RAI, GitHub, Git, Anthropic, Claude, OpenAI, DeepSeek, and acronyms (IA, API, MCP, CSV, SQL).
- Lowercase: months, weekdays, languages (español, inglés), "internet", "web".

### 1.5 Punctuation

- **Inverted marks are mandatory**: ¿Quieres guardar?, ¡Hola, mundo!. The existing manual example `msgstr "Hola, {name}!"` lacks the opening ¡; the sample in `examples/translations/lang/es.po` has it right.
- **Ellipsis.** The English source writes three ASCII dots (`Save As...`), and Studio's code strips exactly those: `PlainTitle` in `ide/commands.inc` does `RIGHT$(t, 3) = "..."` to make tool-bar tooltips and command-palette entries. So in a menu or button string, **copy the dots exactly as the English line has them** ("Guardar como..."). In prose (manual, long notes) use the single character "…". The meaning is unchanged: the command asks for more before it acts.
- **Quotation marks**: typographic double “ ” for names and quoted words; ‘ ’ inside them. Not « » (rare in Latin-American software). Straight quotes stay inside code and where the program needs them.
- **Menu paths** keep the English shape: `Herramientas > Configuración`, with the translated menu and item names, bold in the manual.
- **Colon** ends a field label, no space before it: "Nombre:". A label that is a sentence has no final period; a sentence does.
- **Dashes.** A single raya works like a colon and is spaced on both sides, as in the English ("… — nunca en el proyecto"). A pair of rayas is attached to the words it encloses (—así—).
- **Quoted UI names in prose** are written exactly as the translated UI shows them, in bold: **Guardar como...** in the manual, never a paraphrase.

### 1.6 Numbers, dates, units

- **Do not type separators when a program can.** Counts and amounts come in as placeholders (`{n}`); `TRN$` picks singular for 1 and plural for everything else, **0 included** ("0 archivos"). Give each form a whole sentence so the verb and the adjective agree ("Se guardó 1 archivo" / "Se guardaron {n} archivos").
- In prose: thousands with a non-breaking space (32 000), none for four digits (4096); avoid decimals by rephrasing ("una décima de centavo por pregunta"); if one is unavoidable, use the point, which is what the programs print and what Mexico, Central America and Peru write (⚑ Robert to confirm, since Argentina and Uruguay write a comma).
- **Percent**: "45%", no space, as the English.
- **Money**: write **US$** or **dólares**. A bare **$** is a peso in many countries. The settings caption "$ per million tokens sent / received" becomes "US$ por millón de tokens enviados / recibidos".
- **Million and billion.** *Millón* is 10⁶. English "billion" (10⁹) is *mil millones*; the Spanish *billón* is 10¹². Never translate one as the other.
- **Dates** in prose: "29 de octubre de 2026". Numeric dates are ISO (2026-10-29) in logs; never 10/11/2026, which is two different days in two countries. **Time** is 24-hour (14:30), not a. m./p. m. Relative times: "hace 2 min".
- **Units**: GB, MB, px, s, ms, min, without a plural s; a non-breaking space before the unit in the manual ("16 GB"). The status bar's `Ln 12, Col 5` becomes `Lín 12, Col 5`.

### 1.7 Mnemonics and shortcuts

- **`&` stays in the string; the translator chooses the letter.** Spanish words start with other letters: Save is `&Guardar`. Rules (from the design guide): unique on its menu or surface; every button and field label has one except OK (*Aceptar*) and Cancel (*Cancelar*). Prefer the first letter of the main word if it is free, otherwise a clear consonant; avoid letters with descenders (g, j, p, q, y) and accented letters. `&&` shows a literal ampersand.
- **Shortcuts are physical keys and never move.** Ctrl+F finds (even though *Buscar* starts with B), Ctrl+G goes to a line, Ctrl+H replaces, F5 runs. A translation never changes a shortcut or invents a new one to match a Spanish initial.
- **Key names stay as the English shows them** (`Ctrl`, `Shift`, `Alt`, `Enter`, `Esc`, `Tab`, `Del`, F1–F12, ⌘ ⌥ ⇧): that is what the shortcuts table and many keycaps say. ⚑ Robert: do his keyboards print *Supr* for Del?
- The ten menu-bar mnemonics, all different: `&Archivo`, `&Edición`, `&Ver`, `&Proyecto`, `E&jecutar`, `&Depurar`, `&Formato`, `&Herramientas`, `Ve&ntana`, `Ay&uda`.

### 1.8 Code inside sentences

- **Never translate or inflect code**: keywords (`PRINT`, `DIM`), component names (`RButton`, `RForm`), properties, events, methods, file names, commands, error codes. In prose they are in code font, with **no plural s and no hyphen**: "dos `SUB`", "las `FUNCTION`".
- **Put a Spanish noun in front of the code word** so gender never shows: "el componente `RButton`", "la propiedad `Caption`", "el evento `OnClick`", "el método `Show`", "la instrucción `PRINT`", "la función `LEFT$`", "el tipo `STRING`". When a keyword must carry an article, it takes the gender of the Spanish word it stands for: `SUB` (subrutina) and `FUNCTION` (función) are feminine, `TYPE` (tipo) masculine, `CONST` (constante) feminine.
- **Code samples** in the manual: Q10.

### 1.9 Placeholders and sentences

- `{name}`, `{n}`, `{file}` and the like are never translated or split. They may move within the sentence.
- **Whole sentences only.** Spanish word order, gender and articles differ from English, so a sentence built from pieces in code (`"Deleted " + title + "."`) can't be translated well. Don't translate fragments: report the line, and the plumbing makes it one `TR$` with placeholders.
- **No article before a placeholder** of unknown gender, and no "de/a + el" in front of one (*del*, *al* can't be formed): write "Archivo: {file}" or "Error en {file}", not "El {thing}" or "Guardar el {thing}".

### 1.10 Length

Spanish runs 15 to 30% longer than English. The controls do not grow (buttons are at least 72 or 96 px wide, `docs/design-guidelines.md`, "Controls by context"), so **shorten the Spanish**: keep the verb, drop the adjective ("Preguntar antes de cambiar", not "Preguntar antes de realizar cambios"). If a string still doesn't fit, say so in a translator comment (`#. too long`); don't ask for a wider control.

### 1.11 Texts that carry legal weight

The consent dialogs, the AI cost and privacy notes, the secret-check warnings and the "what is sent" texts were written point by point after a legal review ("Every point the legal review requires stays", design guide). Translate them **by meaning, row by row, dropping nothing**, and send them through the same legal check as any new text before release. Licences, notices and the legal files (`LICENSE`, `NOTICE`, `LEGAL.md`) are **not translated**: the English text is the one that applies; a Spanish summary, if wanted, is a separate page that says so.

### 1.12 The voice in six lines

| English (from the product) | Español neutro |
|---|---|
| Something went wrong in Studio | Algo salió mal en Studio |
| Your work is safe: nothing was lost, and Studio goes on. | Tu trabajo está a salvo: no se perdió nada y Studio sigue funcionando. |
| Let AI apps use RapidR Studio? | ¿Permitir que las apps de IA usen RapidR Studio? |
| Do you want to save the changes you made to {file}? | ¿Quieres guardar los cambios que hiciste en {file}? |
| Type what to find, then Enter. | Escribe lo que buscas y presiona Enter. |
| Select a component on a form to see its properties. | Selecciona un componente de un formulario para ver sus propiedades. |

---

## 2. The terms

### 2.1 The menu bar

| English | Español neutro | Note |
|---|---|---|
| File | Archivo | `&Archivo` |
| Edit | Edición | `&Edición`. The noun; the verb in a command is *Editar* |
| View | Ver | `&Ver`. *Visualización* is longer and no clearer |
| Project | Proyecto | `&Proyecto` |
| Run | Ejecutar | `E&jecutar`. *Correr* is spoken in some countries but reads oddly on a menu |
| Debug | Depurar | `&Depurar` |
| Format | Formato | `&Formato`. The designer's alignment menu |
| Tools | Herramientas | `&Herramientas` |
| Window | Ventana | `Ve&ntana` |
| Help | Ayuda | `Ay&uda` (the A is Archivo's, and y has a descender) |

### 2.2 Common commands

| English | Español neutro | Note |
|---|---|---|
| New Project... | Nuevo proyecto... | *Nuevo* first, like the other menu verbs |
| Open Project or File... | Abrir proyecto o archivo... | |
| Open Recent | Abrir reciente | The Welcome page section is *Recientes* |
| Save / Save As... / Save All | Guardar / Guardar como... / Guardar todo | |
| Close Document / Close Project | Cerrar documento / Cerrar proyecto | A *documento* is an open tab (code or design) |
| Exit | Salir | |
| Undo / Redo | Deshacer / Rehacer | |
| Cut / Copy / Paste | Cortar / Copiar / Pegar | |
| Find... / Replace... / Find in Files... / Go to Line... | Buscar... / Reemplazar... / Buscar en archivos... / Ir a la línea... | Ctrl+F, Ctrl+H and Ctrl+G don't move |
| Settings... | Configuración... | Rejected: *Ajustes* (feels Spain's), *Preferencias* (macOS word), *Opciones* (taken by Project Options) |
| Command Palette... | Paleta de comandos... | |
| Keyboard Shortcuts | Atajos de teclado | *Atajo* is short and neutral |
| Toggle Comment | Alternar comentario | *Alternar* = toggle |
| Format Document | Dar formato al documento | See 1.3 |
| Quick Fix... | Corrección rápida... | |
| Indent Lines / Outdent Lines | Aumentar sangría / Disminuir sangría | |
| Add File / Add Form / Add Module | Añadir archivo / Añadir formulario / Añadir módulo | Q8 (Robert: añadir) |
| Project Options... | Opciones del proyecto... | |
| Theme: Light / Dark / High Contrast / Classic | Tema: Claro / Oscuro / Contraste alto / Clásico | "RapidR (as the System)" is *RapidR (como el sistema)* |
| Reset Layout | Restablecer disposición | |

### 2.3 Buttons and small words

| English | Español neutro | Note |
|---|---|---|
| OK / Cancel | Aceptar / Cancelar | No mnemonic, as in English |
| Apply | Aplicar | Also an AI action |
| `Next >` / `< Back` | `Siguiente >` / `< Atrás` | The wizard. See 1.3 for *Volver* |
| Browse... | Examinar... | Not *Buscar* (that's Find) |
| Add / Remove | Añadir / Quitar | Q8 (Robert: añadir); see 1.3 |
| Delete / Clear | Eliminar / Borrar | See 1.3 |
| Reset / Restore | Restablecer / Restaurar | Reset Token = *Restablecer token* |
| Turn On / Turn Off | Activar / Desactivar | The consent button: **Activar** |
| Learn more | Más información | A link |
| Not Now | Ahora no | |
| Discard / Don't Save | Descartar / No guardar | |
| Send / Stop | Enviar / Detener | |
| Test | Probar | "Test the Key" = *Probar la clave* |
| Preview | Vista previa | "Preview the New Program..." = *Vista previa del programa nuevo...* |
| Default | Predeterminado | One word; *por defecto* is fine aloud |
| Recent / Examples / Start (Welcome page) | Recientes / Ejemplos / Comenzar | |

### 2.4 Parts of the screen

| English | Español neutro | Note |
|---|---|---|
| window | ventana | |
| dialog | cuadro de diálogo | *Diálogo* alone in tight spots |
| pane, panel | panel | One word for both: the English "pane" has no Spanish twin |
| tab | pestaña | |
| tool bar / status bar | barra de herramientas / barra de estado | |
| splitter | divisor | A screen reader says *separador* (its role) |
| tooltip, hint | sugerencia | The AI's "suggest" is *sugerir*; the overlap is harmless |
| icon | ícono | |
| check box / radio button | casilla / botón de opción | "Marca la casilla" = tick it |
| combo box | lista desplegable | |
| text box, field | campo (de texto) | *Campo* where you type |
| label | etiqueta | |
| click / double-click / right-click | hacer clic / doble clic / clic derecho | |
| select / choose | seleccionar / elegir | *Seleccionar* for text and items; *elegir* for a choice among options |
| theme / look | tema / aspecto | "RapidR's look" = *el aspecto de RapidR* |
| Welcome page | Página de bienvenida | The pane tab is *Bienvenida* |

### 2.5 The language and programming

| English | Español neutro | Note |
|---|---|---|
| app (a finished program) | aplicación | Q2 (Robert) |
| form | formulario | See 1.3 |
| component | componente | |
| control | control | The English uses both; keep both. *Componente* when building, *control* for what the user touches |
| property | propiedad | |
| event | evento | |
| method | método | |
| event handler | controlador de eventos | Q4 (Robert) |
| variable / constant | variable / constante | |
| array | arreglo | Q3. Element = *elemento*, its position = *índice* |
| string | cadena de texto | *Cadena* after the first mention; in a screen, *texto* |
| integer | entero | "número entero" in the first mention |
| decimal number | número decimal | "punto flotante" only in the language chapter |
| function | función | A `FUNCTION` gives back a value |
| SUB | subrutina (`SUB`) | A routine with no result |
| parameter / argument | parámetro / argumento | |
| module | módulo | |
| include file | archivo incluido | The `$INCLUDE` directive stays |
| library | biblioteca | Q9 |
| keyword | palabra clave | |
| loop | ciclo | ⚑ *Bucle* is Spain's and also understood; "infinite loop" = *ciclo infinito* |
| compile / compiler | compilar / compilador | When code is turned into bytecode or machine code |
| build | compilar | Q1 (Robert). "Build Native App" = *Compilar aplicación nativa* |
| run | ejecutar | |
| debug / debugger | depurar / depurador | |
| interpreter / interpreted | intérprete / interpretado | |
| runtime | entorno de ejecución | The product **RapidR Runtime** keeps its name |
| bytecode | bytecode | Masculine: "el bytecode" |
| database / table / row / column / query | base de datos / tabla / fila / columna / consulta | |
| server / client / request / response | servidor / cliente / solicitud / respuesta | *Petición* is Spain's |

### 2.6 Studio's panes and areas

| English | Español neutro | Note |
|---|---|---|
| Project | Proyecto | |
| Toolbox | Caja de herramientas | The Tools **menu** is *Herramientas* |
| Properties | Propiedades | The pane's pages: *Propiedades* and *Eventos* |
| Outline | Estructura | Q5 (Robert) |
| Output | Salida | |
| Problems | Problemas | |
| Immediate | Inmediato | Q5 |
| Variables | Variables | |
| Watch | Observar | Q5 (Robert). "Add Watch" = *Añadir a Observar* |
| Call Stack | Pila de llamadas | |
| Breakpoints | Puntos de interrupción | |
| Search | Búsqueda | The verb on its button is *Buscar* |
| Help | Ayuda | |
| Assistant | Asistente | |
| AI Apps | Apps de IA | Q2 |
| Welcome | Bienvenida | |
| Design / Code | Diseño / Código | The two views of a form |
| Preview / Source | Vista previa / Código fuente | A Markdown file's two views |
| Debug Panes | Paneles de depuración | |
| Ln / Col | Lín / Col | `Lín 12, Col 5` |

### 2.7 Running and debugging

| English | Español neutro | Note |
|---|---|---|
| Run / Run Without Debugging | Ejecutar / Ejecutar sin depurar | |
| Stop / Restart | Detener / Reiniciar | |
| Pause / Continue | Pausar / Continuar | F5 says *Continuar* while paused |
| Step Into | Entrar en la llamada | Q6 |
| Step Over | Pasar por encima | Q6 |
| Step Out | Salir de la llamada | Q6. *Salir* alone is File > Exit |
| Run to Cursor | Ejecutar hasta el cursor | |
| Set Next Statement | Establecer la siguiente instrucción | |
| Toggle Breakpoint | Alternar punto de interrupción | |
| breakpoint: When / Hit / Log | punto de interrupción: Cuando / Veces / Registro | The three labels of the breakpoint dialog (condition, hit count, log message) |
| Show Program Window | Mostrar la ventana del programa | |
| Stop at Run-time Errors | Detener en errores de ejecución | |
| running / paused / stopped | en ejecución / en pausa / detenido | |
| run-time error / compile error | error de ejecución / error de compilación | |
| error / warning / note | error / advertencia / nota | The Problems pane's three kinds |
| Build / Build Native App | Compilar aplicación / Compilar aplicación nativa | Q1 (Robert) |
| Build Interpreted App / Build Web App | Compilar aplicación interpretada / Compilar aplicación web | Q1 (Robert) |
| Run in Browser | Ejecutar en el navegador | |
| Reveal the App | Mostrar la app en su carpeta | |

### 2.8 Designer, components, groups

| English | Español neutro | Note |
|---|---|---|
| designer | diseñador | |
| Bring to Front / Send to Back | Traer al frente / Enviar al fondo | |
| Menu Editor / Tab Order | Editor de menús / Orden de tabulación | |
| caption | texto / título | **Texto** of a button or label, **título** of a window; the property `Caption` is code |
| Toolbox groups: Forms and containers | Formularios y contenedores | These are the registry's group names |
| Buttons and input | Botones y entrada de datos | |
| Lists, grids and trees | Listas, tablas y árboles | |
| Menus / Dialogs | Menús / Cuadros de diálogo | |
| Display and drawing | Pantalla y dibujo | |
| Media / Databases / Data science | Multimedia / Bases de datos / Ciencia de datos | |
| Network, devices and CGI | Red, dispositivos y CGI | |
| Non-visual objects / Libraries | Objetos no visuales / Bibliotecas | |
| Web only / AI / IDE | Solo para la web / IA / Studio | The "IDE" group is Studio's own components |
| Property groups: Appearance / Behavior / Data / Network | Apariencia / Comportamiento / Datos / Red | "Layout" = *Disposición* |

The Toolbox lists components by their code names (`RButton`, `RLabel`…); those are never translated, only the group names above and each component's description.

### 2.9 Versions (the Simple words)

The Simple level speaks plain words and shows Git's name in a tooltip: "Git lo llama *commit*". The Pro level speaks Git (Q7).

| English (Simple) | Español neutro | Note (Git's name) |
|---|---|---|
| Versions | Versiones | The panel. The product never calls itself Git |
| Start versioning this project | Empezar a guardar versiones de este proyecto | `git init` |
| Save a version... | Guardar una versión... | commit |
| Timeline | Línea de tiempo | log |
| What changed? | ¿Qué cambió? | diff = *diferencias* |
| Go back to this version | Volver a esta versión | Never a hard reset |
| Restore a file | Restaurar un archivo | |
| Undo this version | Deshacer esta versión | revert |
| Try something new | Probar algo nuevo | Makes an **experimento** (branch) |
| Bring this experiment into main | Incorporar este experimento a la línea principal | merge. If the branch's name (`main`) is shown, it stays as data |
| Copy this change here | Copiar este cambio aquí | cherry-pick |
| Put my changes aside | Apartar mis cambios | stash |
| Mark this version | Marcar esta versión | tag; a **marca** in Simple (1.3) |
| Who wrote this line? | ¿Quién escribió esta línea? | blame |
| Share online / Get latest / Sync | Compartir en línea / Obtener lo último / Sincronizar | push / pull / both |
| Copy a project from the internet | Copiar un proyecto de internet | clone |
| Publish this project | Publicar este proyecto | |
| Shared folder | Carpeta compartida | |
| Safety point | Punto de seguridad | A hidden version saved before anything is rewritten |
| Checkpoint (the AI's) | Punto de control | One per AI turn, on the same timeline |
| Action history | Historial de acciones | With Undo |
| Open read-only | Abrir en solo lectura | |
| Yours / Theirs / Result (the merge tab) | Tu versión / La otra versión / Resultado | *Suyo* would sound like *usted* |
| change block (hunk) | bloque de cambios | |
| secret check | revisión de secretos | Before a version is saved |

### 2.10 AI

| English | Español neutro | Note |
|---|---|---|
| AI | IA | Never "AI". First mention in the manual: inteligencia artificial (IA) |
| RAI | RAI | The brand; no article where possible ("RAI puede…"), "el asistente RAI" otherwise |
| assistant | asistente | |
| AI app (an outside program that connects) | app de IA | Q2 |
| provider | proveedor | |
| model | modelo | |
| on this computer / cloud service | en esta computadora / servicio en la nube | "Free, on this computer" = *Gratis, en esta computadora* |
| API key | clave de API | Never *llave* |
| keychain | llavero | "your system's keychain" = *el llavero del sistema*. By its own name: *Llavero* (macOS), *Administrador de credenciales* (Windows) |
| token (AI: a piece of a word) | token | Plural *tokens*; first mention "tokens (fragmentos de palabras)" |
| token (access, for AI apps) | token de acceso | Reset Token = *Restablecer token* |
| context window | ventana de contexto | |
| chat | chat | Plural *chats*; "New Chat" = *Nuevo chat* |
| Set Up RAI... | Configurar RAI... | |
| propose a change | proponer un cambio | |
| Review the Changes... | Revisar los cambios... | |
| Apply | Aplicar | |
| Keep / Undo | Conservar / Deshacer | The AI's plain words for a change it made |
| Accept All / Reject All | Aceptar todo / Rechazar todo | **Rechazar** a change; **Denegar** a permission |
| audit log | registro de auditoría | |
| Read only | Solo lectura | The four permission modes |
| Ask before changes | Preguntar antes de cambiar | |
| Allow edits | Permitir ediciones | |
| Allow everything | Permitir todo | Status bar: "AI apps: allow everything" = *Apps de IA: permitir todo* |
| No questions asked (chip) | Sin preguntar | ⚑ |
| Allow Once / Allow for This Session / Deny | Permitir una vez / Permitir en esta sesión / Denegar | |
| Build / Pair / Coach / Quiz me (the dial) | Construir / En pareja / Guía / Ponme a prueba | Provisional: Preview 5, not built yet |

### 2.11 Accessibility

| English | Español neutro | Note |
|---|---|---|
| accessibility | accesibilidad | |
| screen reader | lector de pantalla | |
| accessible name / description | nombre accesible / descripción accesible | `AccessibleName` and `AccessibleDescription` stay code |
| keyboard focus | foco del teclado | |
| focus ring | anillo de foco | |
| high contrast | contraste alto | |
| reduced motion | movimiento reducido | |
| light / dark (look) | claro / oscuro | |
| keyboard shortcut | atajo de teclado | |
| mnemonic (the underlined letter) | tecla de acceso | |
| tab order | orden de tabulación | |
| announce | anunciar | `Application.Announce` stays code |
| enabled / disabled / greyed out | habilitado / deshabilitado / atenuado | |

### 2.12 Errors and messages

Studio, the compiler and RAI say what happened, then what to do. Same shape in Spanish.

| English | Español neutro | Note |
|---|---|---|
| can't X | No se puede X | General inability: "No se puede abrir este archivo" |
| couldn't X | No se pudo X | A try that just failed: "No se pudo guardar" |
| Studio couldn't run it | Studio no pudo ejecutarlo | Studio as the subject |
| not found | No se encontró X | "include file not found" = *no se encontró el archivo incluido* |
| isn't there any more | ya no está | |
| doesn't exist / already exists | no existe / ya existe | |
| expected X | Se esperaba X | The compiler: "Expected )" = *Se esperaba )* |
| unexpected X | no se esperaba X | "Syntax error: unexpected 'x'" = *Error de sintaxis: no se esperaba 'x'* |
| Syntax error | Error de sintaxis | |
| unexpected end of file | fin de archivo inesperado | |
| missing X | Falta X | |
| invalid | no válido (no válida) | Not *inválido* |
| unknown / not defined | desconocido / no definido | |
| is required | es obligatorio (obligatoria) | |
| out of range / too long | fuera de rango / demasiado largo (larga) | |
| read-only | solo lectura | No accent on *solo* (RAE 2010) |
| unsaved changes | cambios sin guardar | |
| Nothing to undo | No hay nada que deshacer | |
| Try again | Inténtalo de nuevo | |
| Something went wrong | Algo salió mal | |
| Your work is safe | Tu trabajo está a salvo | |
| Are you sure? | (avoid) | As in the English guide: ask the concrete question with verb buttons: "¿Eliminar el chat “{title}”?" with Eliminar / Cancelar |

Compiler and run-time messages keep the code exactly as written inside them (`'x'`, `SUB`, line numbers) and translate only the words around it. OS and tool messages quoted from outside (`Access is denied. (os error 5)`, `libssl.so.3`) are quoted as they appear, with the Spanish sentence next to them.

---

## 3. Never translate

1. **The language.** Keywords, statements, directives, built-in functions, types and constants: `PRINT`, `DIM`, `SUB`, `FUNCTION`, `IF … THEN`, `$INCLUDE`, `$RESOURCE`, `TR$`, `TRN$`, `INTEGER`, `STRING`, `clRed`, `mbYes`. In prose they are in code font and never get a plural s.
2. **Names in code.** Component classes (`RForm`, `RButton`, `RLabel`, `REdit`, `RStringGrid`, `RDiffView`, `RDataFrame`, `RMcpServer`, `RSecrets`, `RAI`…) and the older Q names (`QFORM`…); properties, methods, events and their values (`Caption`, `Left`, `Show`, `OnClick`, `bsSizeable`); global objects (`Application`); the names a person gives (`Form1`, `Button1`).
3. **Files, commands and addresses.** Extensions (`.rr`, `.bas`, `.rrproj`, `.rrbc`, `.po`, `.md`, `.csv`), file and folder names (`hello.rr`, `lang/`, `ai-audit.jsonl`), paths, environment variables (`ANTHROPIC_API_KEY`), URLs (`http://127.0.0.1:47330/mcp`), the `rapidr` command line and its flags, shell commands.
4. **Identifiers.** Command ids and icon names (`file.save`, `view.theme.dark`), settings and registry keys, JSON field names, log fields, MCP tool names (`studio_status`, `read_file`, `propose_edit`, `app_click`…), the stored ids of permission modes. Only their labels are translated.
5. **Names of products, companies and protocols.** RapidR, RapidR Studio (*Studio* for short), RAI, RapidR Runtime, RapidR SDK, RapidR Database Studio, RapidQ, Git, GitHub, Anthropic, Claude, OpenAI, DeepSeek, Ollama, LM Studio, MLX Serve, MCP, Model Context Protocol, WebMCP, Windows, macOS, Linux, Ubuntu, Debian, DirectX, Direct3D, SQLite, MySQL, Rust. A third party is named only where the person must recognise it.
6. **Acronyms, formats and loanwords.** API, URL, HTTP(S), TLS, SSH, JSON, CSV, SQL, PO, UTF-8, PNG, SVG, ICO, DPI, GPU, CPU, MIDI, CGI, WCAG, GB, MB, `Regex`, `token`, `chat`, `app` (follows Q2), `mouse`, `bytecode`.
7. **Placeholders and markup.** `{name}`, `{n}`, `{file}`, `\n`, `\t`, `&` (the letter is chosen, the sign stays) and `&&`, Markdown, HTML, backticks.
8. **Keys and shortcuts.** Key names (`Ctrl`, `Shift`, `Alt`, `Enter`, `Esc`, `Tab`, `Del`, F1–F12, ⌘ ⌥ ⇧) and every combination (`Ctrl+Shift+P`); see 1.7.
9. **Language codes and numbers of versions.** `es`, `es-AR`, `pt-BR`, `zh-Hans`; `2.119.0`; release names such as "Preview 4" (⚑ Robert: should a release name ever be Spanish?).
10. **What belongs to the person or the program.** A user's own program text, its output, data values in samples ("Ana"), file contents, anything RAI quotes from a project.
11. **Quoted outside text.** An operating system's or another tool's message, quoted as it appears (see 2.12).
12. **Legal texts.** Licences, notices and `LEGAL.md` (1.11).

---

## 4. Open questions for Robert

Ten questions, each with two options. My recommendation is the one the glossary already uses. If you pick the other, only the rows named under the question change.

**Q1. "Build": *crear* or *compilar*?** Run > Build, Build Native App, Build Interpreted App, Build Web App (rows in 2.7).
- A. **Crear**: "Crear app", "Crear app nativa", "Crear app web". Works for all three kinds (an interpreted or web app isn't compiled), and a student reads it without knowing what a compiler is. It overlaps lightly with "Create and Open" for projects.
- B. **Compilar**: "Compilar app nativa". Shorter and familiar to programmers, but wrong for the interpreted and web builds, where nothing is compiled.
- **Recommendation: A.** *Compilar* stays for the places where code really is compiled (bytecode, machine code).

**Q2. "App": *app* or *aplicación*?** The pane **AI Apps**, the build commands, "AI apps" in every consent text.
- A. **app** everywhere in the UI, "Apps de IA"; *aplicación* only in formal manual prose. Short, and everyone from Mexico to Argentina says it. Plural *apps*.
- B. **aplicación** everywhere: "Aplicaciones de IA" (20 characters on a pane tab), "Crear aplicación nativa".
- **Recommendation: A**, because the English UI is deliberately short and the tabs have little room.

**Q3. Arrays: *arreglo*, *array* or *matriz*?** Language chapter, the designer, error messages.
- A. **arreglo** (an array, one dimension; "arreglo bidimensional" for two). A real Spanish word, widely taught in Mexico, Colombia and Peru and understood in the Río de la Plata (⚑ Robert, what do you say in Uruguay?). *Matriz* is kept for the math (`RNum`).
- B. **array**, the English word as is (the loanword). Common in Uruguay, Argentina and Chile when programmers talk, zero ambiguity with matrices, but it leaves an English word inside Spanish sentences, which a school may not want.
- **Recommendation: A**, with "(array)" beside it on the first mention in each chapter so searching works.

**Q4. "Event handler": *manejador de eventos* or *controlador de eventos*?**
- A. **manejador de eventos.** It doesn't collide with *control*, the word the English UI also uses for a component on a form.
- B. **controlador de eventos.** A term many Spanish-speaking students may already know, but it sits next to *control* and *controlar* in the same screen, and *controlador* also means a hardware driver.
- **Recommendation: A.**

**Q5. Three pane names that have no obvious Spanish: Watch, Immediate, Outline.**
- A. **Inspección, Inmediato, Esquema.** The usual Spanish names for these three debugger panes (⚑ a native check welcome).
- B. **Observar, Consola, Estructura.** Plainer for a beginner, but *Consola* collides with the console a program prints to (`PRINT`, the Output pane), and *Observar* is a verb among nouns.
- **Recommendation: A.**

**Q6. Step commands: short and plain, or the long standard ones?**
- A. **Entrar en la llamada** (Step Into), **Pasar por encima** (Step Over), **Salir de la llamada** (Step Out). Say what the key does to a person who has never heard "step into".
- B. **Paso a paso por instrucciones / por procedimientos / para salir.** The long, literal form; accurate, but about 30 characters in a tool-bar tooltip and unclear to a beginner.
- **Recommendation: A.**

**Q7. Pro level of Versions: Git's words in English or Spanish?** (Simple is plain Spanish either way: 2.9.)
- A. **English for the commands, Spanish for the nouns.** *Commit, merge, rebase, stash, cherry-pick, push, pull, fetch, blame* stay in English, since people type and search them that way and the tooltip says "Git lo llama *commit*". *Rama, etiqueta, repositorio, conflicto, remoto* are Spanish.
- B. **Everything Spanish.** *Confirmar, fusionar, reorganizar, apartar, enviar, traer…* Consistent, but few people search the web for "fusionar"; a student would meet the English words the first time they leave Studio.
- **Recommendation: A.**

**Q8. "Add": *agregar* or *añadir*?** Your sample `examples/translations/lang/es.po` has "&Añadir un archivo".
- A. **agregar** (Agregar archivo, Agregar formulario). The word most used from Mexico to Argentina; *añadir* is also understood but reads as Spain's.
- B. **añadir.** Correct and understood everywhere, as in your sample.
- **Recommendation: A**, and the sample file changes to `&Agregar un archivo` when the glossary is approved.

**Q9. "Library": *biblioteca* or *librería*?** The Toolbox group "Libraries", the manual.
- A. **biblioteca.** The correct word (RAE); in a dictionary *librería* is a bookshop.
- B. **librería.** What many Latin-American programmers say aloud (⚑ Robert, what do you say?).
- **Recommendation: A** in the product and the manual; people who say *librería* will still find it.

**Q10. Code samples in the manual: translate the comments and the shown text, or keep all code in English?**
- A. **Translate** the comments and the string literals in the manual's samples ("Hello, World!" becomes "¡Hola, mundo!", and the sample's printed output with it), never the code itself. A beginner can read the whole page. The runnable files in `examples/` stay as they are (English code, `TR$` for the text).
- B. **Keep every sample in English**, comments and strings included. Zero risk of a sample that doesn't match its output; a beginner meets English at the first line.
- **Recommendation: A**, with each translated sample run once to check its output.

---

## 5. Sources read

`ide/commands.inc` (all command and menu titles), the `Caption =` assignments and `Dock.AddPane` titles in `ide/*.inc` (the `TR$` function isn't used in `ide/` yet), `docs/manual/README.md`, `getting-started.md`, `ai-apps.md`, `versions.md`, `debugging.md`, `translations.md`, `troubleshooting.md`, `docs/versions-plan.md` (the table of Simple words), `docs/design-guidelines.md` ("Words", "Buttons", "Confirmations and consent"), `docs/ai-ux-plan.md`, `docs/learn-with-rai-plan.md`, `docs/local-ai-plan.md`, `docs/master-plan.md` (M5), `crates/rapidr-lang/data/components/*.toml` (the `group` and `category` names), and `examples/translations/lang/es.po`.
