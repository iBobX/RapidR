<!-- translated-from: directx-and-media.md 95e19ada0588bf77 -->
# DirectX, multimedia y dispositivos

Los objetos de DirectX, multimedia y dispositivos se ejecutan en todas las
plataformas y en el navegador. Son implementaciones propias de RapidR de lo que
hacen los programas con las API de Windows para DirectX, MCI, WinINet y puertos
seriales, con los miembros y el comportamiento en los que se apoyan los
programas anteriores. Cada uno tiene también un nombre clásico (`QDXSCREEN`
para `RDXScreen`, y así sucesivamente), que se acepta en todas partes.

## DirectX 2D

| Objeto | |
|---|---|
| `RDXScreen` | una superficie fuera de pantalla que se muestra con `Flip`: `Init`, `AutoSize`, `AllowStretch`, `Fill` (colores de DirectDraw), `Pixel`, texto (`TextOut`, `TextRect`), `Draw` / `StretchDraw` / `CopyRect`, `Rotate`, `View.*`; `OnInitialize` / `OnInitializeSurface` cuando su formulario se muestra por primera vez; `FullScreen` cubre la pantalla con la superficie escalada para que quepa |
| `RDXImageList` | bibliotecas de imágenes de DelphiX (`.DXG`) con su transparencia y sus patrones |
| `RDXTimer` | un temporizador de cuadros: `FrameRate`; `Interval = 0` se dispara una vez por cuadro; `ActiveOnly` solo mientras el programa está activo |
| `RDXSound` | un WAV que se reproduce con su `Frequency`, `Volume` y `Pan` (los decibeles de DirectSound), en ciclo o una sola vez; `Playing`, `Position`, `Size`, `Play`, `Stop` |
| `RDXJoystick` | Los clásicos `Update`, `IsLeft` / `IsRight` / `IsUp` / `IsDown`, `Button(n)`; RapidR añade `Index`, `Connected`, `Name`, los ejes `X Y Z R U V`, `Buttons`, `POV` y los eventos `OnButtonDown`, `OnButtonUp`, `OnMove` |

Joysticks: gamepads mediante gilrs (Windows, macOS), evdev (Linux) y la API
Gamepad del navegador, todos con la misma disposición (un control de Xbox tal
como lo mostraba winmm de Windows). Sin ningún joystick, cada miembro indica “no
conectado” en lugar de dar un error.

## Direct3D (modo retenido)

`RD3DFrame`, `RD3DMeshBuilder`, `RD3DMesh`, `RD3DFace`, `RD3DLight`,
`RD3DTexture`, `RD3DVisual`, `RD3DWrap`, `RD3DVector` y los métodos 3D de
RDXScreen (`CreateFrame`, `CreateMeshBuilder`, `CreateLightRGB`,
`SetCameraPosition`, `CameraLookAt`, `Render`, …): marcos, caras, modelos `.X`
(de texto y binarios — se carga todo modelo del conjunto de pruebas de
compatibilidad), luces (ambiental, direccional, puntual, de foco), la cámara,
texturas, envolturas, sombras y transparencia combinada, con iluminación plana o
de Gouraud.

RapidR dibuja el 3D en la GPU mediante wgpu —Metal en macOS, Direct3D 12 en
Windows (con su adaptador de software WARP donde no hay GPU), Vulkan u OpenGL en
Linux, WebGL 2 en el navegador—, nítido a cualquier escala de pantalla y rápido
(el modelo más grande del conjunto de pruebas de compatibilidad, de 16 000
triángulos, tarda unos 4 ms por cuadro), y luego entrega la imagen al búfer
posterior de la pantalla, de modo que el texto dibujado después de `Render`,
además de `Pixel` y `Flip`, funciona como antes. La niebla se acepta y no tiene
efecto (nunca lo tuvo). El 3D de RapidR coincide con los ejemplos del conjunto de
pruebas de compatibilidad, y sigue la documentación de Direct3D donde estos
callan.

## Sonido y música

- `PLAYWAV file [, options]` y `PLAYSOUND` reproducen archivos WAV, Ogg Vorbis,
  FLAC y MP3 (o recursos `$RESOURCE`); `BEEP` y `SOUND freq, duration`.
