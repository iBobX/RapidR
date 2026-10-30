<!-- translated-from: why-rapidr.md ef2c06eac22f124f -->
# Por qué RapidR

RapidR es un hogar moderno para los programas BASIC: escribes un solo código
fuente y se ejecuta en las computadoras de hoy —Windows, macOS, Linux y la
web— con un aspecto moderno, accesibilidad, herramientas de datos y una
inteligencia artificial (IA) que nunca tiene tus claves.

Esta página dice en qué destaca RapidR. **Cada número que aparece en ella se
midió, y las condiciones están a su lado**; los programas que los midieron están
en el repositorio (`tools/bench/`), así que puedes ejecutarlos en tu propia
computadora y obtener tus propios números. Donde algo no se ha medido, esta
página no afirma nada.

## Datos: una tabla de un millón de filas en un parpadeo

`RDataFrame` de RapidR es un motor propio, escrito para RapidR en Rust puro:
columnas de números guardadas como números, trabajos grandes repartidos entre
todos los núcleos de la computadora. El mismo motor se ejecuta en los programas
nativos, en el intérprete y en una página web, así que una respuesta es la misma
en todas partes.

Medido en una tabla que todos obtienen de la misma manera: 1 000 000 de filas,
cinco columnas (un archivo CSV de 36.6 MB), hecha por el mismo generador para
cada herramienta. Tiempos en milisegundos, la mediana de varias ejecuciones, en
un Apple M4 Max (16 núcleos) con macOS, compilaciones release, 15 de octubre de
2026 — la Mac estaba ocupada con otros trabajos (carga promedio de 20 a 30), así
que, en todo caso, las cifras quedan del lado lento. pandas es la versión 3.0.1,
que lee el CSV desde la memoria con su motor predeterminado.

| Paso sobre 1 000 000 de filas | RapidR | pandas 3.0.1 |
|---|--:|--:|
| carga (de texto CSV a una tabla) | 11.8 | 287.6 |
| filtro (`salary > 75000`, la mitad de las filas) | 2.9 | 10.0 |
| ordenación (por salario, de forma estable) | 10.2 | 149.2 |
| agrupación por departamento, el promedio de los números | 4.8 | 22.1 |
| unión con una tabla pequeña | 3.4 | 50.4 |
| descripción (conteo, promedio, desviación estándar, cuartiles …) | 10.1 | 28.5 |
| guardado (de la tabla a texto CSV) | 7.3 | 615.5 |
| **carga + filtro + ordenación + agrupación** | **22.7** | **371.6** |

- **Diez millones de filas** (375 MB de CSV): los mismos cuatro pasos tardan
  **0.27 s** (273 ms) en RapidR; pandas tarda 5.8 s en la misma Mac.
- **Un programa de RapidR es tan rápido como su motor**: los mismos cuatro pasos
  escritos en BASIC tardaron 24.5 ms con `rapidr run` y 25.3 ms compilados como
  nativo (un millón de filas, con la lectura del archivo incluida).
