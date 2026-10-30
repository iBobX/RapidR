<!-- translated-from: data-science-with-rapidr.md 0c3f55244fb9fde5 -->
# Ciencia de datos con RapidR

Este capítulo es un comienzo paso a paso: una tabla de un archivo en un
formulario, en vivo en el diseñador, sin escribir código — luego el código que
la filtra, la ordena y la agrupa, un millón de filas y JSON. Todo lo de aquí
funciona de la misma manera en una aplicación nativa, en una interpretada y en
un navegador web.

Los componentes (todos propios de RapidR):

| Componente | Qué es |
|---|---|
| `RCSV` | un archivo CSV como fuente de datos: establece `FileName` y su tabla está lista |
| `RJson` | un documento JSON — sus registros también son una fuente de datos |
| `RDataFrame` | una tabla que puedes filtrar, ordenar, agrupar, unir y resumir (consulta [Ciencia de datos](data-science.md)); su `Source` toma las filas de un `RCSV`, un `RJson` u otro DataFrame |
| `RDataSource` | la fila actual de un conjunto de datos, compartida por los controles enlazados a él |
| `RDBGrid` | una cuadrícula que muestra un conjunto de datos directamente desde su tabla, un millón de filas tan fácilmente como diez |
| `REdit`, `RLabel` | con `DataSource` y `DataField`, muestran (y un campo de texto cambia) una columna de la fila actual |

Son *componentes vinculados*: los conectas con propiedades, en el diseñador o en
código, y se mantienen al día entre sí.

## 1. Una tabla en un formulario, en el diseñador

Abre [`examples/data/linked_data.rr`](../../../examples/data/linked_data.rr) en
RapidR Studio (la página de bienvenida lo muestra como **linked_data.rr**, “CSV,
grid and edits, linked”). El diseñador muestra el formulario **con los datos ya
dentro**: la cuadrícula enumera las filas de `staff.csv`, y los campos de texto y
las etiquetas muestran los valores de la primera fila. Todavía no se ejecutó nada
— el diseñador lee el archivo por sí mismo.

![linked_data.rr en el diseñador de RapidR Studio: el RDBGrid muestra las ocho filas de staff.csv, los campos de texto Name y Department muestran Alice y Engineering, las etiquetas Salary y Rating 85000 y 4.5](../images/data/linked-designer.png)

Para construirlo tú, en un formulario nuevo:

1. Del grupo **Datos** de la caja de herramientas, suelta un **RCSV** en el
   formulario. No es visible cuando el programa se ejecuta, así que va a la
   bandeja bajo el formulario. En el inspector establece su **FileName** en un
   archivo CSV de tu proyecto, p. ej. `staff.csv`.
2. Suelta un **RDBGrid** en el formulario y establece su **DataSource** en el
   RCSV (el inspector ofrece los componentes que encajan). Aparecen las filas.
3. Suelta un **REdit**; establece también su **DataSource** en el RCSV, y su
   **DataField** en el nombre de una columna, p. ej. `Name` — la lista de
   DataField (su flecha, o F4) ofrece las columnas del archivo. El campo de texto
   muestra el nombre de la primera fila.

   ![La lista DataField del inspector: las columnas name, age y city del conjunto de datos al que está enlazado el campo de texto](../images/data/datafield-columns.png)

![Un formulario nuevo creado en el diseñador: CSV1 (un RCSV) en la bandeja bajo el formulario, un RDBGrid que muestra las filas de staff.csv y un REdit que muestra Alice; el inspector, filtrado por “Data”, muestra el DataField Name y el DataSource CSV1 del campo de texto](../images/data/linked-build.png)

El código que escribe el diseñador es BASIC sencillo:

```basic
CREATE Form AS RForm
    CREATE Staff AS RCSV
        FileName = "staff.csv"
    END CREATE
    CREATE Grid AS RDBGrid
        Left = 10: Top = 44: Width = 610: Height = 230
        DataSource = Staff
    END CREATE
    CREATE NameEdit AS REdit
        Left = 110: Top = 286: Width = 200
        DataSource = Staff: DataField = "Name"
    END CREATE
END CREATE
Form.ShowModal
```

En tiempo de diseño el diseñador lee solo los archivos que están dentro de tu
proyecto (junto al programa), nunca escribe, y muestra hasta un millón de filas.
RapidR Studio en la web hace lo mismo, con los archivos del proyecto en la
página:

![El mismo ejemplo en RapidR Studio en la web: el diseñador muestra las filas de staff.csv en la cuadrícula y los valores de Alice en los campos de texto y las etiquetas; el árbol del proyecto enumera staff.csv bajo Data, la Estructura Staff: RCSV, Team: RDataFrame, Cursor: RDataSource](../images/data/linked-designer-web.png)

## 2. Ejecútalo: la fila actual

