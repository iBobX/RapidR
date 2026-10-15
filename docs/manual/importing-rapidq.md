# Importing RapidQ programs

RapidR runs RapidQ programs **as they are**: you don't have to import
anything to run, build or edit one. Importing is for when you want a program
to use **RapidR's own names** (`RButton` instead of `QBUTTON`) from now on.

Importing always works on a **copy**. Your original files are only read,
never changed.

## What an import does

- It copies the program — the main file, every file it `$INCLUDE`s
  (however deep, wherever RapidR finds them: see
  [Include files](#include-files)), and (for a folder) everything else in
  the folder — into a new folder beside it, named `<name>-rapidr`.
- In the copy, it changes **component names only**: `QFORM` becomes
  `RForm`, `QBUTTON` becomes `RButton`, `QGAUGE` becomes `RProgressBar`, and
  so on. Nothing else changes: not your code, not strings, not comments, not
  your own names (`QButtonCount` stays as it is), not your own TYPEs.
- It **proves** the copy behaves the same: the original and the copy are
  both compiled, and the results must be identical, byte for byte. The
  report says so for every program.
- It writes a report, `rapidr-import-report.md`, beside the copy: every
  name it changed (file, line, column), every include file and where it
  was found, and anything it left alone, and why.
- `$INCLUDE "RAPIDQ.INC"` stays: RapidR supplies RapidQ's constants (`clRed`,
  `MB_OK` …) through that line, without the file.

Before and after, for a small program:

```basic
' greeter.rqw, as written for RapidQ          ' the copy, greeter-rapidr/greeter.rqw
DECLARE SUB SayHello (Sender AS QBUTTON)       DECLARE SUB SayHello (Sender AS RButton)
CREATE Form AS QFORM                           CREATE Form AS RForm
    CREATE NameEdit AS QEDIT                       CREATE NameEdit AS REdit
    END CREATE                                     END CREATE
    CREATE HelloBtn AS QBUTTON                     CREATE HelloBtn AS RButton
        Caption = "&Hello"                             Caption = "&Hello"
        OnClick = SayHello                             OnClick = SayHello
    END CREATE                                     END CREATE
END CREATE                                     END CREATE
```

## Include files

A RapidQ program is often several files: the main program brings in its
libraries and forms with `$INCLUDE`. RapidR finds each one where RapidQ did,
on every system:

1. **in the program's own folder** — a nested `$INCLUDE` too (an include
   that includes another names it from the program's folder, as RapidQ
   compiles there);
2. **in your include folders**, in order — RapidQ's include folder, where
   `WINDOWS.INC`, `RAPIDQ2.INC` and your own libraries are;
3. then **beside the file that includes it**.