- **En un navegador**, donde una página recibe un solo núcleo: los mismos cuatro
  pasos tardan 116 ms para un millón de filas en el motor con un solo hilo, y
  Chrome en la misma Mac carga el millón de filas en unos 0.15 s, las filtra en
  10 ms, las ordena en 60 ms y las agrupa en unos 35 ms
  ([Ciencia de datos con RapidR](data-science-with-rapidr.md#5-a-million-rows)).
- El motor responde exactamente como el anterior (comprobado contra él con
  decenas de miles de programas aleatorios): más rápido, nunca distinto.

La prueba de rendimiento, su generador de tablas y los presupuestos que una
compilación tiene que cumplir: [tools/bench/README.md](../../../tools/bench/README.md).
Cómo usar datos en un programa: [Ciencia de datos](data-science.md) y el
recorrido paso a paso [Ciencia de datos con RapidR](data-science-with-rapidr.md),
donde una tabla de un archivo se muestra en vivo en el diseñador antes de que
escribas una línea de código.

## Una máquina virtual ligera y rápida

El RapidR Runtime —lo que ejecuta tu programa cuando no es una compilación
nativa— es una máquina virtual de bytecode hecha para ser rápida, pequeña y
silenciosa.

| Medido el 15 de octubre de 2026 | Una Mac (Apple silicon) | Linux en una máquina virtual, un núcleo, 2 GB |
|---|--:|--:|
| Arranque: compilar `hello.rr` y ejecutarlo (tiempo transcurrido) | 6.5 ms | 2.4 ms |
| Un ciclo de 10 millones de iteraciones | 0.6 s | 1.29 s |
| …con el depurador, con un punto de interrupción puesto | 0.7 s | 1.33 s |
| Memoria de un programa pequeño y de ese ciclo | 12 MB | — |
| Una ventana que espera eventos, con el depurador | 0% de CPU | 0% de CPU |
| RapidR Studio con ese programa en ejecución | 0.25% de un núcleo | 0.25% de un núcleo |

<!-- legal -->
(Los tiempos son tiempo de CPU, el de la ejecución más rápida de varias, de
`tools/bench/vm_bench` (el ciclo de la Mac: 0.61 s en la primera ejecución,
0.58 s al medirlo de nuevo para esta página), que además guarda los otros
presupuestos en `tools/bench/vm_budgets.json`; si no se cumple un presupuesto,
fallan las comprobaciones de la versión. Un programa que espera sus eventos no
hace sondeos (*polling*): nada lo despierta hasta que algo ocurre, así que la
batería de una laptop y el único núcleo de una Raspberry Pi no se gastan
esperando.)

**El depurador también es rápido.** Un paso —F10, hasta que la línea nueva y sus
valores están en la pantalla— tardó 14 ms en RapidR Studio de escritorio y 19 ms
en Studio en el navegador, en una Mac sin otro trabajo en marcha (la referencia
registrada de `tests/studio_perf.mjs`, la mediana de tres). Medido de nuevo para
esta página, con otros trabajos en ejecución, tardó 19 ms y 29 ms (con una carga
promedio de 9), y 21 ms y 44 ms (con una carga promedio de más de 30); el
presupuesto es de 50 ms. El propio Studio se inicia en 0.28 s en esa máquina
virtual de Linux.

## Imágenes a partir de datos: una pantalla completa en una décima de milisegundo

Un programa que dibuja una imagen a partir de números —la pantalla de un
emulador, un filtro de imágenes, un mapa de calor, un juego de la vida— pone la
imagen completa en un mapa de bits con una sola llamada: `SetPixels`,
`SetPixels8` (números de paleta) o `GetPixels` para leerla de vuelta
([Dibujo: lienzos, mapas de bits y píxeles](graphics.md#drawing-many-pixels-fast)).
Una imagen de 256 x 192 (49 152 píxeles) puesta en un mapa de bits y dibujada al
doble de tamaño, en milisegundos, en un Apple M4 Max con macOS, compilaciones
release, 30 de octubre de 2026, con la Mac ocupada con otros trabajos (carga
promedio de 20 a 40):

| Imagen de 256 x 192, dibujada al doble de tamaño | el intérprete | compilación nativa | web (Chromium) |
|---|--:|--:|--:|
| `Pset` para cada píxel | 77.7 | 62.0 | 83 |
| **`SetPixels8` + `StretchDraw`** | **0.087** | **0.090** | **0.127** |

Un cuadro a 50 por segundo dura 20 ms, así que la imagen cuesta menos de medio
por ciento del cuadro. El programa es `tools/bench/pixels.rr` (y
`tests/web_pixels_bench.mjs` en un navegador);
[pixels_fast.rr](../../../examples/graphics/pixels_fast.rr) dibuja un plasma y el
juego de la vida a 50 cuadros por segundo y dice, debajo de la imagen, cuánto
tardó cada paso. En [tools/bench/README.md](../../../tools/bench/README.md)
también se mide el mismo cuadro en una pantalla de alta densidad de píxeles (2x
y 3x).

## Un programa, todos los sistemas

Escribes un programa una vez. Se ejecuta como ejecutable nativo, en el intérprete
y en un navegador web, en Windows, macOS y Linux — y **se comprueba que se
comporte igual**:

- Un **conjunto de pruebas de conformidad de más de 250 programas**, cada uno con
  su salida esperada, se ejecuta en los tres entornos de ejecución.
- RapidR dibuja todas las ventanas por sí mismo (su núcleo de interfaz de
  usuario), así que un programa no cambia con el sistema en el que se ejecuta. Las
  pruebas **capturan cada ventana de los programas de prueba de interfaz gráfica
  en el escritorio y en un navegador, y exigen que las dos imágenes coincidan byte
  por byte y que sus árboles de accesibilidad sean iguales** (más de cien casos,
  cada uno a escala de pantalla 1× y 2×). Las ventanas propias de RapidR Studio
  coinciden de la misma manera.
- Lo que las personas esperan de su sistema sigue siendo del sistema: la barra de
  menús de macOS, los cuadros de diálogo Abrir y Guardar, el portapapeles, el
  ícono de la bandeja.
<!-- legal -->
- Una compilación web es un `.zip` estático para cualquier alojamiento web: sin
  programa de servidor, sin contactar nada a menos que lo haga el programa
  ([La web](web.md)).

## Los programas anteriores se ejecutan sin cambios

Los programas anteriores siguen haciendo lo que hacían, y los extras de RapidR
son aditivos. Un conjunto de pruebas de compatibilidad con 428 programas de
ejemplo anteriores vigila, programa por programa, y los resultados se publican
completos; lo que se ejecuta, lo que difiere y lo que RapidR no hace (DLL de 32
bits, OLE) está en una sola página: [Diferencias y extensiones](differences.md).
Para pasar un programa anterior a los nombres de componentes propios de RapidR,
[impórtalo](importing-rapidq.md): una copia, comprobada para que se compile al
mismo bytecode, con un informe.

## Accesible sin esfuerzo

Todo programa se puede usar con el teclado y con un lector de pantalla, sin que
quien lo escribe tenga que hacer nada: cada ventana publica un árbol de
accesibilidad (roles, nombres, valores, estados, acciones) para VoiceOver en
macOS, Narrator y NVDA en Windows y AT-SPI en Linux; en un navegador, un espejo
ARIA del mismo árbol. El contraste del texto cumple WCAG AA en todos los temas y
AAA en el de contraste alto; el aspecto sigue la configuración clara, oscura o de
contraste alto del sistema. El texto enriquecido llega a un lector de pantalla
con su formato, y el propio RapidR Studio está hecho de la misma manera
([Componentes y objetos](components.md#accessibility)).

## Seguro por diseño

<!-- legal -->
- **Las compilaciones web llevan una Content-Security-Policy** hecha a partir de
  lo que usa el programa: sin scripts en línea, otros servidores solo para los
  componentes de red y las direcciones que el programa escribe de forma explícita
  ([Compilar aplicaciones y sus íconos](building-apps.md#build-a-web-app)).
- **Un valor incorrecto es un error, no un cierre inesperado**: las funciones de
  memoria (`VARPTR`, `PEEK`, `POKE`) trabajan sobre vistas de los datos del propio
  programa y rechazan cualquier otra dirección; los valores hostiles detienen el
  programa con un mensaje y una línea.
- Los **parámetros SQL** se enlazan, nunca se pegan: el texto que escribe quien
  usa el programa no puede cambiar una instrucción ([Bases de datos](databases.md)).
<!-- legal -->
- **Ninguna biblioteca TLS se compila dentro de tu programa**: HTTPS usa el TLS y
  los certificados del propio sistema operativo. (La poca criptografía estándar
  que sí contiene un programa está enumerada en
  [LEGAL.md](../../../LEGAL.md).)
<!-- legal -->
- **Un programa descargado pregunta antes de ejecutarse**, una vez por archivo.
- La web se audita con el mismo rigor que el escritorio:
  [la auditoría de seguridad](../../security-audit.md) y
  [la política para reportar un problema](../../../SECURITY.md).

## IA, con tus claves nunca en tus programas

<!-- legal -->
RapidR Studio tiene un asistente de IA que permanece **desactivado hasta que lo
activas**, y el componente `RAI` les da a tus propios programas la misma IA —
Anthropic (Claude), OpenAI, DeepSeek o cualquier servidor compatible con
OpenAI, incluido un modelo en tu propia computadora que no envía nada al
exterior.

- El asistente solo **propone**: sus cambios son diferencias que aceptas o
  rechazas, un paso de deshacer cada una, y un programa nuevo se muestra en una
  vista previa antes de que se escriba nada.
<!-- legal -->
- **Una clave nunca se escribe en un proyecto, en una compilación, en un
  registro ni en una captura de pantalla**: vive en el llavero de tu sistema, o
  se le pide a quien usa el programa cuando este se ejecuta; una compilación que
  encuentra una en el código fuente de un programa te avisa.
<!-- legal -->
- **Puedes ver todo lo que se envía**: “Qué se envió” muestra la última
  solicitud, y cada solicitud y decisión va a un registro de auditoría en tu
  computadora. RapidR no tiene telemetría.
<!-- legal -->
- Es barata a propósito: una solicitud “Hazme un programa de formulario de
  saludo” usó entre 7600 y 22 000 *tokens* (fragmentos de palabras) enviados (de
  un tercio a dos tercios desde la caché del proveedor) con el modelo rápido de
  cada proveedor, medido el 15 de octubre de 2026 — una fracción de centavo.

[El asistente de IA y RAI](ai-assistant.md).

## Lo que esta página no afirma

- Los números de velocidad son de una Mac (y de una máquina virtual de Linux
  pequeña); tu computadora será distinta, y por eso los scripts están en el
  repositorio.
- Un navegador le da a una página un solo núcleo, así que el motor de datos es
  más lento allí que en el escritorio (números más arriba).
- Las llamadas a las bibliotecas propias de Windows (`DECLARE … LIB "user32"`)
  funcionan solo en Windows; RapidR no finge ser Windows
  ([Diferencias y extensiones](differences.md)).
- RapidR es una Preview. La hoja de ruta enumera lo que aún no está:
  [ROADMAP.md](../../../ROADMAP.md).