Presiona **F5**. Haz clic en una fila de la cuadrícula: el campo de texto muestra
el nombre de esa fila. La cuadrícula y el campo de texto comparten la **fila
actual** del RCSV. Escribe un nombre nuevo en el campo de texto y la cuadrícula
lo muestra de inmediato — la tabla cambió. Haz clic en el **encabezado** de una
columna para ordenar la cuadrícula por ella (una flecha muestra el orden); haz
clic de nuevo para el otro sentido, una tercera vez para el orden propio del
archivo. Ordenar la cuadrícula solo cambia lo que ella muestra; la tabla en sí
conserva su orden (los números de la izquierda son el número de cada fila en la
tabla).

![linked_data.rr en ejecución: la cuadrícula ordenada por Salary, de mayor a menor (la flecha en su encabezado), con la fila de Diana como actual — los campos de texto y las etiquetas de abajo muestran sus valores](../images/data/linked-running.png)

El teclado también funciona: las flechas, Page Up, Page Down, Home y End mueven la
fila actual; la rueda del mouse y la barra de desplazamiento desplazan.

## 3. Una fuente de datos en el medio

`linked_data.rr` pone dos componentes más entre el archivo y los controles:

```basic
CREATE Team AS RDataFrame
    Source = Staff            ' el DataFrame toma las filas del archivo
END CREATE
CREATE Cursor AS RDataSource
    DataSet = Team            ' la fila actual del DataFrame
    OnDataChange = RowChanged
END CREATE
```

y enlaza la cuadrícula, los campos de texto y las etiquetas a `Cursor`. ¿Por qué?

- **Un RDataFrame** se puede cambiar: `Team.Filter "Department", "=", "Engineering"`
  conserva tres filas — la cuadrícula y los campos de texto lo siguen de
  inmediato. `Team.Refresh` vuelve a tomar las filas del archivo. (`Staff` mismo
  no se toca: el DataFrame tiene su propia copia, que no cuesta nada hasta que
  cambia.)
- **Un RDataSource** se mueve por las filas con `First`, `Prior`, `Next` y `Last`,
  dice dónde está (`Row`, desde 0, y `RecordCount`), lee la fila actual
  (`Field("Name")`) y la cambia (`SetField "Name", "Ann"`), y te avisa cuando algo
  cambió (`OnDataChange`). Su `EOF` es True después de que `Next` no pudo avanzar
  más, así que esto recorre todas las filas:

```basic
Cursor.First
WHILE NOT Cursor.EOF
    PRINT Cursor.Field("Name"), Cursor.Field("Salary")
    Cursor.Next
WEND
```

Todo lo que está enlazado a `Cursor` lo sigue; otro RDataSource sobre el mismo
DataFrame tendría una fila actual propia.

## 4. Filtrar, ordenar y agrupar

Los botones de `linked_data.rr` son de una línea cada uno:

```basic
SUB ShowEngineering
    Team.Filter "Department", "=", "Engineering"
END SUB

SUB ShowEveryone
    Team.Refresh
END SUB

SUB SaveTable
    Team.SaveToCSV "staff_edited.csv"
    IF Team.Error <> "" THEN
        Status.Caption = "No se pudo guardar " + Team.Error
    ELSE
        Status.Caption = "Se guardaron " + STR$(Team.RowCount) + " filas en staff_edited.csv"
    END IF
END SUB
```

Un guardado puede fallar —la carpeta puede ser de solo lectura (la carpeta propia
de una aplicación, los ejemplos de RapidR donde está instalado)— así que comprueba
`Error` después y díselo al usuario: dice por qué con palabras sencillas
(`staff_edited.csv: this folder can't be written (it is read-only, or not
yours)`), y es `""` cuando el archivo se escribió. `LoadFromCSV`, `LoadFromJSON` y
`SaveToJSON` también lo establecen.