- **RMIDI** (nombre anterior `QMIDI`, de `QMidi.inc`) reproduce archivos MIDI
  estándar con la temporización de MCI en una salida MIDI del sistema que
  produzca sonido; donde no hay ninguna (macOS, Linux, navegadores sin Web MIDI)
  los reproduce el sintetizador General MIDI propio de RapidR. Una salida cuenta
  cuando su nombre dice que es un sintetizador (el *GS Wavetable Synth* de
  Windows, FluidSynth, TiMidity …) o, en Windows, cuando es la primera que no es
  un ciclo (loop) ni un bus. Las salidas MIDI propias de una Mac (un bus IAC, una
  sesión de red, un puerto virtual que creó otro programa) y el *Midi Through* de
  Linux solo pasan mensajes a otros programas y no producen sonido, así que una
  canción nunca se envía por sí sola a una de ellas: la reproduce el sintetizador
  de RapidR. Para tocar en un teclado o en un módulo de sonido que hayas
  conectado, nómbralo antes de que se inicie el programa (una parte de su nombre,
  sin distinguir mayúsculas de minúsculas); `builtin` siempre se reproduce en el
  sintetizador de RapidR. Un nombre que ninguna salida tiene se informa en la
  salida de errores y se reproduce en el sintetizador de RapidR.

  ```
  RAPIDR_MIDI_OUT="Roland" rapidr run song.rr     (macOS, Linux)
  set RAPIDR_MIDI_OUT=builtin & rapidr run song.rr  (Windows)
  ```

  Ejemplo: [`examples/media/midi_compose.rr`](../../../examples/media/midi_compose.rr)
  escribe una canción en código (acordes, una línea de bajo, batería en el
  canal 10) y la reproduce. En un navegador, la variable de página
  `RAPIDR_MIDI_OUT` hace lo mismo.
- **RWave** (`QWAVE`, de `QWave.inc`) reproduce archivos WAV, graba desde la
  entrada predeterminada (`New`, `Record`), guarda y elimina.
- **RVideo** (`QVIDEO`, de `QVideo.inc`) reproduce archivos AVI con los
  decodificadores propios de RapidR: sin comprimir, Microsoft RLE, Microsoft
  Video 1, Cinepak y Motion JPEG, con su sonido PCM — en un formulario o en una
  ventana propia.
- **RCDAudio** (`QCDAUDIO`, de `Qcdaudio.inc`) responde como una computadora sin
  unidad de CD.

Los objetos multimedia comparten estos miembros: `Open`, `Close`, `Play`, `Stop`,
`Pause`, `State`, `FileOpen`, `Lenght` (la ortografía clásica), `Error` (los
textos propios de MCI), `Volume`, su `Timer` y `OnChange`. Un programa que
incluye con `$INCLUDE` el archivo de biblioteca clásico obtiene el objeto de
RapidR.

## Sonido creado muestra por muestra: RAudioOut

**RAudioOut** reproduce el sonido que tu programa va creando sobre la marcha, un
número (una *muestra*) a la vez: el zumbador o el chip de sonido de un emulador,
un sintetizador, un tono para una clase de física, un juego de música. El
programa escribe muestras en la cola del flujo; el dispositivo de sonido las
reproduce en un hilo propio y nunca espera al programa.

![El generador de tonos tocando La4: botones de onda, controles deslizantes de frecuencia y volumen y un botón Stop a la izquierda; a la derecha, la pantalla de un osciloscopio con tres ciclos de una onda senoidal y la lectura “440.0 Hz sine, playing: 60 ms queued, 0 gaps”](../images/media/tone-generator.png)

```basic
DIM Speaker AS RAudioOut
DIM phase AS DOUBLE

SUB Feed (Count AS INTEGER)                 ' el flujo pide Count muestras
    DIM buf(Count - 1) AS INTEGER, i AS INTEGER
    FOR i = 0 TO Count - 1
        buf(i) = SIN(phase) * 6000          ' un tono suave de 440 Hz
        phase = phase + 2 * 3.14159265 * 440 / Speaker.SampleRate
    NEXT
    Speaker.Write buf, Count
END SUB

Speaker.OnNeedData = Feed
Speaker.Start
```

