"""The scenes of RapidR Studio's own goldens (tests/visual/studio/): what a
person sees when they open Studio and do the things people do — the Welcome
page, the designer with a component selected, the code editor's completion
and hover, the debugger paused, the AI panel, Markdown, the import report,
the dialogs. Each is captured by the UI kernel's headless host in each of
RapidR's looks (light, dark, high contrast) and compared with its approved
image: `tools/visual/gallery.py studio check` (tests/visual/README.md).

A scene is Studio started the way the tests start it — `rapidr run
ide/studio.rr --home . --fresh --theme <look> [open] --do <steps>` — with
the steps tests/studio_shell.mjs and tests/studio_flows.mjs use (the very
commands, one each 400 ms once Studio is up: `wait` lets a turn go by) and
the capture taken `delay` seconds after the start:

    name     the image's name (tests/visual/studio/<look>/<name>@<s>x.png)
    open     the file Studio opens ("" for none; {work}: see below)
    do       the steps (--do)
    events   RAPIDR_TEST_EVENTS: real mouse and key events through the kernel
    delay    seconds before the capture (the steps take 0.4 s each, and
             what they start — a debug session, a fetch — its time)
    window   which window of the run is the image (1: Studio; 2: the dialog
             it has open)
    scales   1 (default) or 1 and 2 (high-DPI)
    mock     the AI mock provider (tests/ai_mock_server.mjs) is up: {mock}
             in the steps is its address
    env      more of the run's environment (RAPIDR_NO_RUST: the same dialog
             on a machine with Rust and one without)
    work     the scene writes beside what it opens: a copy of this folder
             (relative to the repository) in a folder of its own, opened from
             there ({work}); no machine's path may be on the screen
    why      what the scene guards, for the listing

Everything here is deterministic on purpose: the fonts are the ones RapidR
carries, the window's size is fixed, the paths are the repository's own
relative ones, a fresh settings store and layout every run (--fresh), the
program's keys made up and kept in memory, no clock on the screen.
"""

