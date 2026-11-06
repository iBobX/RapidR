# Menús del clic derecho

Haz clic derecho en casi cualquier cosa de RapidR Studio y aparece un menú
corto con lo que puedes hacer con ella ahí: un archivo en el árbol del
proyecto, la pestaña de un documento, el código, un componente de un
formulario, una fila de Propiedades o de Eventos, un chat del asistente,
un problema, una línea de la salida, un valor al depurar, una página de la
Ayuda, la barra de estado, la pestaña de un panel, un proyecto reciente, un
componente del cuadro de herramientas. El
teclado abre los mismos menús, y también un lector de pantalla. Tus propios
programas tienen lo mismo con un evento, `OnContextMenu`, y las listas
pueden mostrar pequeñas acciones con icono en sus filas con `RowActions`.

## Abrir un menú

| Cómo | Dónde se abre el menú |
|---|---|
| Un clic derecho (en un Mac, también un clic con dos dedos o Control-clic) | donde hiciste clic; lo que tocaste se selecciona antes |
| **Mayús+F10** o la **tecla de menú** | debajo de la fila, la pestaña o el componente seleccionado, o debajo del cursor en el código; su primer elemento ya está elegido |
| El «mostrar menú» de un lector de pantalla (VoiceOver: VO+Mayús+M) | igual que el teclado |

En el menú: las flechas se mueven, **Intro** elige, la letra subrayada
elige su elemento, **Esc** cierra el menú y te devuelve adonde estabas.

Todos los menús siguen las mismas reglas, así que siempre sabes dónde mirar:

- **La acción predeterminada va primero, en negrita**: lo que hace allí un
  doble clic o Intro (Abrir para un archivo, Editar el valor para una
  propiedad).
- **Se muestran las teclas** junto a cada elemento, en un color más suave:
  el menú te enseña el atajo.
- **Los grupos van siempre en el mismo orden**: para qué es, crear algo
  nuevo, editar, buscar y mostrar, lo propio del modo (depurar, versiones,
  la alineación del diseñador), Preguntar a RAI y **al final, lo que hace
  perder algo** (Quitar del proyecto, Eliminar).
- **En gris, no escondido**: un elemento que no se puede usar ahora sale en
  gris y dice por qué («No hay nada seleccionado», «El archivo principal se
  queda en el proyecto») en una pequeña ayuda junto al menú cuando te
  detienes en él o llegas con las flechas (un lector de pantalla la lee como
  su descripción), así los elementos nunca cambian de lugar.
- **Preguntar a RAI nunca envía nada por sí solo**: abre el asistente con el
  archivo, el código o el problema citado en el cuadro de la pregunta; tú
  agregas tu pregunta y pulsas Enviar. Con el Bloqueo escolar apagando la
  IA, el elemento no aparece.

## Los menús de RapidR Studio

![Un archivo con clic derecho en el árbol del proyecto: Abrir en negrita, las teclas a la vista, Comparar con, y Quitar del proyecto y Eliminar archivo... al final](../images/context-menus/studio-project-file.png)