- **Muestras.** De forma predeterminada una muestra es un número de 16 bits de
  -32 768 a 32 767, 44 100 de ellas por segundo (`SampleRate`, de 8000 a 96 000).
  `Write` acepta un arreglo (array) de INTEGER (o BYTE, LONG …), una cadena
  binaria (dos bytes por muestra, primero el byte bajo), un `RMemoryStream`
  (desde su `Position`) o un número, con `Count` y `Offset` opcionales;
  `WriteFloat` acepta de -1 a 1 de un arreglo SINGLE o DOUBLE; `WriteSample v`
  escribe una muestra, la forma más fácil de aprender. `Bits = 8` lee bytes de 0
  a 255 (128 es silencio) como los tienen los archivos WAV de 8 bits. Con
  `Channels = 2` las muestras van intercaladas: izquierda, derecha, izquierda …
- **Mantenerlo alimentado.** Mantén en cola unos `BufferMs` (60 ms de forma
  predeterminada) de sonido. Escribe en `OnNeedData(Count)`, que el flujo genera
  en el propio hilo de tu programa cuando la cola baja de `BufferMs` (Count la
  vuelve a llenar), o, en un programa que trabaja un cuadro a la vez, revisa
  `Queued` (ms en espera) desde un `RTimer` y escribe hasta que esté cerca de
  `BufferMs`. El sonido que oyes llega con un retraso de unos `BufferMs`: un
  valor menor responde antes, uno mayor resiste un momento de mucha actividad.
- **Cuando el programa se retrasa**, la cola se vacía: el flujo reproduce
  silencio tras un fundido de salida de 2 ms (sin chasquido), cuenta el hueco en
  `Underruns` y hace entrar el siguiente sonido con un fundido. `Stop` baja el
  sonido de la misma manera; `Volume` (0–100, según el oído: 50 suena la mitad de
  fuerte) se desliza durante 10 ms, así que un control deslizante nunca provoca
  chasquidos. `Enabled = False` pausa; `Clear` descarta lo que espera. `Start` le
  da al dispositivo de sonido 40 ms de silencio antes de tus muestras, de modo
  que unos audífonos que toman una ráfaga al comenzar (los Bluetooth lo hacen)
  nunca cortan el inicio.
- **En todas partes.** Igual en Windows, macOS y Linux, interpretado y nativo, y
  en el navegador. El dispositivo convierte tu `SampleRate` al suyo. Sin un
  dispositivo de sonido (o en un programa compilado sin sonido) el flujo se
  reproduce en silencio, sin perder el ritmo, y `Error` dice por qué.

### Emuladores: sonido a partir de una línea de tiempo de niveles

El sonido más simple de una computadora es un bit conectado a un altavoz, que el
programa invierte en momentos exactos. Un emulador lo reproduce un cuadro a la
vez (examples/media/beeper.rr, y la receta examples/recipes/frame_audio.rr):

1. ejecuta la máquina emulada durante un cuadro (a 50 cuadros por segundo, 1/50
   de sus ciclos), anotando el ciclo de cada escritura al altavoz;
2. para cada una de las muestras del cuadro (44 100 / 50 = 882), qué parte de su
   tiempo estuvo alto el bit: de -1 a +1 según esa proporción. Promediar sobre
   la muestra en lugar de leer el bit una sola vez mantiene el sonido limpio (sin
   el siseo del aliasing);
3. un bloqueador de DC (`out = x - prevX + 0.995 * prevOut`): un altavoz real no
   puede mantener un empuje, así que un bit que se deja en alto vuelve poco a
   poco hacia el centro;
4. escribe las 882 muestras con `Write`.

Un `RTimer` revisa con frecuencia y ejecuta cuadros hasta que hay unos 60 ms en
cola, así que un tic que llega tarde se pone al día y el sonido nunca tiene
huecos. Usa `Interval = 10`: los tics de un `RTimer` llegan en los pasos de
15.6 ms del temporizador del sistema, como siempre (`Interval = 20` solo marca
cada 31 ms, lo que le deja a una cola de 60 ms poco margen para un tic tardío).
El intérprete de RapidR hace un cuadro así en 0.2 a 0.6 ms, una pequeña parte de
los 20 ms de que dispone.