SCENES = [
    {
        "name": "welcome",
        "open": "",
        "delay": 4,
        "scales": [1, 2],
        "why": "the first thing a new user sees: Start, Examples, Recent, the empty panes",
    },
    {
        "name": "designer-pins",
        "open": "examples/data/dataframe.rr",
        "do": "wait,view.designer",
        "events": "designdoc(0).__mousedown_225_154,designdoc(0).__mouseup_225_154",
        "delay": 7,
        "scales": [1, 2],
        "why": "the designer on live data (a grid and a chart read from a CSV), a component selected, its anchor pins (filled where anchored)",
    },
    {
        "name": "project-properties",
        "open": "tests/fixtures/studio_projprops/RenMany.rrproj",
        "do": "wait,view.designer,project:select,prop:Theme=RapidR Dark,row:Theme,wait",
        "delay": 7,
        "why": "the project's name selected in the Project pane: the project's properties (Theme with its description, name, main file, targets, arguments, the designer's grid), and the form in the designer drawn in the look the Theme row chose — dark — whatever look Studio has",
    },
    {
        "name": "designer-handles",
        "open": "examples/rapidq/notepad.bas",
        "do": "view.designer,designer.place.QBUTTON",
        "events": ",".join(
            f"designdoc(0).{e}"
            for e in [
                "__mousedown_112_132", "__mouseup_112_132",
                "__mousedown_503_262", "__mousemove_533_262", "__mousemove_563_262", "__mouseup_563_262",
                "__mousedown_122_142", "__mousemove_142_162", "__mousemove_162_182", "__mouseup_162_182",
            ]
        ),
        "delay": 6,
        "why": "the designer after a drag and a placed button: selection handles, the form's grips, the dialogs' tray, the inspector following",
    },
    {
        "name": "editor-completion",
        "open": "examples/gui/hello_form.rr",
        "do": "focus:codedoc(0),key:Ctrl+End,key:Enter,type:dim y as string,key:Escape,key:Enter,type:form.c",
        "delay": 6,
        "scales": [1, 2],
        "why": "the code editor with the completion list open and its documentation beside it",
    },
    {
        "name": "editor-hover",
        "open": "examples/gui/hello_form.rr",
        "do": 'focus:codedoc(0),key:Ctrl+End,type:ShowMessage "Hi",key:Escape,key:Home,key:Right,key:Right,edit.showHover,wait,wait',
        "delay": 7,
        "why": "the editor's hover: a statement's signature and what it does",
    },
    {
        "name": "editor-problems",
        "open": "examples/gui/hello_form.rr",
        "do": "focus:codedoc(0),key:Ctrl+End,type:x$ = NameEdit.Txet,key:Escape,wait,wait,wait,view.problems",
        "delay": 8,
        "why": "a misspelt member squiggled in the code, listed in Problems",
    },
    {
        "name": "debug-variables",
        "open": "examples/debugging/shop.rr",
        "do": "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables,wait",
        "delay": 8,
        "why": "the debugger paused on shop.rr: the current line, Variables (Locals, Globals), the Call Stack",
    },
    {
        "name": "debug-callstack",
        "open": "examples/debugging/shop.rr",
        "do": "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,debug.stepInto,wait,wait,frame:1,wait,view.callStack,wait",
        "delay": 9,
        "why": "the Call Stack with its second frame picked, the others not selected",
    },
    {
        "name": "debug-breakpoints",
        "open": "examples/debugging/shop.rr",
        "do": "line:76,debug.toggleBreakpoint,line:147,debug.toggleBreakpoint,bpcond:count > 1,run.start,wait,wait,wait,view.breakpoints,wait",
        "delay": 9,
        "why": "the Breakpoints pane: two breakpoints, the second's condition in its strip",
    },
    {
        "name": "debug-watch",
        "open": "examples/debugging/shop.rr",
        "do": "line:76,debug.toggleBreakpoint,watch:total,watch:count,watch:orders(1).Buyer.Name,run.start,wait,wait,wait,run.start,wait,wait,wait,view.watch,wait,changed:report",
        "delay": 10,
        "why": "Watch, and values marked as changed since the last stop",
    },
    {
        "name": "debug-datatip",
        "open": "examples/debugging/shop.rr",
        "do": "line:76,debug.toggleBreakpoint,run.start,wait,wait,wait,hover:76:37,wait,wait,wait,tipopen:1,wait,wait,tipopen:4,wait,wait",
        "delay": 9,
        "scales": [1, 2],
        "why": "a data tip opened like a tree: an array of records, one record, its nested one",
    },
    {
        "name": "debug-error",
        "open": "tests/fixtures/studio_debug/oops.rr",
        "do": "run.start,wait,wait,wait,view.variables",
        "delay": 6,
        "why": "a run-time error: its line marked, the message at the line's end, Variables",
    },
    {
        "name": "ai-panel",
        "open": "examples/gui/hello_form.rr",
        "do": "ai:on,ai:provider=compatible,ai:base={mock},ai:model=mock-small,ai:ask=hello there,wait,wait,wait",
        "delay": 6,
        "mock": True,
        "why": "the AI Assistant's panel: a question, the streamed answer, the status line (a mock provider: no network, no key)",
    },
    {
        "name": "settings-ai",
        "open": "",
        "do": "ai:on,ai:provider=compatible,ai:base={mock},ai:key=sk-not-a-real-key,ai:model=mock-small,ai.settings",
        "window": 2,
        "delay": 5,
        "mock": True,
        "why": "Settings > AI: provider, a made-up key (kept in memory), the models its provider lists, the permissions",
    },
    {
        "name": "markdown",
        "open": "tests/fixtures/markdown/rapidr-import-report.md",
        "do": "wait",
        "delay": 4,
        "why": "a Markdown file's preview, with its outline",
    },
    {
        "name": "richtext",
        "open": "examples/gui/richtext.rr",
        "do": "wait,view.designer",
        "delay": 7,
        "why": "the rich-text example in the designer: the toolbar, the RRichEdit, the inspector",
    },
    {
        "name": "import-report",
        "open": "",
        # (the report's first lines and Output name the machine's own paths — and
        # the path's length moves everything below it, a scrolled report
        # included: the Immediate tab is shown over Output and the program's
        # own tab is the one open, its report beside it)
        "do": "import:{work}/greeter.rqw,wait,wait,view.immediate,window.previous,wait,view.code,wait",
        "work": "tests/fixtures/rapidq_import",
        "delay": 9,
        "why": "a RapidQ program imported: its copy opened with RapidR's names, its include in the project, the import report in the next tab",
    },
    {
        "name": "save-prompt",
        "open": "tests/fixtures/studio_save/main.rr",
        "do": 'wait,code:"ONE"=>"ONE saved",file.exit',
        "window": 2,
        "delay": 4,
        "why": "the question before changes are lost: Save, Don't Save, Cancel",
    },
    {
        "name": "project-options",
        "open": "{work}/Notes.rrproj",
        "work": "tests/fixtures/studio_app",
        "do": "project.options",
        "env": {"RAPIDR_NO_RUST": "1"},
        "window": 2,
        "delay": 4,
        "why": "Project > Project Options: the app's name, ID, version and icon",
    },
    {
        "name": "find-in-files",
        "open": "tests/fixtures/studio_split/main.rr",
        "do": "wait,find:SayGreeting",
        "delay": 4,
        "why": "Find in Files: its results in the pane, grouped by file",
    },
    {
        "name": "project-translations",
        "open": "tests/fixtures/studio_lang/Bookshop.rrproj",
        "do": "wait,wait",
        "delay": 5,
        "why": "the Project pane of a program that translates: lang/ holds its .po catalogs and a .pot template, each with the translation icon, beside the other kinds of file (a vector image, XML, data, MIDI)",
    },
    {
        "name": "palette",
        "open": "",
        "do": "palette:view",
        "delay": 4,
        "why": "the command palette over Studio, filtered to the View commands",
    },
]
