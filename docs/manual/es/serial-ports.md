<!-- translated-from: serial-ports.md d3ca6a2aeb060944 -->
# Puertos seriales y placas IoT

ESP32, Arduino, Raspberry Pi Pico y la mayoría de las demás placas de
microcontrolador hablan con una computadora a través de un adaptador serial USB.
Un programa de RapidR llega a ellas con **RComPort** (nombres anteriores
`QCOMPORT` y `COMPORT`), además de unos pocos extras de RapidR hechos para estas
placas: los puertos enumerados con sus ID de USB, las líneas DTR / RTS que
reinician una placa, líneas completas de entrada y un evento cuando se conecta o
se desconecta una placa.

Funciona de la misma manera en las compilaciones nativas, en el intérprete
(`rapidr run`) y en el navegador (Chrome y Edge, mediante Web Serial). El ejemplo
[`examples/iot/esp32_monitor.rr`](../../../examples/iot/esp32_monitor.rr) es un
monitor serial completo construido con lo que sigue.

## Abrir un puerto

```basic
DIM Board AS RComPort        ' o QCOMPORT: el mismo componente
Board.Port = "COM3"          ' Windows; /dev/cu.usbserial-0001 (macOS), /dev/ttyUSB0 (Linux)
Board.BaudRate = 115200
Board.Open
IF Board.Connected THEN PRINT "abierto"
```

`DataBits` (8), `Parity` (0: ninguna) y `StopBits` (1) se leen en `Open`.
`OnOpen` se dispara cuando se abre; `OnComError` recibe el mensaje cuando no se
abre (**…El sistema no puede encontrar el archivo especificado.** *(…The system
cannot find the file specified.)* para un puerto que no existe, **…Acceso
denegado.** *(…Access is denied.)* para uno que otro programa tiene abierto).
`Close` lo cierra.

En macOS y Linux, `Port` también puede ser `COMn`: el n-ésimo puerto que enumera
el sistema. En el navegador, `COMn` es el n-ésimo puerto que se le permitió a la
página; un puerto que la página todavía no tiene abre el selector del navegador,
que solo funciona desde un clic (el `Open` de un controlador de `OnClick`).

## Qué puertos hay

```basic
FOR i = 0 TO Board.ListPorts - 1
    PRINT Board.PortName(i), Board.PortDescription(i), HEX$(Board.PortVendorID(i))
NEXT
```

| Miembro | Qué da |
|---|---|
| `ListPorts` | Mira los puertos que hay ahora; devuelve cuántos |
| `PortCount` | Cuántos encontró el último `ListPorts` |
| `PortName(i)` | El nombre que toma `Port` (`i` desde 0) |
| `PortDescription(i)` | Qué es el dispositivo: el nombre que le da el sistema ("CP2102N USB to UART Bridge Controller", "M5stack"), o, si no, el chip que nombran sus ID de USB ("CH340 USB to serial", "USB JTAG/serial debug unit (ESP32)") |
| `PortManufacturer(i)`, `PortSerialNumber(i)` | Tal como los dice el dispositivo USB (el número de serie permite distinguir dos placas idénticas) |
| `PortVendorID(i)`, `PortProductID(i)` | Los ID de USB: `&H10C4` Silicon Labs, `&H1A86` WCH (CH340), `&H0403` FTDI, `&H303A` Espressif; 0 cuando no es USB |
| `FillList(Control)` | Pone los puertos en una lista o en una lista desplegable, en el orden de `PortName`, con seleccionado el puerto que nombra `Port`; devuelve cuántos |

```basic
Board.FillList(Ports)                       ' Ports es una lista desplegable
Board.Port = Board.PortName(Ports.ItemIndex)
```

El escritorio enumera todos los puertos que tiene el sistema: en macOS los
puertos `/dev/cu.*` (sus gemelos `tty.*` esperan una portadora, que una placa
nunca levanta), con los detalles USB de IOKit; en Windows los puertos `COMn` con
los de SetupAPI; en Linux los puertos seriales `/dev/tty*` con los de sysfs. El
navegador enumera solo los puertos que se le permitieron a la página, como
`COM1`, `COM2` …, con sus ID de USB (el navegador no da nombres, así que la
descripción sale de los ID). El navegador responde un momento después de que el
programa crea su primer RComPort, así que una lista tomada en el primer instante
del programa puede salir vacía: `OnPortsChanged` se dispara cuando llegan los
puertos.

## Conectado, desconectado

```basic
Board.OnPortsChanged = PortsChanged
SUB PortsChanged(Added AS STRING, Removed AS STRING)
    IF INSTR(Removed, Board.Port) THEN Board.Close
    Board.FillList(Ports)
END SUB
```

`OnPortsChanged` recibe los nombres que llegaron y los nombres que se fueron (un
CR LF entre dos nombres). El escritorio revisa cerca de una vez por segundo
mientras el programa atiende el evento; el navegador avisa de inmediato.

## Líneas

Las placas imprimen texto una línea a la vez. `ReadLine` da la siguiente línea
completa, sin su final:

```basic
l$ = Board.ReadLine(500)     ' espera hasta 500 ms una línea; "" si no llegó ninguna
```

Sin un tiempo de espera, espera hasta un segundo; `ReadLine(0)` no espera. Una
línea que solo llegó en parte se queda donde está para la siguiente lectura.
`HasLine` es 1 cuando hay una línea completa esperando.