![El zumbador de un bit: arriba, el bit del altavoz como una onda cuadrada durante un cuadro, debajo las 882 muestras del cuadro, y la lectura: la nota, el cuadro hecho en mucho menos de un milisegundo, 75 ms en cola, 0 huecos](../images/media/beeper.png)

### En el navegador

Un navegador reproduce sonido solo después de que la persona haya hecho clic o
presionado una tecla en la página: llama a `Start` desde el `OnClick` de un
botón. Hasta que la página pueda reproducir, `Error` lo dice; el sonido empieza
(y `Error` se borra) en cuanto puede. Mantén la cola un poco más larga allí
(`BufferMs = 80` o más) si la página está ocupada: el navegador reproduce el
flujo entre el trabajo propio de la página.

Los ejemplos: [tone_generator.rr](../../../examples/media/tone_generator.rr)
(una onda senoidal, cuadrada, de diente de sierra o ruido, y un osciloscopio de
las muestras reproducidas), [beeper.rr](../../../examples/media/beeper.rr) (el
altavoz de un bit), [chiptune.rr](../../../examples/media/chiptune.rr) (tres
voces de onda cuadrada que tocan la melodía propia de RapidR) y la receta
[frame_audio.rr](../../../examples/recipes/frame_audio.rr).

## Entrada y salida MIDI: RMIDIPort

**RMIDIPort** conecta tu programa con teclados, controladores, pads de
percusión y sintetizadores MIDI: los muestra, abre una entrada y una salida,
avisa a tu programa con eventos lo que llega, y envía notas, controles y
mensajes exclusivos del sistema (SysEx). No hay punteros, búferes ni llamadas
al sistema que acertar, y funciona igual en Windows, macOS, Linux y el
navegador.

![El monitor MIDI: arriba, la entrada y la salida elegidas; la lista de los mensajes recibidos con su hora, sus bytes en hexadecimal y lo que significan (un SysEx, una nota encendida y una apagada), y abajo la solicitud de identidad lista para enviar](../images/media/midi-monitor.png)

```basic
DIM Midi AS RMIDIPort

SUB NoteOn (Channel AS INTEGER, Note AS INTEGER, Velocity AS INTEGER)
    PRINT "nota "; Note; " en el canal "; Channel; ", velocidad "; Velocity
END SUB

Midi.OnNoteOn = NoteOn
IF Midi.InputCount > 0 THEN
    Midi.OpenInput 0                          ' la primera entrada
ELSE
    PRINT "No hay entrada MIDI: "; Midi.Error
END IF
```

- **Dispositivos.** `InputCount` y `OutputCount` leen las listas del sistema;
  `InputDevice(i)` y `OutputDevice(i)` dan los nombres (0 es el primero);
  `FillInputs List` y `FillOutputs List` los ponen en un `RListBox` o un
  `RComboBox`. `OpenInput` y `OpenOutput` aceptan un número o un nombre: el
  nombre exacto, o el primero que lo contiene (`Midi.OpenInput "keys"` abre
  “USB Keys”). Dos dispositivos con el mismo nombre se distinguen: el segundo
  es “Nombre (2)”. El evento `OnDevicesChanged` se produce cuando se conecta o
  desconecta uno. Un `RMIDIPort` abre una entrada y una salida; crea uno por
  cada teclado que quieras escuchar.
- **Recibir.** El evento `OnMessage(Status, Data1, Data2)` recibe cada mensaje
  corto: notas, controles, cambios de programa, inflexión de tono. Para los
  más comunes hay eventos más sencillos: `OnNoteOn` y `OnNoteOff` (canal de 1
  a 16, nota, velocidad; una nota encendida con velocidad 0 cuenta como
  apagada) y `OnControl` (una perilla, un control deslizante, el pedal de
  sostenido). Los eventos se producen en el hilo de tu programa, en el orden
  en que llegaron los mensajes; `Timestamp` dice cuándo llegó el mensaje (ms
  desde que se abrió la entrada). El reloj de sincronía y el sensado activo,
  que algunos dispositivos envían todo el tiempo, se descartan hasta que
  pongas `IgnoreClock = False` o `IgnoreActiveSensing = False`.
