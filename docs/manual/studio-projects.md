# Projects in RapidR Studio

A RapidR program can be one file or many: a main file that `$INCLUDE`s
the others. RapidR Studio keeps the list in a project file (`.rrproj`) and
shows it in the **Project** pane. This page covers what the Project pane
shows, opening what you worked on last, and the PATH your programs get when
Studio runs them.

The pictures were made with Studio's own seven-file test project
(`tests/fixtures/studio_forms` in RapidR's source): its windows are in
`ui.rr`, three forms in one file; the other files hold its SUBs and
FUNCTIONs.

## Forms come from your code

Any file that creates a form at its top level, with `CREATE Name AS RForm`
(or `RFormMDI`, or the older names `QFORM` and `QFORMMDI`), is a form file.
It shows under **Forms**, whatever the project file said about it. A file
that creates several forms shows each of them under it:

```basic
' ui.rr: the program's three windows
CREATE Form AS RForm
    Caption = "Shelf"
    ...
END CREATE

CREATE NewForm AS RForm
    Caption = "New Item"
    ...
END CREATE

CREATE CfgForm AS RForm
    Caption = "Options"
    ...
END CREATE
```

![The Project pane: ui.rr under Forms with its three forms, Form, NewForm and CfgForm; main.rr and the other files under Modules. NewForm is selected and open in ui.rr's designer](images/studio-projects/forms-tree.png)

- **A file with several forms** opens in the tree to show its forms, one a
  line. Open one (double-click it, or select it and press Enter) and it
  shows in the file's designer. Select a form, or a component of one, and
  the designer switches to that form; it tells a screen reader which one
  ("Designing NewForm, form 2 of 3 in this file").
- **A file with no form** (the main file that only `$INCLUDE`s the others
  and starts the program, a file of SUBs) shows under **Modules**.
- **The tree follows your typing.** Add a `CREATE … AS RForm` to a module
  and it moves to Forms; take the last form out of a file and it moves back
  to Modules.
- **The project file follows too.** When you save (File > Save, Ctrl+S, or
  Save All), Studio writes the kinds your code says into the `.rrproj`, and
  the Output says what changed:

  ```text
  main.rr makes no form: now under Modules
  ui.rr makes a form: now under Forms
  Saved …/Shelf.rrproj
  ```

  A file the AI assistant writes for you is listed the same way: a new file
  that creates a form goes under Forms.

An include file (`.inc`) that creates a form is a form file too. One that
creates none stays under Includes.

## Open Recent

**File > Open Recent** lists the last ten projects and files you opened,
newest first, each with its folder. Press its number to open it. **Clear
Recent** empties the list (the files stay where they are).

![File > Open Recent: Shelf.rrproj in ~/Projects/Shelf and pantry.rr in ~/Projects/Pantry, then Clear Recent](images/studio-projects/open-recent.png)

From the keyboard: **Alt+F**, then **R** (on a Mac, the menu bar's File >
Open Recent). The command palette (Ctrl+Shift+P) has the same items under
"Open Recent". The Welcome page's **Recent** list shows the first six.

If something on the list isn't there any more (moved, renamed, on a drive
that isn't connected), choosing it takes it off the list, and the status
bar and the Output say so.

## Reopening the last project

When Studio starts without a file to open, the Welcome page offers the
project you had open last, with the files that were open in it:

![The Welcome page: a card "Reopen Shelf, and the 2 files you had open" at the top of Start, with the keyboard's focus ring](images/studio-projects/reopen.png)

The keyboard starts on the card: press **Enter** (or Space, or click it),
and the project opens with its files in their tabs and the layout you left.
To start somewhere else, use the other items on the page as usual.

You can turn the offer off in **Tools > Settings > General**: clear
**Offer to reopen the last project and its files at start-up**.

## Project properties

Select the project's name, the top row of the **Project** pane, and the
**Properties** pane shows the project: "RenMany  Project" in its header,
then its settings in groups, with what each one does under the rows (select
a row to read it). Select a form or a component again and it shows that
instead.

![The Project pane with the project's name selected, and the Properties pane showing the project: Theme set to RapidR Dark, the form in the designer drawn dark while Studio is light](images/studio-projects/project-properties.png)

- **Theme** (Appearance) is the look your program's windows are drawn in:
  **RapidR** follows the system (light, dark or high contrast), **RapidR
  Light**, **RapidR Dark** and **RapidR High Contrast** choose one, and
  **Classic** is the Windows look. The program keeps it in one line at the
  top of its main file, `$THEME RapidR Dark` (none for **RapidR**), so the
  choice travels with the code: the row puts the line in, changes it or
  takes it out as one step you can undo in that file's editor, and the file
  shows as changed until you save it. The form designer draws your form in
  a look you choose at once, whatever look Studio itself has (**View >
  Theme**), the way the program will look when it runs; with **RapidR** it
  shows the form in Studio's own look. Edit the `$THEME` line by hand and
  the designer and the row follow.
- **Name**, **MainFile** and **Targets** (General): the project's name (its
  files keep theirs), the file the program starts from, and where it runs
  (see below).
- **Arguments** (Run): the command-line arguments **Run** gives the program,
  what `COMMAND$` reads. A "quoted" one keeps its spaces.
- **Grid**, **Snap** and **Guides** (Designer): the form designer's grid in
  pixels, snapping to it, and the alignment guides.

Each change but Theme is saved to the project file (`.rrproj`) at once; the
Output says so.

**The main file is where the program starts.** When the project's main file
is a module that another of its files `$INCLUDE`s, Studio says so as the
project opens — *The main file lang.rr is included by main.rr: the program
probably starts in main.rr* —; choose **MainFile** here to change it. And
**Run** always says why it didn't start, or what to look at: nothing open,
a main file that isn't there, the program's errors (in **Problems**), or a
main file that ran and ended at once with no window and nothing printed.

## Selecting a file shows it

Select a file in the Project pane and, a moment later, the editor shows it:
its tab comes to the front, or — when it isn't open — it opens in a
**preview tab**, which the next file you select takes over while you
haven't changed it. Open it (Enter, or a double click) or type in it and it
stays. The keys stay in the Project pane, so you can go down the list with
the arrows and read each file as you go — `lang/es.po`, then
`lang/pt-BR.po` — never one file's text under another file's name.

## Studio keeps your work

- **A mistake in Studio itself never closes it.** If something goes wrong
  in Studio's own code, Studio stops what it was doing, and a dialog says
  *Something went wrong in Studio* with what happened. Your work is safe:
  your documents, saved or not, and the assistant's chat stay as they were,
  and Studio goes on. **Copy Details** copies what happened (for a bug
  report); Studio's log keeps it too (`studio-errors.log` in RapidR's
  settings folder: `~/Library/Application Support/RapidR` on macOS,
  `%APPDATA%\RapidR` on Windows, `~/.config/rapidr` on Linux).

  ![Something went wrong in Studio: what happened, where in Studio's code, and that your work is safe; Copy Details and OK](images/studio-projects/studio-error.png)
- **Unsaved work survives a crash.** While a document has changes you
  haven't saved, Studio keeps a copy of it in that same folder (never in
  your project), every 20 seconds and the moment something goes wrong. If
  Studio is ended without closing — a crash, a forced quit, the computer
  switched off — it asks at the next start: **Restore your unsaved work?**
  **Restore** puts each change back in its document, not saved yet (one
  Undo goes back to the file as it is on disk); **Discard** lets the copies
  go; **Not Now** keeps them and asks again next time. Closing Studio the
  usual way (every change saved or let go) removes the copies.

## Targets: where your program runs

**Project > Project Options** has a **Targets** choice (the project's
properties have it too, as **Targets**): **Desktop** (the
default), **Web**, or **Desktop and web**. It says where your program is
meant to run, and Studio keeps it in the project file:

```toml
[build]
runs_on = "web"
```

Some components and functions work on one runtime only. `RServerSocket`, for
example, listens on a port, which a browser page can't do. The Problems tab
tells you about them only when your project runs where they don't work:

- **Desktop**: nothing about what works on the desktop only (that's where it
  runs); a note for anything that works in the browser only.
- **Web**: a note for each component or function that works on the desktop
  only.
- **Desktop and web**: notes for both.

Each note comes once, at the first use, with how many uses there are:
"RServerSocket works on the desktop only: the web runtime doesn't have it
(12 uses)".

![Project Options: Targets set to Web, with the note "In a browser. Problems notes what works on the desktop only."](images/studio-projects/targets.png)

A project that builds a web app too (`targets` in `[build]` includes `web`)
starts as **Desktop and web**; any other starts as **Desktop**.

## The PATH your programs get

On macOS and Linux, an app started from the Dock, Finder or a desktop
launcher gets the system's short PATH (`/usr/bin:/bin:/usr/sbin:/sbin`), not
the one your Terminal has. A program that runs `SHELL "php -v"`, looks up
`curl`, or uses a Homebrew tool worked in Terminal and failed from Studio.

Studio reads your login shell's PATH once, when it starts, and gives it to
the programs it runs; their `SHELL` and `RUN` commands inherit it. Building
and Run in Browser get it too. Try it:

```basic
PRINT "PATH=" + ENVIRON$("PATH")
SHELL "command -v brew || echo 'brew: not found'"
```

Run from Studio (F5), the Output shows the same PATH Terminal's
`echo $PATH` does, and where `brew` is.

**Tools > Settings > General** shows the folders, in order, and where they
came from. After you change your shell's startup files (`~/.zprofile`,
`~/.zshrc`, `~/.bash_profile` …), press **Refresh**: the next Run gets the
new PATH.

![Tools > Settings > General: the offer to reopen, and Programs' PATH: the folders from the login shell, one a line, with Refresh](images/studio-projects/settings-path.png)

How Studio reads it: it runs your login shell (`$SHELL -l -i -c`) with no
terminal and nothing to read, so it can never stop to ask you anything, and
gives it three seconds. Studio only reads; it never writes to your shell's
files. If the shell doesn't answer in time, Studio uses its own PATH plus
the usual folders that exist on your computer (`/opt/homebrew/bin`,
`/usr/local/bin`, `~/.cargo/bin`, `~/.local/bin` …), and Settings says why.

On Windows, apps get the full PATH already, and on the web programs run in
the browser: there the PATH isn't changed, and Settings doesn't show it.

For your own tools, the same values are in the `RProgramSession` component:
`ShellPath`, `ShellPathSource`, `ReadShellPath` and `RefreshShellPath` (see
the [members reference](reference/members.md#rprogramsession)).