| Clic derecho en | Obtienes |
|---|---|
| **Un archivo** del árbol del proyecto | **Abrir**, Abrir el código (un formulario) · Añadir módulo, Agregar archivo existente... · Renombrar (F2), Copiar ruta, Copiar ruta relativa, Mostrar en Finder (Explorador de archivos, Archivos) · Ver el historial (un proyecto con versiones) · Preguntar a RAI sobre este archivo... · Quitar del proyecto (Supr: el archivo se queda en el disco) |
| **Una carpeta** o **un grupo** (Forms, Modules…) | Agregar formulario (un grupo), Añadir módulo, Agregar archivo existente..., Agregar carpeta... · Copiar ruta, Copiar ruta relativa, Mostrar (una carpeta) · Contraer todo |
| **El proyecto** (su propia fila) | Agregar formulario, Añadir módulo, Agregar archivo existente..., Agregar carpeta... · Copiar ruta, Mostrar · Guardar una versión... (o Empezar a versionar este proyecto), Contraer todo · Preguntar a RAI sobre este proyecto... · Opciones del proyecto... · Cerrar el proyecto |
| **La pestaña de un documento** | Cerrar documento (Ctrl+W), Cerrar los demás, Cerrar los de la derecha, Cerrar los guardados, Cerrar todos los documentos · Copiar ruta, Copiar ruta relativa, Mostrar, Mostrar en el proyecto · Dividir a la derecha, Dividir hacia abajo · Historial local de este archivo... |
| **El código** | Ir a la definición (F12), Buscar referencias (Mayús+F12), Cambiar nombre del símbolo... (F2), Corrección rápida... (Ctrl+.) · Cortar, Copiar, Pegar · Alternar punto de interrupción (F9), Ejecutar hasta el cursor (y, en pausa, Establecer la siguiente instrucción, Agregar inspección) · Alternar comentario, Versiones ▸ · Preguntar a RAI sobre este código... |
| **Un componente** de un formulario | **Editar el manejador del evento**, Editar el texto (F2) · Cortar, Copiar, Pegar, Duplicar · Traer al frente, Enviar al fondo, Centrar en el formulario ▸ · Orden de tabulación, Propiedades (F4) · Preguntar a RAI sobre este componente... · Eliminar (Supr) |
| **Varios componentes** | Alinear ▸, Igualar el tamaño ▸, Espaciar por igual ▸ · Cortar, Copiar, Pegar, Duplicar · Traer al frente, Enviar al fondo · Propiedades · Preguntar a RAI · Eliminar |
| **El formulario** mismo | Pegar, Seleccionar todo · Editor de menús, Orden de tabulación · Zoom ▸ · Código (F7), Propiedades · Preguntar a RAI sobre este formulario... |
| **Una propiedad** | **Editar el valor** (Intro), Restablecer el valor predeterminado (Supr) · Copiar el valor, Copiar el nombre, Copiar como código (`GreetButton.Caption = "Greet"`) · Ayuda: Caption (F1) · Preguntar a RAI sobre Caption... |
| **Un evento** | **Ir al manejador** (o Crear el manejador), Quitar el manejador (Supr: el SUB se queda en el código) · Copiar el nombre, Copiar como código · Ayuda (F1) · Preguntar a RAI |
| **Un chat** del asistente | **Abrir** (Intro) · Renombrar (F2), Copiar como Markdown · Eliminar (Supr) |
| **Un problema** (la página Problemas de Salida) | **Ir al problema** (Intro), Corrección rápida... (Ctrl+.), Explicar este problema · Copiar, Copiar todos los problemas · Preguntar a RAI sobre este problema... |
| **Una línea de la salida** (Salida, Compilación) | **Ir a main.rr, línea 12** (una línea que nombra un lugar) · Copiar, Copiar línea, Copiar todo, Seleccionar todo · Ajustar líneas largas ✓ · Borrar |
| **Inmediato** | Copiar, Pegar, Seleccionar todo · Borrar |
| **Un valor** en Variables u Observar (en pausa) | Establecer valor... (F2) · Copiar valor, Copiar expresión, Ver texto... · Añadir a Observar (Variables) o Quitar de Observar (Observar, Supr) |
| **Una llamada** de la Pila de llamadas | **Ir a esta llamada** (Intro) · Copiar, Copiar todo |
| **Un punto de interrupción** | **Ir al punto de interrupción** (Intro), Editar condición... (Mayús+F9), Desactivar (o Activar) el punto de interrupción (Espacio) · Copiar, Copiar todo · Quitar el punto de interrupción (Supr), Eliminar todos los puntos de interrupción (Ctrl+Mayús+F9) |
| **Una página de la Ayuda** | Atrás, Adelante · Copiar, Seleccionar todo · Abrir en el navegador (una página del manual) · Preguntar a RAI sobre esta página... |
| **Un ejemplo de código** de la Ayuda | **Probarlo** (una copia del ejemplo se abre en Studio: F5 lo ejecuta), Copiar el código, Insertar en el cursor · Preguntar a RAI sobre este ejemplo... |
| **La barra de estado** | el mensaje: Copiar el mensaje · el proyecto: Mostrar en Finder, Opciones del proyecto... · Versiones · Lín, Col: Ir a la línea... (Ctrl+G) · las apps de IA: Actividad de aplicaciones de IA, Configuración de aplicaciones de IA... · la etiqueta del Bloqueo escolar: Bloqueo escolar... |
| **La pestaña** o el título de un panel | Cerrar el panel · Flotar (Acoplar, si flota) · Restablecer disposición |
| **Un proyecto reciente** en la página de bienvenida | **Abrir** · Copiar ruta, Mostrar en Finder · Quitar de recientes |
| **Un componente** del cuadro de herramientas | **Agregar al formulario** (Intro) · Ayuda: RButton (F1), Abrir un ejemplo · Preguntar a RAI cómo usar RButton... — un grupo: Contraer todo, Expandir todo |

