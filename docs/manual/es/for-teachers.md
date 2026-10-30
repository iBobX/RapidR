<!-- translated-from: for-teachers.md 693cfec497ad9c4a -->
# Para docentes: bloqueo escolar

El bloqueo escolar mantiene desactivada la IA de RapidR Studio con una contraseña. Un docente puede activarlo para la cuenta que usan los estudiantes; el equipo de informática de una escuela puede activarlo para todas las cuentas de una computadora, de una manera que los estudiantes no puedan quitar. Todo lo demás en Studio sigue funcionando: escribir código, diseñar formularios, ejecutar y depurar programas, la Ayuda.

Está **desactivado hasta que alguien lo active**. Por ahora puede bloquear dos cosas:

| Qué | Qué ven los estudiantes mientras está bloqueado |
|---|---|
| **IA en Studio (RAI)** | El panel de RAI muestra *Tu docente desactivó la IA* en lugar del chat; no se envía nada a una IA. Configurar RAI y Herramientas > Configuración > IA son de solo lectura. Los comandos de IA de los menús y de la paleta de comandos aparecen atenuados, con el motivo al lado. |
| **Aplicaciones de IA (MCP)** | Las aplicaciones de IA de la computadora no pueden conectarse a Studio: su servidor de aplicaciones de IA no se inicia (y se detiene si estaba en ejecución). Herramientas > Configuración > Aplicaciones de IA es de solo lectura, con su interruptor desactivado y atenuado. |

<!-- legal -->
La barra de estado muestra una pequeña ficha **Bloqueo escolar** mientras el bloqueo está activado. Y si Studio llega a cerrarse de forma inesperada, su informe se queda en la computadora sin la franja que ofrece informarlo en GitHub (una cuenta de GitHub es para personas de 13 años o más).

<!-- legal -->
El bloqueo escolar no recopila nada sobre estudiantes ni docentes y no envía nada a ninguna parte: guarda un solo valor de configuración en la computadora (más abajo).

## Activarlo

1. **Herramientas > Bloqueo escolar…**
2. Deja marcadas las dos casillas bajo *Qué bloquear* (o desmarca una).
<!-- legal -->
3. Escribe una contraseña dos veces: al menos 6 caracteres. La línea debajo de los campos dice qué tan fácil es de adivinar; una más larga, con números o símbolos, es más segura.
4. **Bloquear**.

![Herramientas > Bloqueo escolar: qué bloquear (“AI in Studio”, “AI apps”), la contraseña escrita dos veces, su seguridad (“Fair”), “Lock” y “Cancel”](../images/for-teachers/turn-on.png)

A partir de entonces, cada vez que Studio se inicia en esta cuenta, la IA está desactivada:

![El panel de RAI con un candado y las palabras “AI is turned off by your teacher”, “RAI and AI apps are off in RapidR Studio on this computer” y un botón “Unlock”](../images/for-teachers/card.png)

![La ficha “School Lock” de la barra de estado, junto a la línea y la columna](../images/for-teachers/chip.png)

![Configuración > IA con una nota arriba, “AI is turned off by your teacher”, y todas las opciones atenuadas](../images/for-teachers/settings.png)

![Configuración > Aplicaciones de IA: “Let AI apps use RapidR Studio” sin marcar y atenuado, la misma nota debajo](../images/for-teachers/apps-settings.png)

## Desbloquearlo para una clase

**Desbloquear…** (en la tarjeta de RAI o en una nota) o **Herramientas > Bloqueo escolar…** pide la contraseña:

- **Desbloquear hasta cerrar Studio**: la IA vuelve a funcionar hasta que se cierre Studio. La próxima vez que se inicie, vuelve a estar bloqueada. Mientras tanto, la ficha dice *Bloqueo escolar: desbloqueado*; **Herramientas > Bloqueo escolar… > Volver a bloquear** lo bloquea de inmediato.
- **Desactivar el bloqueo escolar**: el bloqueo termina en esta cuenta hasta que alguien lo active otra vez. También pide la contraseña.

<!-- legal -->
Cuando una contraseña es incorrecta, Studio lo dice con claridad e indica los intentos que quedan. Después de **5 contraseñas incorrectas seguidas**, Studio espera **un minuto** antes de comprobar otra y muestra el tiempo que falta. Cerrar y abrir Studio no reinicia la cuenta.

