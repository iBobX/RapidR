# Spanish for RapidR Studio: review sheet (ES-STUDIO, batch 1)

For Robert, 2026-10-30. First pass of every Studio text in Spanish (español neutro, [the glossary](glossary-es.md)): **1,654 of 1,654 texts** (Studio's own 1,334 + RapidR's own 320 shown in its panes and dialogs) and **104 of 104 compiler texts** (35 messages + 69 hints under them). Nothing is marked "to review" except the doubts below. How to see it: Settings > General > Idioma > Español, or `rapidr ide --lang es`. Files: `ide/lang/es.po`, `crates/rapidr-value/lang/es.po`, `crates/rapidr-diagnostics/lang/es.po`.

1. [The 40 most visible strings](#1-the-40-most-visible-strings)
2. [Where I had a real doubt](#2-where-i-had-a-real-doubt) (marked `#, review` in the catalogs)
3. [The legal texts side by side](#3-the-legal-texts-side-by-side) (marked `# legal` in `ide/lang/es.po`)
4. [What remains](#4-what-remains)

## 1. The 40 most visible strings

The menu bar, the main commands, the Welcome page, the status bar. `&` marks the underlined letter (hidden here).

| # | English | Español |
|---|---|---|
| 1 | File | Archivo |
| 2 | Edit | Edición |
| 3 | View | Ver |
| 4 | Project | Proyecto |
| 5 | Run | Ejecutar |
| 6 | Debug | Depurar |
| 7 | Format | Formato |
| 8 | Tools | Herramientas |
| 9 | Window | Ventana |
| 10 | Help | Ayuda |
| 11 | New Project... | Nuevo proyecto... |
| 12 | Open Project or File... | Abrir proyecto o archivo... |
| 13 | Save | Guardar |
| 14 | Save As... | Guardar como... |
| 15 | Save All | Guardar todo |
| 16 | Save a Version... | Guardar una versión... |
| 17 | Exit | Salir |
| 18 | Undo | Deshacer |
| 19 | Redo | Rehacer |
| 20 | Find... | Buscar... |
| 21 | Replace... | Reemplazar... |
| 22 | Command Palette... | Paleta de comandos... |
| 23 | Reset Layout | Restablecer disposición |
| 24 | Run | Ejecutar |
| 25 | Stop | Detener |
| 26 | Build | Compilar |
| 27 | Build Native App | Compilar aplicación nativa |
| 28 | Step Into | Entrar en la llamada |
| 29 | Step Over | Pasar por encima |
| 30 | Step Out | Salir de la llamada |
| 31 | Toggle Breakpoint | Alternar punto de interrupción |
| 32 | Settings... | Configuración... |
| 33 | Set Up RAI... | Configurar RAI... |
| 34 | RapidR Help | Ayuda de RapidR |
| 35 | Toolbox | Caja de herramientas |
| 36 | Outline | Estructura |
| 37 | Start | Comenzar |
| 38 | Recent | Recientes |
| 39 | Examples | Ejemplos |
| 40 | BASIC is powerful again: desktop and web apps from one source. | BASIC vuelve a ser potente: aplicaciones de escritorio y web desde un solo código fuente. |

## 2. Where I had a real doubt

| English | Español | Why |
|---|---|---|
| Case | Mayús. | The search toggle. The glossary says "Mayúsculas y minúsculas": 23 characters do not fit the Find pane, so the checkbox reads "Mayús." and the tooltip says it whole. |
| Word | Palabra | Same: the glossary says "Palabra completa"; "Palabra" fits, the tooltip says "Solo palabras completas". |
| Whole words only | Solo palabras completas | The tooltip that completes the two above. |
| Back | Atrás | One text for the Chats list and the Set Up RAI wizard. The glossary: "Atrás" in a wizard, "Volver" from the Chats list. One word had to serve both: "Atrás". |
| Add a watch: | Añadir a Observar: | A watch is an expression you observe. "Añadir a Observar" follows the glossary; the plural noun is "Expresiones observadas" (my word, not in the glossary). |
| Watches | Expresiones observadas | See above. |
| Call Stack | Pila de llamadas | Pane tab. "Pila de llamadas" and "Puntos de interrupción" (the next row) are the glossary words, but at the default 1280 px the debug pane group is narrow and the dock trims the longest tab titles ("Pila de llama…", "Puntos de int…"; the whole title is the tooltip). Shorter tab words ("Llamadas", "Interrupciones") would fit: your call. |
| Breakpoints | Puntos de interrupción | See above. |
| Show Hover | Mostrar información flotante | The editor command that shows the tooltip of the word under the cursor. "Mostrar información flotante": I found no neutral one-word term. |
| Go to Anything... | Ir a cualquier elemento... | "Ir a cualquier elemento..." (the command palette's other name). "Ir a cualquier lugar" would also do. |
| Fold | Plegar | "Plegar" / "Desplegar" / "Plegar todo": what the Spanish code editors use for code folding. "Contraer/Expandir" is the other family. |
| Select Next Occurrence | Seleccionar siguiente ocurrencia | "ocurrencia" (an anglicism that programmers use) or "coincidencia". |
| Includes | Archivos incluidos | The Project tree group of include files: "Archivos incluidos" (the glossary: include file = archivo incluido). |
| Assets | Activos | "Activos" (pictures, sounds: what a game engine calls assets). "Recursos" is taken by Resources. |
| asset | activo | See above (the tree's kind word for one). |
| handle | identificador | An operating-system handle (a value in the debugger): "identificador". |
| Dock | Acoplar | The pane's own menu, put it in its dock: "Acoplar" (dock); "Ocultar automáticamente" for Auto Hide. |
| A data frame and a chart | Un marco de datos y un gráfico | RDataFrame as "marco de datos" (the term Spanish-speaking data people use), not "DataFrame". |
| Data dashboard | Tablero de datos | "Tablero de datos" (dashboard): Mexico/Argentina say "tablero", Spain "panel"; "panel" is taken by pane. |
| Recipe: a drawing pad (RCanvas) | Receta: un tablero de dibujo (RCanvas) | "Tablero de dibujo" for a drawing pad; "lienzo" is the canvas. |
| Quiet: the marks only | Discreto: solo las marcas | The three levels of Code helpers: "Discreto" (Quiet) / "Útil" (Helpful) / "Todo": plain words, not in the glossary. |
| Helpful: marks, the line's help, argument names | Útil: marcas, la ayuda de la línea, nombres de argumentos | See above. |
| Effort: Auto | Esfuerzo: auto | "Esfuerzo: auto": shortened from "automático" so the chip fits the narrow Assistant pane (the words in the Settings list are "Automático", "Bajo", "Medio", "Alto"). |
| No questions asked | Sin preguntar | The chip of the "Allow everything" mode. The glossary marked it as unsure: "Sin preguntar". |
| AI checkpoint: {what}, {when} | Punto de control de la IA: {what}, {when} | "Punto de control de la IA" (the glossary: checkpoint = punto de control). |
| Compact | Compactar | Compact the chat (summarise it): "Compactar". |
| Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence. | Copyright (c) 2025-2026 Ruta Internet SRL. Licencia MIT. | Only "MIT licence" is translated ("Licencia MIT"); the notice itself is English by the glossary (legal text). |
| A key ({hint}) is found in the {source}: the code never holds it. | Hay una clave ({hint}) en {source}: el código nunca la contiene. | {source} is a name from the program's code ("the environment", "the keychain") that comes in English: the sentence cannot take its article. Needs the Rust side to translate the name (see "what remains"). |
| Waiting for {name}'s answer… {n} s | Esperando la respuesta… {n} s ({name}) | {name} can be "Claude", a provider's name, or "the server": "de {name}" would give "de el servidor", so the sentence puts the name in parentheses: "Esperando la respuesta… 12 s (Claude)". |
| hint | sugerencia | The word that labels a hint under a compiler message ("— sugerencia: …"). "Pista" is the other candidate. |

## 3. The legal texts side by side

Consent, privacy, cost, key, age and secret-check texts, translated point by point (nothing shortened or softened; where the layout needed room, the Spanish was tightened without dropping a point). 60 texts. For the lawyer check: `grep -B1 "^# legal" ide/lang/es.po`.

| English | Español |
|---|---|
| The last request, as sent (your key isn't in it) | La última solicitud, como se envió (tu clave no está en ella) |
| Send shares your message, this chat, what the assistant reads of this project and, while your program runs, what its windows show (it asks you first) with {provider}: nothing else, nothing before. | Enviar comparte con {provider} tu mensaje, este chat, lo que el asistente lee de este proyecto y, mientras tu programa se ejecuta, lo que muestran sus ventanas (antes te pregunta): nada más, nada antes. |
| RAI wants to click and type in your running program: Allow for this run, or Don't allow, in its chat. | RAI quiere hacer clic y escribir en tu programa en ejecución: elige Permitir en esta ejecución, o No permitir, en su chat. |
| Your key is never shown or logged. Every request and change is in the audit log on this computer: {location} | Tu clave nunca se muestra ni se registra. Cada solicitud y cada cambio quedan en el registro de auditoría de esta computadora: {location} |
| Type the API key in APIKey (kept in your keychain, never in the code): the provider's models are listed here then. | Escribe la clave de API en APIKey (se guarda en tu llavero, nunca en el código): entonces los modelos del proveedor aparecen aquí. |
| The key is kept in {store} as {q}{name}{q}: the program names it, never the key. | La clave se guarda en {store} como {q}{name}{q}: el programa la nombra, nunca contiene la clave. |
| A key ({hint}) is kept in {store} as {q}{name}{q}: the code names it, never the key. Type another to replace it. | Hay una clave ({hint}) guardada en {store} como {q}{name}{q}: el código la nombra, nunca contiene la clave. Escribe otra para reemplazarla. |
| A key ({hint}) is found in the {source}: the code never holds it. | Hay una clave ({hint}) en {source}: el código nunca la contiene. |
| Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good. | Se guardan en esta computadora, en la carpeta propia de Studio — nunca en el proyecto. Eliminar quita un chat para siempre. |
| Nothing you ask leaves this computer, and it costs nothing. A model runs here with Ollama, LM Studio or MLX Serve (free programs you install). It needs a fast computer with plenty of memory (16 GB or more); answers are slower and simpler than a cloud model's. | Nada de lo que preguntas sale de esta computadora, y no cuesta nada. Un modelo se ejecuta aquí con Ollama, LM Studio o MLX Serve (programas gratuitos que tú instalas). Necesita una computadora rápida con mucha memoria (16 GB o más); las respuestas son más lentas y más simples que las de un modelo en la nube. |
| Anthropic (Claude), OpenAI or DeepSeek: usually the best answers. You make a key on the provider's website and pay it for what you use: a question usually costs from a tenth of a cent (small, fast models) to a few cents (the largest). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to that provider, under its privacy terms. Your key stays in this computer's keychain, never in a project. | Anthropic (Claude), OpenAI o DeepSeek: normalmente las mejores respuestas. Creas una clave en el sitio web del proveedor y le pagas por lo que usas: una pregunta suele costar desde una décima de centavo (modelos pequeños y rápidos) hasta unos pocos centavos (los más grandes). Lo que envías —tu pregunta, el chat, las partes de tu proyecto que RAI lee y, mientras tu programa se ejecuta, lo que muestran sus ventanas— va a ese proveedor, bajo sus términos de privacidad. Tu clave se queda en el llavero de esta computadora, nunca en un proyecto. |
| Paste the key the provider gave you: it is kept in this computer's keychain, never in a project | Pega la clave que te dio el proveedor: se guarda en el llavero de esta computadora, nunca en un proyecto |
| What it costs, and what is sent | Cuánto cuesta y qué se envía |
| {name} is running here. Choose a model and Finish. Free, and nothing you ask leaves this computer. A model of a small window (under 16,000 tokens) gets RAI's lite instructions, so it can still help. | {name} se está ejecutando aquí. Elige un modelo y presiona Finalizar. Es gratis, y nada de lo que preguntas sale de esta computadora. Un modelo con una ventana pequeña (menos de 16 000 tokens) recibe las instrucciones ligeras de RAI, así que aún puede ayudar. |
| Claude's small, fast models (Haiku) cost about a tenth of a cent a question; the largest (Opus) a few cents. Anthropic charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to Anthropic, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key. | Los modelos pequeños y rápidos de Claude (Haiku) cuestan alrededor de una décima de centavo por pregunta; el más grande (Opus), unos pocos centavos. Anthropic cobra a tu cuenta lo que usas: fija un límite mensual en su sitio web. Lo que envías —tu pregunta, el chat, las partes de tu proyecto que RAI lee y, mientras tu programa se ejecuta, lo que muestran sus ventanas— va a Anthropic, bajo sus términos de privacidad. Tu clave se queda en el llavero de esta computadora, nunca en un proyecto. Los servicios en la nube son para personas adultas: si tienes menos de 18 años, pide a un padre, una madre o un docente que cree la clave. |
| OpenAI's small models (mini, nano) cost a fraction of a cent a question; the largest a few cents. OpenAI charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to OpenAI, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key. | Los modelos pequeños de OpenAI (mini, nano) cuestan una fracción de centavo por pregunta; el más grande, unos pocos centavos. OpenAI cobra a tu cuenta lo que usas: fija un límite mensual en su sitio web. Lo que envías —tu pregunta, el chat, las partes de tu proyecto que RAI lee y, mientras tu programa se ejecuta, lo que muestran sus ventanas— va a OpenAI, bajo sus términos de privacidad. Tu clave se queda en el llavero de esta computadora, nunca en un proyecto. Los servicios en la nube son para personas adultas: si tienes menos de 18 años, pide a un padre, una madre o un docente que cree la clave. |
| DeepSeek's models cost a fraction of a cent a question. DeepSeek charges your account for what you use (you pay in advance on its website). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to DeepSeek, in China, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key. | Los modelos de DeepSeek cuestan una fracción de centavo por pregunta. DeepSeek cobra a tu cuenta lo que usas (pagas por adelantado en su sitio web). Lo que envías —tu pregunta, el chat, las partes de tu proyecto que RAI lee y, mientras tu programa se ejecuta, lo que muestran sus ventanas— va a DeepSeek, en China, bajo sus términos de privacidad. Tu clave se queda en el llavero de esta computadora, nunca en un proyecto. Los servicios en la nube son para personas adultas: si tienes menos de 18 años, pide a un padre, una madre o un docente que cree la clave. |
| You pay the provider for what you use. What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key. | Le pagas al proveedor por lo que usas. Lo que envías —incluido, mientras tu programa se ejecuta, lo que muestran sus ventanas— va a él, bajo sus términos de privacidad. Tu clave se queda en el llavero de esta computadora, nunca en un proyecto. Los servicios en la nube son para personas adultas: si tienes menos de 18 años, pide a un padre, una madre o un docente que cree la clave. |
| Paste your API key first: the provider lists its models only to a key it knows. | Pega primero tu clave de API: el proveedor solo enumera sus modelos a una clave que conoce. |
| Warning: AI apps may change and run anything in this project without asking | Advertencia: las aplicaciones de IA pueden cambiar y ejecutar cualquier cosa de este proyecto sin preguntar |
| Off by default. On: an AI app on this computer that has the address and token below can work with your open project | Desactivado de forma predeterminada. Activado: una aplicación de IA en esta computadora que tenga la dirección y el token de abajo puede trabajar con tu proyecto abierto |
| When on, an AI app on this computer that has the address and token below can work with your open project; you see everything it does in the AI Apps pane (Tools > AI Apps Activity). Paste both into the AI app's MCP settings. | Cuando está activado, una aplicación de IA en esta computadora que tenga la dirección y el token de abajo puede trabajar con tu proyecto abierto; ves todo lo que hace en el panel Aplicaciones de IA (Herramientas > Actividad de aplicaciones de IA). Pega ambos en la configuración MCP de la aplicación de IA. |
| AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program. | Las aplicaciones de IA pueden mirar —leer el proyecto, revisar código, consultar cosas, ver la salida del programa y Studio—, pero no cambiar, ejecutar, mirar ni usar tu programa. |
| Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you. | Los cambios a los archivos se hacen de inmediato (cada uno es un paso de Deshacer, listado en el panel Aplicaciones de IA); ejecutar el programa, mirarlo o usarlo, y preguntarle a RAI siguen esperando tu permiso. |
| Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project. | Nada espera tu permiso: las aplicaciones de IA cambian archivos, ejecutan el programa, lo miran y lo usan, y le preguntan a RAI por su cuenta — y un programa puede hacer en esta computadora todo lo que tú puedes. Todo sigue listado, Detener lo termina y las herramientas propias de Studio nunca entregan claves ni archivos de fuera del proyecto. |
| AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny. | Las aplicaciones de IA pueden mirar el proyecto con libertad; cada cambio, ejecución, vistazo a tu programa en ejecución o pregunta a RAI espera tu decisión en el panel Aplicaciones de IA: Permitir una vez, Permitir en esta sesión o Denegar. |
| New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it. | Proyecto nuevo: Permitir ediciones. Permitir todo no pasa a un proyecto nuevo: vuelve a elegirlo aquí si lo quieres. |
| Allow everything for AI apps in this project? | ¿Permitir todo a las aplicaciones de IA en este proyecto? |
| Nothing waits for you: AI apps change files, run the program, look at it, click and type in it and ask RAI (which may cost money) on their own. | Nada espera tu permiso: las aplicaciones de IA cambian archivos, ejecutan el programa, lo miran, hacen clic y escriben en él y le preguntan a RAI (lo cual puede costar dinero) por su cuenta. |
| Keep in mind | Ten en cuenta |
| A program they run can do anything you can on this computer, and what they click in it is real. Choose this only while you watch an AI app you trust. | Un programa que ejecuten puede hacer en esta computadora todo lo que tú puedes, y los clics que hagan en él son reales. Elige esto solo mientras observas una aplicación de IA en la que confías. |
| You still see | Sigues viendo |
| Everything they do, listed in the AI Apps pane; Stop ends it. | Todo lo que hacen, listado en el panel Aplicaciones de IA; Detener lo termina. |
| Never | Nunca |
| Studio's own tools never hand over your keys, or files outside the project. | Las herramientas propias de Studio nunca entregan tus claves ni archivos de fuera del proyecto. |
| AI apps change and run anything in this project without asking | Las aplicaciones de IA cambian y ejecutan cualquier cosa de este proyecto sin preguntar |
| It can see | Puede ver |
| It can | Puede |
| It never sees | Nunca ve |
| The files of your open project — your code, forms and data files — and nothing else on your computer. It also gets pictures of Studio's window and of your running program's windows, and a written list of what they show, as you see them. | Los archivos de tu proyecto abierto —tu código, tus formularios y tus archivos de datos— y nada más de tu computadora. También recibe imágenes de la ventana de Studio y de las ventanas de tu programa en ejecución, y una lista escrita de lo que muestran, tal como tú lo ves. |
| Check code, and read what your program prints and RAI's chat. It can also change forms and code, open other projects, run, debug, look at, click and type in your program, and ask RAI (which may cost money), each only as your permission mode allows; the default asks you first. | Revisar código y leer lo que imprime tu programa y el chat de RAI. También puede cambiar formularios y código, abrir otros proyectos, ejecutar, depurar, mirar tu programa, hacer clic y escribir en él, y preguntarle a RAI (lo cual puede costar dinero), cada cosa solo como lo permita tu modo de permisos; el predeterminado te pregunta primero. |
| Your AI provider keys or passwords: your keychain keeps them. | Tus claves de proveedores de IA o tus contraseñas: tu llavero las guarda. |
| RapidR sends none of this anywhere, but the AI app may pass what it reads to the company that makes it, under that company's terms. A program can show private data, yours or other people's, and clicking in it acts as you: a real Send, Pay or Delete button does it for real, so use test data. Many AI apps are for adults only: if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust. | RapidR no envía nada de esto a ningún lugar, pero la aplicación de IA puede pasar lo que lee a la empresa que la fabrica, bajo los términos de esa empresa. Un programa puede mostrar datos privados, tuyos o de otras personas, y hacer clic en él actúa como tú: un botón real de Enviar, Pagar o Eliminar lo hace de verdad, así que usa datos de prueba. Muchas aplicaciones de IA son solo para personas adultas: si tienes menos de 18 años, configura esto con un padre, una madre o un docente. Conecta solo aplicaciones de IA en las que confíes. |
| Share only what you may share: schoolwork, an employer's code or someone else's licensed code may not be yours to share. Turn this off, or Reset Token, to disconnect every AI app. Studio keeps a list of what they did on this computer only; Clear Activity removes it. | Comparte solo lo que puedas compartir: las tareas escolares, el código de un empleador o el código con licencia de otra persona pueden no ser tuyos para compartir. Desactiva esto, o usa Restablecer token, para desconectar todas las aplicaciones de IA. Studio guarda una lista de lo que hicieron solo en esta computadora; Borrar actividad la quita. |
| {client} wants to look at or use your running program: {what}. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data. | {client} quiere mirar o usar tu programa en ejecución: {what}. Permitir una vez cubre imágenes, clics y escritura hasta que termine esta ejecución. Puede ver todo lo que el programa muestre, incluidos datos privados. |
| Let AI apps use RapidR Studio? | ¿Permitir que las aplicaciones de IA usen RapidR Studio? |
| With this on, an AI app on this computer that has your token can work with your open project through Studio, and you see everything it does in the AI Apps pane. | Con esto activado, una aplicación de IA en esta computadora que tenga tu token puede trabajar con tu proyecto abierto a través de Studio, y ves todo lo que hace en el panel Aplicaciones de IA. |
| AI apps that have the token can connect to RapidR Studio | Las aplicaciones de IA que tengan el token pueden conectarse a RapidR Studio |
| The list of what AI apps did is removed from this computer ({location}). | La lista de lo que hicieron las aplicaciones de IA se quita de esta computadora ({location}). |
| Off by default. On, Studio shows the AI Assistant panel (View > AI Assistant, Ctrl+I): ask about your project or for a new program. Nothing is sent until you press Send there. A new program it makes opens in Studio at once (Undo takes it back); changes to your files are shown for you to review, and nothing runs until you press Run. | Desactivado por defecto. Activado, Studio muestra el panel Asistente de IA (Ver > Asistente de IA, Ctrl+I): pregunta por tu proyecto o pide un programa nuevo. No se envía nada hasta que presionas Enviar allí. Un programa nuevo se abre en Studio al instante (Deshacer lo revierte); los cambios a tus archivos se muestran para que los revises y nada se ejecuta hasta que presionas Ejecutar. |
| Paste the key the provider gave you; it is kept in your system's keychain, never in a project | Pega la clave que te dio el proveedor; se guarda en el llavero de tu sistema, nunca en un proyecto |
| Kept in this browser's storage for the page: anyone using this browser profile could read it | Se guarda en el almacenamiento de este navegador para la página: cualquier persona que use este perfil del navegador podría leerla |
| Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project — nothing outside it, nothing before you press Send. A log of every request and change stays on this computer. | Enviar comparte tu mensaje, el chat hasta ahora, las instrucciones de RapidR y lo que el asistente lee del proyecto abierto — nada fuera de él, nada antes de que presiones Enviar. Un registro de cada solicitud y cambio se queda en esta computadora. |
| A local server needs no key (one is kept if you give it: some servers ask for one). | Un servidor local no necesita clave (se guarda una si la das: algunos servidores la piden). |
| Your key {hint} is kept in {store}, never in a project. | Tu clave {hint} se guarda en {store}, nunca en un proyecto. |
| Paste your key and press Save Key: it is kept in {store} (or on this device, if you tick the box), never in a project. | Pega tu clave y presiona Guardar clave: se guarda en {store} (o en este dispositivo, si marcas la casilla), nunca en un proyecto. |
| Paste your key and press Save Key: it is kept in {store}, never in a project. | Pega tu clave y presiona Guardar clave: se guarda en {store}, nunca en un proyecto. |
| Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence. | Copyright (c) 2025-2026 Ruta Internet SRL. Licencia MIT. |
| This line looks like a key or a password. Versions are often shared, and a secret saved in one stays in the timeline: take it out first, or save anyway if it isn't a real one.  \|\|  These {n} lines look like keys or passwords. Versions are often shared, and a secret saved in one stays in the timeline: take them out first, or save anyway if they aren't real ones. | Esta línea parece una clave o una contraseña. Las versiones a menudo se comparten, y un secreto guardado en una se queda en la línea de tiempo: quítalo primero, o guarda de todos modos si no es uno real.  \|\|  Estas {n} líneas parecen claves o contraseñas. Las versiones a menudo se comparten, y un secreto guardado en una se queda en la línea de tiempo: quítalos primero, o guarda de todos modos si no son reales. |
| Not saved: lines look like keys or passwords. The dialog lists them. | No se guardó: hay líneas que parecen claves o contraseñas. El cuadro de diálogo las enumera. |

## 4. What remains

What the walk through Studio in Spanish (`tests/studio_lang.mjs`, 29 scenes, light theme, desktop; captures in `tests/results/studio-lang/es-*.bmp` when you run it) still shows, and what it cannot yet:

- **English that comes from Rust code, not from a catalog.** Each needs a `tr_ui` or a catalog entry (a code change, so left for the lead):
  - The RAI chat's cards and replies built in `crates/rapidr-studio/src/ai.rs`. The consent card for clicking and typing in a running program ("RAI wants to click and type in your running program… [Allow for this run] [Don't allow]", line 805) carries legal weight: translate it before Spanish ships. The Spanish sentence that points to it (the status line) already says "Permitir en esta ejecución" / "No permitir".
  - The Versions pane's undo line: "Deshacer: Saved the version “first”" (the engine's action text, `crates/rapidr-vcs/src/lib.rs:640`).
  - The key store's name inside two sentences ("the macOS Keychain", "memory, for this session only": `crates/rapidr-ai/src/keys.rs`, filling `{store}` and `{source}`), which therefore read "se guarda en the macOS Keychain".
  - Counts with an English thousands comma ("8,070 de 11,468", `Num` in `ide/debug.inc`).
  - The Help pane's own words: its start page ("Put the caret on a word…", "Start with", `crates/rapidr-langsvc/src/help.rs`) and the lines around an entry ("Its default event is…", "No entry for…"). The entries themselves, the Toolbox's tooltips and the components' accessible descriptions are the registry docs' batch.
  - The run-time error messages of a running program (known; the manual's "Not yet" says so).
- **Cut off.** At the default 1280 px the debug panes' group (Variables, Observar, Pila de llamadas, Puntos de interrupción) is narrow, and the dock elides the two longest tab titles ("Pila de llama…", "Puntos de int…"; the whole title is the tooltip). A wider default for that group, or shorter tab words, would fix it (see the doubts above). Everything else the walk found cut off was shortened or made to wrap (Settings' labels and notes, the Assistant's effort chip, the Versions pane's changes line).
- **Menu letters.** Two items of one menu share a letter only where English's items share it too (File: Nuevo proyecto and Versiones; Edit and View have several, View has 28 items for 26 letters). `rapidr i18n studio` flags only the clashes Spanish adds, and there are none.
- **Not translated, on purpose.** Code words and names (`RButton`, `Caption`, `OnClick`…), key names (Ctrl, Enter, Esc, Tab, Space, Backspace), file names and paths, `/commands`, the values `on`/`off`/`True`/`False`, and the program's own text (the form in the designer: "Hello, RapidR", "Greet").
- **Not walked.** The web page's Studio (the scenes are desktop only here: `tools/build_studio_web.sh` and a server make them run there too), the dark and high-contrast themes, a real (non-test) key store, an AI answer from a model (it speaks the language of the question).