El archivo del árbol del proyecto tiene además **Comparar con ▸**: la
versión del disco (un archivo abierto con cambios), la última versión (un
proyecto con versiones) u otro archivo que elijas, uno al lado del otro; y,
al final, **Eliminar archivo...**, que pregunta antes y mueve el archivo a
la papelera del sistema (la Papelera de reciclaje en Windows), de donde
puedes recuperarlo —nunca el archivo principal, nunca un archivo fuera de la
carpeta del proyecto, no en el navegador—. **Quitar del proyecto** deja el
archivo en el disco. El componente de un formulario en el árbol y uno en el
diseñador tienen **Ayuda: RButton** (F1).

**Sobre un problema en el código**, el menú empieza con el grupo propio del
problema: su corrección misma cuando el servicio de lenguaje tiene una, en
negrita («Change to Caption» para un `Captoin` mal escrito), luego
Corrección rápida... (Ctrl+.) y **Explicar este problema** (una página de la
Ayuda con el problema, dónde está y la ayuda del nombre sobre el que está);
Preguntar a RAI sobre este problema... lo cita.

![Un problema con clic derecho en la página Problemas de Salida: Ir al problema en negrita](../images/context-menus/studio-problems.png)

![Un miembro mal escrito con clic derecho: su corrección primero, en negrita](../images/context-menus/studio-code-fix.png)

![El menú del panel Observar desde el teclado (Mayús+F10) en pausa, con el aspecto oscuro](../images/context-menus/studio-watch-keyboard-dark.png)

Los nombres exactos de los elementos son los de los menús de Studio en tu
idioma.

![Un componente con clic derecho en el diseñador: primero se selecciona; su manejador de evento en negrita, Eliminar al final](../images/context-menus/studio-designer.png)

![Mayús+F10 en el código: el menú se abre debajo del cursor, con Ir a la definición elegido](../images/context-menus/studio-code-keyboard.png)

Los menús los dibuja RapidR mismo, con el aspecto de Studio (claro,
oscuro, alto contraste), así que aparecen en capturas y grabaciones, y RAI
y las apps de IA pueden ver y elegir sus elementos como tú.

![El menú de una propiedad con el aspecto oscuro](../images/context-menus/studio-inspector-dark.png)

### Los chats del asistente

Haz clic en un chat para abrirlo (las flechas solo se mueven por la lista;
Intro abre). Los chats se agrupan bajo **Hoy**, **Ayer** y **Antes**. La
fila bajo el mouse, y la fila seleccionada, muestran tres botones pequeños:
**Renombrar** (F2: el título se edita en la fila misma, Intro lo conserva,
Esc lo deja), **Eliminar** (Supr) y **Más** (el menú de la fila, también
Mayús+F10, con **Copiar como Markdown**: las preguntas y respuestas del chat
en el portapapeles). Eliminar actúa enseguida y ofrece **Deshacer** junto a
la lista, por eso no pregunta antes. Sin chats todavía, la lista muestra
**Empezar un chat**.