Names are matched **in any case, with `\` or `/`**: RapidQ code was written
on Windows, so `$INCLUDE "Lib\Geometry.INC"` finds `lib/geometry.inc` on a
Mac and on Linux. A path from another computer — `C:\rapidq\include\x.inc`,
`\RapidQ\include\x.inc` — is found by its ending (`include\x.inc`, then
`x.inc`) in those folders.

`RAPIDQ.INC` needs no file: RapidR has it built in (so are `QMidi.inc`,
`QWave.inc`, `QVideo.inc`, `QCDAudio.inc`, `QCgi.inc`, `QDownload.inc`,
`QDockForm.inc` and `QDirListView.inc`).

**Your include folders.** In RapidR Studio: **Tools ▸ Settings…**, then
**Add Folder…** under *Include folders*. Studio uses them for everything:
opening a program, its problems, Run, Build and Import. From the command
line: `--include-dir <folder>` (any number of times), or the
`RAPIDR_INCLUDE_PATH` environment variable.

![Tools > Settings: the Include folders list with RapidQ's include folder in it, Add Folder and Remove beside it](images/import/include-settings.png)

**When you import**, every include the program uses is copied with it, its
names converted too:

- the program's own include files keep their place (`lib/geometry.inc`
  stays in `lib/`);
- a file from an include folder goes into the copy's folder as it was in
  the include folder (`GL\QGL.INC` → `GL/QGL.INC`), so the copy needs no
  include folder;
- an `$INCLUDE` line that wouldn't find its file's copy any more
  (`..\shared\common.inc`, `C:\rapidq\include\x.inc`) is changed to name
  the copy, and the report says so;
- `RAPIDQ.INC` and the other includes RapidR has built in are never
  copied — your own copy of one neither (the report lists it). If your
  program uses something of your `RAPIDQ.INC` that RapidR's doesn't have
  (RapidQ's `QBColor` array), the report says to keep that file's folder in
  your include folders.

The report's **Include files** table lists each `$INCLUDE`, where it was
found and where its copy is:

![The imported Shapes example: the project's Includes are forms/shapesform.inc, lib/geometry.inc and lib/units.inc; the report lists each include, found in the program's folder, and its copy](images/import/import-includes.png)

**An include that isn't found anywhere is never passed over.** The report
lists it under *Not found* with what to do, Studio's **Output** says it and
the status line turns red, and `rapidr import-rapidq` prints it and ends
with exit code 1:

```text
main.bas:5: include file not found: Inc\Tools.INC — add the folder that holds Tools.INC in RapidR Studio > Tools > Settings > Include folders (or `--include-dir <folder>`), or put Tools.INC beside the program (…), and import again
```

Add the folder, then import again. In a browser, RapidR Studio has only the
files you give it: import the program's **folder** (File ▸ Import RapidQ
Folder…), or pick the program **and its include files together** in File ▸
Import RapidQ Project or File… — they are found by their names.

Try it with the example [rapidq/shapes](../../examples/rapidq/shapes/shapes.bas):
a RapidQ program whose library and window are in `lib/` and `forms/`:

```sh
rapidr run rapidq/shapes/shapes.bas
rapidr import-rapidq rapidq/shapes/shapes.bas
```

## Which files

You can import one file — `.bas`, `.rqw`, `.rqb`, `.rq` (the extensions
RapidQ's editors saved programs with) or an `.inc` — or a whole folder, or a
RapidR project (`.rrproj`). Each file keeps its extension in the copy. You
don't need to import `.rqw`, `.rqb` or `.rq` files to use them: RapidR and
RapidR Studio open, run and build them directly.

## In RapidR Studio

1. Choose **File ▸ Import RapidQ Project or File…** to pick a file, or
   **File ▸ Import RapidQ Folder…** to pick a folder. The Welcome page has
   the same command: **Import a RapidQ Program…**. In the command palette
   (**Ctrl+Shift+P**, **⌘⇧P** on a Mac) type `import`.
2. Studio writes the copy beside the original (`greeter-rapidr`; if that
   folder exists, `greeter-rapidr-2` and so on) and says in **Output** how
   many names changed and that each program compiles exactly as before.
3. The copy opens as a project: its files in **Project**, its main file in
   the editor, and the report in a tab beside it.

![The command palette with "import" typed: File: Import RapidQ Folder and File: Import RapidQ Project or File](images/import/import-commands.png)

![After the import: the copy open as a project (Forms: greeter.rqw, Includes: include/shapes.inc) and rapidr-import-report.md beside it: the summary, each program compiled as the original, then the include files and each change](images/import/import-result.png)

In the copy, the designer, the toolbox, the inspector and the outline all
show RapidR's names, and the code says them too:

![The imported form in the designer: HelloBtn selected, the inspector's header reads "HelloBtn RButton", the outline lists Form: RForm, NameEdit: REdit, HelloBtn: RButton](images/import/imported-design.png)

In the browser, RapidR Studio imports the same way: pick the file or folder
(its files are read into the page), and the copy is written beside it in the
page's storage.

## From the command line

```sh
rapidr import-rapidq greeter.rqw                 # the copy in greeter-rapidr/
rapidr import-rapidq greeter.rqw converted       # the copy in converted/
rapidr import-rapidq my-folder                   # a whole folder
rapidr import-rapidq prog.bas --include-dir C:\RapidQ\include   # where RapidQ's includes are
```

It prints a summary and the report's path. `rapidr upgrade-names file.rr`
does the same to one of **your own** files in place (`--dry-run` shows the
changes first).

## Good to know

- **Names you mix are fine.** RapidR reads `QBUTTON` and `RButton` as the
  same component, always, without any setting — in old and new files alike
  (see [Components](components.md#rapidqs-names)).
- **When you add to a file, Studio follows its style.** In a file written
  with RapidQ's names, the designer adds `QBUTTON` and completion offers
  RapidQ's names, so the file never mixes them; elsewhere, RapidR's names.
- **What isn't converted**, and is listed in the report: names in an
  `$IFDEF` branch your build doesn't compile, names a `$DEFINE` makes, and
  includes that weren't found (add RapidQ's include folder to your include
  folders: [Include files](#include-files)).
- RapidQ's own `.tpl` IDE templates aren't copied.