Todo lo que puede hacer RDataFrame funciona en un DataFrame vinculado — `Sort`,
`GroupBy`, `Select`, `Describe`, `Merge` … ([Ciencia de datos](data-science.md#rdataframe));
los controles enlazados a él muestran el resultado.

## 5. Un millón de filas

[`examples/data/million_rows.rr`](../../../examples/data/million_rows.rr) escribe
una vez un archivo de un millón de registros de personal inventados, y luego lo
lee con un RCSV en un RDataFrame que muestra un RDBGrid. Desplázalo: la
cuadrícula dibuja solo las filas que están en pantalla, así que es tan rápida con
un millón de filas como con diez. Los botones filtran, ordenan y agrupan el
millón completo y dicen cuánto tardó cada uno.

![million_rows.rr: un millón de filas en la cuadrícula (números de fila de 0 a 16 en pantalla, el deslizador de la barra de desplazamiento arriba), “Read big.csv: 1000000 rows in 20 ms” debajo](../images/data/million-rows.png)

En la Mac de este manual (un Apple M4 Max) el programa lee el archivo de 25 MB en
unos 15 ms, conserva las filas con un salario superior a 100 000 (219 912 de
ellas) en 2 ms, ordena el millón en 11 ms y lo agrupa por departamento en 5 ms —
RDataFrame trabaja por columnas y reparte los trabajos grandes entre todos los
núcleos. Escribir el archivo la primera vez tarda más (unos 3 segundos: eso es
BASIC escribiendo un millón de líneas una por una). Un navegador lo ejecuta en un
solo núcleo: en Chrome, en la misma Mac, leer tarda 0.15 s, el filtro 10 ms, el
orden 60 ms y la agrupación 33 ms — todavía una fracción de segundo.

Cuando se ejecuta donde su carpeta no se puede escribir (los ejemplos de RapidR
donde está instalado — RapidR Studio abre tu copia en su lugar), no puede escribir
`big.csv`: la ventana lo dice y explica por qué, con los botones desactivados, en
lugar de una cuadrícula vacía. Lo comprueba como puede hacerlo cualquier
programa: `FILEEXISTS` y `FILELEN` después de cerrar el archivo.

![million_rows.rr ejecutado en una carpeta que no puede escribir: “Couldn't write big_2000.csv: the folder it runs in can't be written: it is read-only, or the disk is full. Copy the example to a folder of yours and run it there…”, con los botones atenuados](../images/examples/million-rows-read-only.png)

## 6. Registros JSON

Un `RJson` que contiene un arreglo (array) de registros (`[{"name": "Pencil", "price": 0.5}, …]`),
JSON Lines, o columnas (`{"name": [...], "price": [...]}`) también es una fuente
de datos:

```basic
CREATE Products AS RJson
    FileName = "products.json"
END CREATE
CREATE Stock AS RDataFrame
    Source = Products
END CREATE
CREATE Grid AS RDBGrid
    DataSource = Stock        ' un DataFrame directamente: su propia fila actual
END CREATE
```

[`examples/data/json_source.rr`](../../../examples/data/json_source.rr) muestra
los productos en una cuadrícula, campos de texto para el actual, un filtro,
totales por categoría, y guarda la tabla de nuevo como JSON (`Stock.SaveToJSON`).

![json_source.rr: los productos de products.json en la cuadrícula, con Calculator como actual, su nombre, existencias y precio en los campos de texto y la etiqueta de abajo](../images/data/json-source.png)

Cuando el programa cambia el documento (`Products.Parse`, `LoadFile` …), el
DataFrame vuelve a tomar los registros.

## Conviene saber

- **Dónde se encuentran los archivos**: un `FileName` sin carpeta se busca en la
  carpeta actual y luego junto al programa. En un navegador, es uno de los
  archivos de la página (los del proyecto, o uno que guardó el programa).
- **Los errores** no detienen el programa: un RCSV cuyo archivo no está no tiene
  filas, su `Error` dice por qué (`there is no file "people.csv"`) y se dispara su
  `OnError(Message)`; el `Error` de un RJson dice por qué no se pudieron leer sus
  registros, el de un RDataFrame por qué falló su última carga o su último
  guardado. Muéstralos en la ventana — una cuadrícula vacía por sí sola no dice
  nada:

  ```basic
  IF Staff.Error <> "" THEN Status.Caption = "No se pudieron leer los datos del personal: " + Staff.Error
  ```

- **Lo que el usuario escribe** en un campo de texto enlazado a una columna
  numérica tiene que ser un número: `"4x"` no se acepta (la tabla conserva su
  valor, el campo de texto conserva el texto hasta que se corrija o cambie la
  fila), y el `DataError` del campo de texto dice por qué —
  `stock is a whole number: "4x" isn't one` — para que lo muestre su `OnChange`.
- **Delimitadores y encabezados**: `Delimiter = ";"` (o `"tab"`, `"|"` …);
  `HasHeader = False` cuando la primera línea son datos (las columnas son
  entonces `column_1`, `column_2` …).
- **Números y texto**: una celda se lee como el texto con el que se escribió
  (`"30.50"` sigue siendo `"30.50"`); el tipo de una columna es lo que son sus
  celdas, así que los números se ordenan y se comparan como números. Un valor
  ausente (un campo vacío, `NA`, el `null` de JSON) se muestra atenuado como
  `null` en una cuadrícula.
- **Todos los miembros** de estos componentes, con lo que hace cada uno, están en
  la referencia: [componentes](../reference/components.md),
  [miembros](../reference/members.md).

Lo que viene (docs/data-science-plan.md): Excel, Parquet y fuentes de bases de
datos como componentes, edición en la cuadrícula, una barra de navegación,
gráficos enlazados a columnas, condiciones escritas como expresiones de BASIC,
uniones y tablas dinámicas, y SQL sobre DataFrames.