## En tus propios programas

### OnContextMenu

Todo componente que puede tomar el teclado tiene `OnContextMenu (X, Y,
Key)` (de RapidR):

| Argumento | |
|---|---|
| `X`, `Y` | dónde va el menú, en los píxeles del propio componente: donde la persona hizo clic, o debajo del elemento seleccionado cuando lo pidió el teclado |
| `Key` | cómo lo pidió la persona: `0` el mouse, `1` Mayús+F10 o la tecla de menú, `3` un lector de pantalla, `4` el botón Más de una fila (`X`, `Y` su esquina inferior derecha: abre allí el menú alineado a la derecha) |

Muestra tu menú con `PopupAt`:

```basic
CREATE NoteMenu AS RPopupMenu
  SystemMenu = 0                 ' lo dibuja RapidR: tu aspecto, visible para las herramientas de IA
  CREATE OpenItem AS RMenuItem
    Caption = "&Open" + CHR$(9) + "Enter"     ' las teclas después de un Tab
    Default = 1                  ' en negrita: lo que hace un doble clic
  END CREATE
END CREATE

SUB NotesMenu (X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RListBox)
  IF Sender.ContextIndex < 0 THEN EXIT SUB
  Sender.ItemIndex = Sender.ContextIndex      ' un clic derecho no selecciona por sí solo
  IF Key = 4 THEN NoteMenu.Alignment = 1 ELSE NoteMenu.Alignment = 0
  NoteMenu.PopupAt Sender, X, Y
END SUB

Notes.OnContextMenu = NotesMenu
```

- Una lista dice para qué elemento es el menú en **`ContextIndex`**: el del
  clic derecho (un clic derecho no cambia la selección, como antes), el
  seleccionado para el teclado, o la fila cuyo botón Más se pulsó.
- Un componente con un **`PopupMenu`** y sin `OnContextMenu` abre ese menú
  por sí solo, como siempre con el clic derecho, y en un programa de RapidR
  (`.rr`) también desde el teclado; un programa clásico (`.bas`, `.rqw`)
  conserva solo el clic derecho. Asigna `OnContextMenu` y decide tu manejador.
- **`RMenuItem.Default`** dibuja un elemento en negrita; **`RPopupMenu.SystemMenu = 0`**
  hace que RapidR dibuje el menú en todos los sistemas (por defecto macOS y
  Windows muestran el suyo); **`RPopupMenu.AccessibleName`** es cómo lo llama
  un lector de pantalla mientras está abierto («Acciones para orders.rr»).
- Un administrador de paneles dice en qué pestaña de documento fue el clic
  derecho en **`ContextDocument`**, y en qué pestaña o título de panel en
  **`ContextPane`** (**`DockBack Name`** acopla un panel flotante donde
  estaba).
- Un árbol dice en qué nodo en **`ContextIndex`** (un clic derecho tampoco
  lo selecciona); un cuadro de herramientas si fue un componente o un grupo
  en **`ContextKind`**; una consola de salida el lugar que nombra una línea
  con **`PlaceFile(Index)`** y **`PlaceLine(Index)`** (con `ClickedLine`); una
  vista Markdown el bloque de código en **`ContextCode`**; un editor de
  código el problema en un lugar con **`ProblemAt(Line, Column)`**, las
  palabras de su primera corrección con **`FixAt`**, y la aplica con
  **`ApplyFixAt`**.

![El ejemplo: una nota con clic derecho](../images/context-menus/notes-menu.png)

### Un menú desde un botón: el patrón •••

Un menú que cae bajo un botón no necesita `OnContextMenu`: ábrelo desde el
`OnClick` del botón con `PopupAt`, en su borde inferior.

```basic
SUB MoreClick (Sender AS RButton)
  NoteMenu.Alignment = 1                       ' su borde derecho en X
  NoteMenu.PopupAt Sender, Sender.Width, Sender.Height
END SUB
```