- **Enviar.** `SendNoteOn canal, nota, velocidad`, `SendNoteOff`,
  `SendControl`, `SendProgram`, o `Send status, data1, data2` para cualquier
  mensaje corto. `SendHex "90 3C 64"` envía bytes escritos en hexadecimal.
  Cada uno devuelve True cuando el mensaje salió; si no, `Error` dice por qué
  en palabras simples (un valor mayor que 127, ninguna salida abierta, el
  dispositivo se desconectó).
- **Ráfagas.** Los mensajes esperan en una cola hasta que tu programa los
  toma, así que una ráfaga nunca se pierde; si el programa se queda muy atrás
  (más de 32 768 en espera), `Dropped` cuenta lo que se perdió y el evento
  `OnError` lo avisa.

### SysEx: mensajes exclusivos del sistema

Un mensaje SysEx es un bloque completo de bytes de `F0` a `F7`: volcados de
sonidos, ajustes del dispositivo, la solicitud de identidad a la que un
sintetizador responde con su fabricante y modelo. **SysEx está apagado hasta
que tu programa lo pide: pon `IgnoreSysEx = False` para recibir SysEx.**
Entonces el evento `OnSysEx(Data)` recibe cada mensaje completo, como una
cadena binaria (el `ASC` de cada carácter es un byte), aunque haya llegado en
partes; `Hex(Data)` lo muestra como `F0 7E 7F 06 02 …`. `SendSysEx Data`
envía una cadena binaria, y `SendHex` lo mismo escrito en hexadecimal:

```basic
Midi.IgnoreSysEx = False                      ' recibir SysEx
Midi.OnSysEx = Answer                         ' SUB Answer (Data AS STRING)
Midi.SendHex "F0 7E 7F 06 01 F7"              ' solicitud de identidad: “¿quién eres?”
```

![SysEx con un sintetizador: a la izquierda los dispositivos, a la derecha los mensajes universales y el dispositivo elegido, los bytes F0 7E 7F 06 01 F7, y abajo el registro: la solicitud de identidad enviada, la respuesta recibida y descifrada (fabricante, familia, modelo, versión)](../images/media/midi-sysex.png)

- **Cuidado con SysEx.** Un mensaje SysEx puede cambiar ajustes guardados
  dentro de un dispositivo y, en algunos, incluso reemplazar su firmware.
  Envía solo mensajes que conozcas (del manual del dispositivo) y ejecuta
  programas que envían SysEx solo si confías en ellos.
- **Identificadores de fabricante.** El segundo byte de un mensaje SysEx dice
  de quién es el formato. Los mensajes *universales* del estándar MIDI (`7E`,
  `7F`), como la solicitud de identidad y GM System On, funcionan con
  cualquier dispositivo General MIDI; los ejemplos usan solo esos. Si tu
  programa publicado define un formato SysEx propio, necesita un
  identificador de fabricante de la MIDI Association (revisa sus condiciones
  actuales); el identificador `7D` es solo para pruebas, escuelas y trabajos
  que no se publican.

### En el navegador

La primera vez que el programa muestra o abre un dispositivo, una nota breve
explica lo que quiere y luego el navegador pide a la persona que permita
MIDI. El programa espera la respuesta. SysEx es una pregunta aparte, que se
hace solo cuando el programa pone `IgnoreSysEx = False` o envía un SysEx;
decirle que no deja funcionando las notas y los controles. Un “no” es un
mensaje claro en `Error` (“MIDI access was not allowed in this browser”),
nunca un fallo; después de “Ahora no”, `OpenInput` u `OpenOutput` vuelven a
preguntar.

- MIDI funciona en un navegador solo en una página servida por HTTPS o desde
  tu propia computadora; una página mostrada dentro de un marco necesita
  `allow="midi"` en ese marco. Algunos navegadores no tienen MIDI: entonces
  las listas quedan vacías y `Error` lo dice.