`LineEnd` es lo que termina una línea: `CHR$(10)` salvo que lo establezcas. Con
`CHR$(10)` también se descarta un CR justo antes, así que las líneas `CR LF` (el
`println` de Arduino, la ROM del ESP32) y las `LF` salen limpias. Establece
`LineEnd = CHR$(13)` para un dispositivo que termina las líneas solo con CR, o
cualquier otro texto para otros protocolos.

O deja que las líneas lleguen a ti:

```basic
Board.OnLine = GotLine
SUB GotLine(Received AS STRING)
    Log.AddStrings Received
END SUB
```

`OnLine` se dispara por cada línea completa (el entorno de ejecución revisa cada
50 ms). Una línea que entrega se da por leída: `ReadString` y `ReadLine` ya no la
ven. `OnRxChar` (el clásico: llegaron bytes) se sigue disparando como antes.

`ReadString(Count, Wait)` y `WriteString(Text, Wait)` funcionan como siempre,
para todo lo que no va línea por línea.

## Las líneas del módem: reiniciar una placa

| Miembro | |
|---|---|
| `DTR`, `RTS` | Data Terminal Ready y Request To Send: 1 activada, 0 desactivada. Al leerlas devuelven lo que se estableció. Un cambio se aplica de inmediato en un puerto abierto, y si no, en `Open`. Hasta que un programa las establece siguen a `DcbFlags` (las dos activadas, la manera clásica en que se abre un puerto) |
| `CTS`, `DSR`, `CD`, `RI` | Las líneas que maneja el otro extremo (solo lectura; 0 mientras está cerrado) |
| `SendBreak(ms)` | Mantiene la línea en un estado de break (250 ms sin argumento) |

Las placas ESP32 (y ESP8266, y muchas otras) conectan DTR y RTS, a través de dos
transistores, a los pines **EN** (reinicio) e **IO0** (modo de arranque) del
chip:

| DTR | RTS | EN | IO0 |
|---|---|---|---|
| 1 | 1 | alto (se ejecuta) | alto |
| 0 | 0 | alto (se ejecuta) | alto |
| 0 | 1 | **bajo (en reinicio)** | alto |
| 1 | 0 | alto | **bajo** |

Así que un reinicio normal —la placa arranca de nuevo en su propio programa— es:

```basic
Board.DTR = 0
Board.RTS = 1      ' EN bajo: el chip se detiene
SLEEP 0.1
Board.RTS = 0      ' EN alto, IO0 alto: arranca su programa
```

y la ROM del ESP32 imprime su registro de arranque a 115200 baudios:

```
ets Jun  8 2016 00:22:57

rst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)
configsip: 188777542, SPIWP:0xee
…
entry 0x400805e4
```

luego lo que imprima el programa de la placa (ese vino de un M5StickC Plus). En
cambio, mantener IO0 en bajo mientras EN sube inicia el modo de descarga del
chip: eso es lo que hacen las herramientas de grabación (flashing), y nada en
esta página lo necesita.

Las placas del estilo de Arduino Uno se reinician cuando DTR pasa de desactivada
a activada (un capacitor en el pin de reinicio), y por eso abrir su puerto
reinicia el sketch.

## Según el sistema

- **macOS**: los adaptadores USB aparecen como `/dev/cu.usbserial-…` (FTDI,
  CP210x), `/dev/cu.SLAB_USBtoUART` (el controlador propio de Silicon Labs),
  `/dev/cu.wchusbserial…` (el controlador de WCH) o `/dev/cu.usbmodem…` (placas
  con USB nativo, ESP32-S3 / C3, Pico). Las versiones recientes de macOS
  funcionan con CP210x, CH340 y FTDI sin controladores adicionales.
- **Windows**: `COM3`, `COM4` …; `COM10` y los siguientes también funcionan. Las
  placas CH340 pueden necesitar el controlador de WCH en versiones anteriores de
  Windows.
- **Linux**: `/dev/ttyUSB0` (adaptadores), `/dev/ttyACM0` (USB nativo). Tu
  usuario tiene que estar en el grupo `dialout` (`uucp` en Arch) para abrirlos.
- **El navegador**: Web Serial, en Chrome y Edge (no en Firefox ni en Safari), en
  páginas `https://` o `localhost`. La página le pide al usuario que elija un
  puerto la primera vez; después puede abrirlo de nuevo sin preguntar. Las
  esperas de `ReadLine` y `ReadString` dejan que la página se redibuje mientras
  esperan.

Solo un programa puede tener un puerto abierto a la vez: cierra el monitor del
IDE de Arduino o de `esptool` antes de abrir el puerto desde tu programa, y al
revés.

## Probar sin una placa

Las pruebas nunca abren un dispositivo real. Establece `RAPIDR_TEST_COMPORT` (una
variable de entorno en el escritorio, un `window.RAPIDR_TEST_COMPORT` en la
página, o el propio `ENVIRON` del programa antes de su primer puerto) con puertos
simulados: `COM2:echo` (lo que se escribe vuelve), `COM3:reply:OK\r\n` (cada
escritura recibe respuesta), `COM4:busy` (en uso) y `COM5:esp32` (un ESP32 detrás
de un CP2102N: `ListPorts` lo describe, DTR / RTS lo reinician e imprime un
registro de arranque). Las líneas hacen retorno (loopback) como las de un
conector de prueba: RTS a CTS, DTR a DSR y CD. Cambiar la variable mientras el
programa se ejecuta conecta y desconecta puertos (`OnPortsChanged`).

`tools/esp32_check.sh` revisa a mano una placa real: enumera los puertos, abre la
placa a 115200, la reinicia e imprime el registro de arranque —interpretado y
nativo— y nunca escribe un byte en la placa.