El menú sigue abierto cuando el clic termina. Lo mismo vale para un menú que
abre el `OnMouseDown` o el `OnMouseUp` de una etiqueta, o cualquier manejador
que corre con un botón del ratón: **el gesto que abrió un menú nunca elige
nada de él ni lo cierra**. Pulsa una etiqueta cuyo `OnMouseDown` llama a
`PopupAt` y suelta unos píxeles más allá, sobre el primer elemento: el menú
sigue abierto y no se elige nada, como en un menú del sistema. Pulsa, arrastra
hasta un elemento y suelta, y se elige. Después, un clic en un elemento lo
elige, y un clic en cualquier otro sitio cierra el menú (y no hace nada más).

![El botón ••• del ejemplo: su menú bajo él, aún abierto después del clic](../images/context-menus/notes-dots-menu.png)

### RowActions: botones pequeños en las filas de una lista

```basic
Notes.RowActions = "rename:Renombrar (F2),delete:Eliminar (Supr),more:Más acciones (Mayús+F10)"
Notes.OnRowAction = NotesAction

SUB NotesAction (Index AS INTEGER, Action AS STRING, Sender AS RListBox)
  IF Action = "delete" THEN Sender.DelItems Index
END SUB
```

Hasta tres acciones, cada una un nombre de icono de RapidR con su ayuda
después de dos puntos. Se muestran en la fila bajo el mouse y en la fila
seleccionada de una lista que tiene el teclado —nunca solo bajo el mouse,
así el teclado y las pantallas táctiles también las tienen—. **F2** es
`rename` y **Supr** `delete` en la fila seleccionada; **`more`** abre el
`OnContextMenu` de la fila (`Key` 4). Pon también cada acción de fila en el
menú de la lista.

![La fila bajo el mouse con sus botones Renombrar, Eliminar y Más](../images/context-menus/notes-row-actions.png)

### Filas de encabezado y renombrar en la fila

```basic
Notes.AddItems "Hoy", "Comprar leche", "Antes", "Leer el capítulo 3"
Notes.Heading 0, 1                 ' el título de un grupo: en negrita, más suave, nunca seleccionado
Notes.Heading 2, 1
Notes.OnEdited = NoteRenamed

SUB NotesAction (Index AS INTEGER, Action AS STRING, Sender AS RListBox)
  IF Action = "rename" THEN Sender.EditItem Index      ' el editor sobre la fila
END SUB

SUB NoteRenamed (Index AS INTEGER, S AS STRING)
  ' S es lo que se escribió (Intro); cámbialo, o el elemento lo recibe tal cual
END SUB
```

**`Heading(i)`** convierte un elemento en fila de encabezado: un clic no lo
elige, las flechas, Inicio y Fin lo saltan, no tiene acciones de fila y un
lector de pantalla oye un encabezado. **`EditItem i`** abre un editor sobre
el texto del elemento (como en RTreeView): Intro lo conserva
(**`OnEdited`**), Esc lo deja.

### Mover un archivo a la papelera

**`MOVETOTRASH(Path$)`** mueve un archivo o una carpeta a la papelera del
sistema (la Papelera de reciclaje en Windows) y da True; desde allí la
persona aún puede recuperarlo, a diferencia de `KILL`. El Eliminar
archivo... de Studio lo usa ([el lenguaje](language.md#a-la-papelera-movetotrash),
[trash.rr](../../../examples/basics/trash.rr)).

El ejemplo completo es [context_menus.rr](../../../examples/gui/context_menus.rr),
con un botón ••• sobre la lista que abre el mismo menú desde su `OnClick`:
`rapidr run examples/gui/context_menus.rr`.

### Para las herramientas de IA

`app_layout` enumera los elementos de un menú abierto, y `app_click` con
`menu` («NoteMenu > Rename») elige uno, también con un clic derecho: los
menús que dibuja RapidR los ven y los usan RAI y las apps de IA igual que
una persona ([Apps de IA](ai-apps.md), [ojos y manos](app-eyes-and-hands.md)).
