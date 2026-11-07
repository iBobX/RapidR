# Ofrece las herramientas de tu aplicación a agentes de IA

`RAgentTools` permite que tu aplicación web ofrezca herramientas a agentes de
IA que funcionan en el navegador, como el propio asistente de un navegador o
una extensión. Funciona con navegadores que implementan el borrador WebMCP,
un Draft Community Group Report del W3C Web Machine Learning Community
Group. Es un borrador: no es un estándar del W3C, puede cambiar y hoy solo
algunos navegadores lo ofrecen (mira la tabla). Donde un navegador no lo
ofrece, `RAgentTools.Available` es 0 y tu programa sigue funcionando.

<!-- legal -->
WebMCP es el nombre de un borrador de especificación del W3C Web Machine
Learning Community Group; RapidR usa el nombre solo para decir qué borrador
sigue su función para el navegador. WebMCP es una especificación distinta del
Model Context Protocol; Model Context Protocol® y MCP™ son marcas de LF
Projects, LLC (https://www.lfprojects.org/policies/), y RapidR las usa solo
para decir qué protocolo habla. Los nombres del W3C y de los navegadores
pertenecen a sus dueños (https://www.w3.org/policies/). RapidR es
independiente de todos ellos y ninguno lo hace, lo patrocina ni lo respalda.
RapidR no incluye texto, descripción de interfaz ni código del borrador.

## Una herramienta en cuatro líneas

```rapidr
DIM Agent AS RAgentTools
SUB ToolCalled (Tool AS STRING, Arguments AS STRING, CallId AS INTEGER)
    IF Tool = "add" THEN Agent.Reply CallId, STR$(VAL(Agent.Argument("A")) + VAL(Agent.Argument("B")))
END SUB
Agent.OnToolCall = ToolCalled
Agent.AddTool "add", "Adds two numbers.", "A AS DOUBLE, B AS DOUBLE", "read"
IF Agent.Start = 0 THEN PRINT Agent.Error
```

- **AddTool** ofrece una herramienta: su nombre, una frase que lee la IA para
  decidir cuándo llamarla, sus argumentos escritos como los parámetros de una
  SUB y cómo actúa: `"read"` (solo lee), `"change"`, `"delete"` o
  `"network"`. Añade `untrusted` cuando su respuesta contiene lo que
  escribieron otras personas (una nota, un mensaje, una página web):
  `"read untrusted"`. Al agente se le dice que trate ese texto como datos.
- **Start** ofrece todas las herramientas a los agentes del navegador; una
  herramienta añadida o quitada después se refleja al instante. **Stop** las
  retira todas.
- **OnToolCall** recibe cada llamada, de una en una: la herramienta, sus
  argumentos como JSON (`Argument` lee uno) y el número de la llamada.
  **Reply** responde, ahora o más tarde (guarda el número).
  `Reply CallId, "qué salió mal", 1` marca un fallo: di qué hacer en su
  lugar, el agente actúa según eso.
- Una llamada incorrecta (falta un argumento, una palabra donde va un
  número) nunca llega a tu programa: al agente se le dice lo que necesita la
  herramienta.

El ejemplo [`examples/web/agent_tools.rr`](../../../examples/web/agent_tools.rr)
es una lista de notas que un agente puede leer y ampliar.

## Seguro de forma predeterminada

- **Los cambios preguntan primero.** Con `ConfirmChanges` en True (el valor
  predeterminado), la llamada a una herramienta que no es `"read"` muestra la
  pregunta propia del navegador a quien usa tu aplicación antes de que
  `OnToolCall` la vea. Si dice que no, se le dice al agente. Ponlo en False
  solo si tu programa pregunta por su cuenta (`MESSAGEDLG`).
- **No se muestra nada de tus ventanas** a menos que lo digas.
  `Expose = "read"` añade dos herramientas que los agentes pueden usar sobre
  lo que muestran tus ventanas: `app_components` (el nombre de cada
  componente, su tipo, lo que muestra, si se ve y si está habilitado) y
  `app_get` (una propiedad). `Expose = "write"` añade `app_set` (una
  propiedad cambiada como lo haría tu programa, preguntando primero). Nunca
  un componente que no está en una ventana, nunca una propiedad que guarda
  una credencial.
- **Las respuestas se revisan**: nunca se envía una palabra con forma de
  clave (`sk-…`).
- **Respuestas cortas**: una respuesta de más de `MaxResultSize` caracteres
  se corta, diciendo cuánto quedó fuera. En un navegador son 1500 si no lo
  cambias: quienes hacen los navegadores aconsejan respuestas cortas (un
  agente puede rechazar una larga).
- **Solo esta página**: las herramientas se ofrecen a los agentes del origen
  de tu página, nunca a otros sitios, y solo en una página segura (HTTPS, o
  tu propia computadora mientras pruebas).
- **El navegador no dice quién llama**: `ClientName` es `"browser agent"`.
  Ofrece solo lo que cualquier agente del navegador de la persona puede usar.
  Lo que tu programa ofrece y la privacidad de los datos de quienes lo usan
  son tu responsabilidad.

## Qué navegadores lo ofrecen (2026-11-07)

| Navegador | Hoy |
|---|---|
| Chrome | Una prueba (versiones 149 a 156) para los sitios que se registran; en tu propia computadora, activa `chrome://flags/#enable-webmcp-testing`. |
| Edge | Una prueba desde la versión 150. |
| Firefox | No lo ofrece. |
| Safari | No lo ofrece. |

En todos los demás, `Available` es 0, `Start` devuelve False y `Error` dice
por qué en palabras sencillas; tu aplicación funciona como antes. Escribe del
mismo modo la línea que ven tus usuarios: di qué pueden hacer igualmente.

## En el escritorio

El mismo programa funciona también en el escritorio: allí `RAgentTools`
sirve a las aplicaciones de IA de esta computadora por MCP, como hace
`RMcpServer`. `Start` crea un token cuando `Token` está vacío; `Url` y
`Token` son lo que la persona escribe en su aplicación de IA.
`ConfirmChanges` no puede preguntar desde allí: la llamada a una herramienta
de cambio se rechaza mientras sea True, así que ponlo en False y pregunta en
`OnToolCall` (`MESSAGEDLG`).

## Mira también

- [Deja que las aplicaciones de IA trabajen en Studio](ai-apps.md): RapidR
  Studio ofrece sus propias herramientas del mismo modo, en el escritorio y
  en un navegador.
- [Red](network.md): `RMcpServer`, para programas de escritorio que sirven a
  aplicaciones de IA.