- RapidR no envía a ningún sitio los nombres de los dispositivos de la
  persona. Lo que haga tu propio programa con ellos depende de ti: un nombre
  de dispositivo puede identificar a una persona (“el teclado de Ana”), así
  que no lo envíes a un servidor sin preguntar.

En Linux, MIDI (como el sonido) usa la biblioteca ALSA del sistema
(`libasound2`): si empaquetas tu programa, inclúyela como dependencia y no la
copies dentro del paquete.

Los ejemplos: [midi_monitor.rr](../../../examples/media/midi_monitor.rr) (todo
lo que envía un dispositivo, en hexadecimal y en palabras, y un mensaje
escrito en hexadecimal enviado de vuelta) y
[midi_sysex_port.rr](../../../examples/media/midi_sysex_port.rr) (mensajes
SysEx universales enviados a un sintetizador, su respuesta de identidad
descifrada, una nota y un acorde para comprobar el cable).

## Dispositivos y red

| Objeto | |
|---|---|
| `RComPort` | un puerto serial (nombre anterior `COMPORT`): `Port`, `BaudRate`, `DataBits`, `Parity`, `StopBits`, `Open`, `Close`, `ReadString`, `WriteString`, `OnRxChar`, `OnComError`, … — serial2 en el escritorio (nada que instalar en Linux), Web Serial en el navegador; los extras de RapidR para placas IoT (ListPorts, DTR / RTS, ReadLine, OnLine, OnPortsChanged) en [Puertos seriales y placas IoT](serial-ports.md) |
| `RSocket` (nombre anterior `QSOCKET`), `RServerSocket` | cliente y servidor TCP (el navegador: solo cliente WebSocket); servidores en [Redes](network.md): solo esta computadora (`127.0.0.1`), por líneas, bytes o solicitudes HTTP completas |
| `RHttpServer` | un servidor web en tu programa, en `127.0.0.1`: una API local, un panel de control, un servidor MCP ([Redes](network.md)) |
| `RHttp` | `Get` y `Post` de HTTP(S) |
| `RDownload` | nombre anterior `QDOWNLOAD` (`Qdownload.inc`), con `LeechFile`: un archivo de un servidor web a una variable o a un archivo, con progreso; el puerto 443 es HTTPS, se siguen las redirecciones, y las ventanas del programa siguen funcionando mientras espera |
| `RCGI` | nombre anterior `QCGI` (`qcgi.inc`), el objeto CGI: las variables CGI como propiedades, `Parse`, `Get(Name, Value)` — para programas `$APPTYPE CGI` detrás de un servidor web |

<!-- legal -->
HTTPS usa el TLS del sistema operativo (el framework Security de macOS, SChannel
de Windows, OpenSSL 3 de Linux) y sus certificados: RapidR no compila ninguna
biblioteca TLS dentro de tu programa.

## La bandeja del sistema

Los programas ponen un ícono en el área de notificación de Windows con
`RNotifyIconData` y `Shell_NotifyIcon`, y reciben sus clics en el `WndProc` de su
formulario. RapidR mantiene funcionando en todas partes esa única llamada de
Windows: el ícono se muestra en la barra de menús de macOS, en el área de
notificación de Windows, en la bandeja de Linux (StatusNotifierItem) y en una
franja en la parte inferior derecha de una página web, y los clics llegan a
`WndProc (hWnd, uMsg, wParam, lParam)` como los mensajes de mouse de Windows.
`NI.hIcon = Application.Icon` muestra el ícono del programa. En Windows, el
`WndProc` del formulario también recibe todos los demás mensajes de su ventana
(consulta [Mensajes de Windows](components.md#mensajes-de-windows-el-wndproc-de-un-formulario));
en los demás sistemas, los de la bandeja son los únicos.

## El registro

<!-- legal -->
`RRegistry` es el propio registro de Windows en Windows, como siempre lo fue. En
macOS, Linux y la web es un almacén por usuario que responde de la misma manera
(`~/Library/Application Support/RapidR/registry.reg`,
`~/.config/rapidr/registry.reg`). `RAPIDR_REGISTRY=<archivo>` pone las claves en
ese archivo en todos los sistemas, así que las pruebas nunca tocan el registro
real.