![Desbloquear: “That isn't the password. 4 more tries before a one-minute wait”](../images/for-teachers/unlock.png)

![Después de cinco contraseñas incorrectas: “Too many wrong passwords. Try again in 0:54”, con los botones atenuados hasta entonces](../images/for-teachers/wait.png)

## La contraseña

<!-- legal -->
RapidR nunca guarda la contraseña en sí. Guarda una forma codificada de un solo sentido (un *hash* con sal: PBKDF2-HMAC-SHA256, 600 000 rondas), de la que no se puede leer la contraseña. Solo quien escribe la contraseña correcta puede desbloquear.

<!-- legal -->
No la compartas: cualquiera que la conozca puede desbloquear hasta que se cierre Studio.

<!-- legal -->
**¿La olvidaste?** En una sola cuenta, el bloqueo es una configuración de Studio: con Studio cerrado, quita el valor llamado `SchoolLock` bajo `Software\RapidR\Studio` en la configuración de Studio y luego activa el bloqueo escolar otra vez con una contraseña nueva:

| Sistema | Dónde está la configuración de Studio |
|---|---|
| Windows | el registro, `HKEY_CURRENT_USER\Software\RapidR\Studio` (Editor del Registro) |
| macOS | `~/Library/Application Support/RapidR/registry.reg` (un archivo de texto: borra la línea `"SchoolLock"=…`) |
| Linux | `~/.config/rapidr/registry.reg` (lo mismo) |
| Studio en un navegador | el almacenamiento de la página: borra los datos del sitio en el navegador |

<!-- legal -->
Un bloqueo establecido por la política de la computadora (más abajo) solo lo cambia el equipo de informática.

## ¿Qué tan fuerte es?

<!-- legal -->
- **En una sola cuenta** (Herramientas > Bloqueo escolar), el bloqueo es una herramienta de aula, no un muro: quien pueda cambiar los archivos de esa cuenta —como hacen los pasos de *¿La olvidaste?*— puede quitarlo. Evita que un estudiante active la IA sin querer o por capricho; no detiene a alguien decidido que sepa dónde guarda Studio su configuración.
<!-- legal -->
- **La política de la computadora** (siguiente sección) se guarda en un lugar que solo un administrador puede cambiar. Los estudiantes no pueden quitarla, y Studio solo puede desbloquearla hasta que se cierre, con la contraseña del equipo de informática; nunca desactivarla.
<!-- legal -->
- **Studio en un navegador** guarda el bloqueo en el almacenamiento de la página, solo para ese navegador en esa computadora. Cualquiera puede borrar los datos de un sitio en su navegador, y eso quita el bloqueo: en un navegador es un recordatorio, no un bloqueo. No hay política de la computadora para un navegador.
<!-- legal -->
- El bloqueo escolar se refiere a RapidR Studio. No bloquea sitios web de IA ni otros programas de la computadora, y los programas que los estudiantes escriben con RapidR aún pueden usar el componente `RAI` con su propia clave, como cualquier programa.

## Para el equipo de informática de la escuela: la política de la computadora

<!-- legal -->
Un archivo de política activa el bloqueo escolar para todas las cuentas de una computadora. Colócalo donde solo un administrador pueda escribir:

| Sistema | El archivo de política |
|---|---|
| Windows | `%ProgramData%\RapidR\policy.toml` (normalmente `C:\ProgramData\RapidR\policy.toml`) |
| macOS | `/Library/Application Support/RapidR/policy.toml` |
| Linux | `/etc/rapidr/policy.toml` |

Genera la línea de la contraseña con `rapidr school-lock hash` (pide la contraseña dos veces, sin mostrarla, e imprime una línea; con `--stdin` la lee de un script):

```text
$ rapidr school-lock hash
Password:
Type it again:
password_hash = "pbkdf2-sha256$600000$5c1f…$9a2e…"
```

Luego escribe el archivo, pegando esa línea:

```toml
# RapidR Studio: bloqueo escolar para todas las cuentas de esta computadora
school_lock = true
locked = ["ai", "ai_apps"]     # "ai": RAI en Studio; "ai_apps": aplicaciones de IA (MCP)
password_hash = "pbkdf2-sha256$600000$5c1f…$9a2e…"
```

Compruébalo con `rapidr school-lock check` (lee el archivo de la computadora, o uno que indiques, y dice qué bloquea):

```text
$ rapidr school-lock check
/Library/Application Support/RapidR/policy.toml: School Lock is on for every user of this computer.
Locked: RAI in Studio (ai), AI apps, MCP (ai_apps).
Unlock: with the password password_hash was made from, until Studio closes. It can't be turned off in Studio.
```

Cómo la lee Studio:

<!-- legal -->
- **La política manda.** Mientras bloquea, el bloqueo escolar propio de cada usuario no importa, y Studio ofrece solo *Desbloquear hasta cerrar Studio* (con la contraseña de la política) y *Volver a bloquear*; nunca *Desactivar el bloqueo escolar*. Los estudiantes ven *Tu escuela desactivó la IA*.
<!-- legal -->
- **Sin `password_hash`**, nadie puede desbloquearlo en Studio.
<!-- legal -->
- **Un archivo que Studio no puede leer** (un TOML no válido, un valor incorrecto, un hash dañado) bloquea todas las funciones sin forma de desbloquear, y Studio dice por qué en su panel Salida: falla hacia el lado seguro. Corrige el archivo para cambiar eso.
<!-- legal -->
- `school_lock = false` significa que la computadora no pone ningún bloqueo: el bloqueo escolar de cada cuenta sigue vigente. Si omites `locked`, se bloquean las dos funciones.
<!-- legal -->
- Todas las cuentas deben poder leer el archivo (Studio se ejecuta como el usuario) y solo los administradores pueden cambiarlo: legible para todos, modificable solo por administradores. No contiene ninguna contraseña, solo su hash.

![El bloqueo escolar establecido por la política de la computadora: “AI is turned off by your school”, el campo de la contraseña, “Unlock Until Studio Closes” y “Cancel”, sin “Turn School Lock Off”](../images/for-teachers/policy.png)

## Preguntas

<!-- legal -->
**¿El bloqueo escolar sabe quiénes son mis estudiantes?** No. No guarda nombres, cuentas ni registros de personas; solo la configuración del bloqueo (activado, qué bloquea, el hash de la contraseña) y la cuenta de contraseñas incorrectas, en la computadora.

<!-- legal -->
**¿Puede un estudiante ver la contraseña en la configuración?** No: solo se guarda el hash, y la contraseña no se puede deducir de él. Una contraseña corta o común aún puede adivinarse probando, y por eso hay una espera de un minuto después de cinco intentos incorrectos.

**¿Y los chats anteriores de RAI?** Se quedan en la computadora como estaban; se muestran otra vez cuando se desbloquea la IA.

**¿Más funciones para bloquear?** El bloqueo escolar está hecho para bloquear más partes de Studio más adelante; por ahora es la IA.
