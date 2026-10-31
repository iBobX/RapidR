# Design audit, October 2026

RapidR Studio measured against [the design guide](design-guidelines.md) on
2026-10-30 (lane STUDIO-POLISH, Preview 5): every pane, dialog and tab
strip, at **1280 × 720** on a 1280 × 720 screen, in light and in Spanish,
dark in Spanish, high contrast in English.

## How it was looked at

- **Pictures**: Studio headless (`rapidr run ide/studio.rr --fresh`, 2×),
  its window 1280 × 720 (the test step `layout:size=1280x720`), the screen
  1280 × 720 (`RAPIDR_TEST_SCREEN=1280x720`, a test hook of the headless
  host). 15 states × 4 looks: the main window (designer and Output /
  Problems), RAI's pane, Versions, Help, AI Apps, the debug panes,
  Settings' three pages, Keyboard Shortcuts, School Lock, About, Set Up
  RAI, New Project, Project Options (Keyboard Shortcuts and About are
  message boxes, which the pictures don't show).
- **The layout report** (`RWindowCapture.Problems`, what MCP's
  `layout_report` says): text cut off, a control past its window's or its
  panel's edge, two controls on top of each other — in every window shown,
  a dialog's too (the test event `layoutprobe.ontimer` writes it to the
  Output while the dialog waits).
- **The dialog size rule**: a dialog at most 85 % of the screen's work
  area (612 × 1088 on 1280 × 720), its buttons inside it.
- **Contrast and screen readers**: the flows' text-contrast check on what
  is drawn, and the tab strips' names in the accessibility tree.

The checks stay: `node tests/studio_flows.mjs dialog-fits` (20 flows,
English and Spanish).

## Fixed

| What | Where | Now |
|---|---|---|
| Tabs were bare words | Settings (General / AI / AI Apps), Output / Build / Problems, Properties / Events | each its icon and its name; icons alone, named in a tooltip and for a screen reader, when a strip is too narrow — never a cut name (`RTabControl.TabIcons` for programs) |
| Settings was 822 pixels tall (918 in Spanish): its buttons off a 720 screen | settings.inc | at most 85 % of the screen: the pages scroll under the tabs, OK and Cancel stay under them |
| Project Options was 657 tall | build.inc | the same rule: the fields scroll, the buttons stay |
| Two lines under Settings' tabs (the tab strip's empty page frame) | settings.inc | one line |
| Set Up RAI's Look Again, Cancel and Next in the small default font | aisetup.inc | Studio's font, as every dialog |
| The Output pane's search box gone in a 500-pixel pane once the tabs had icons | console.rs | shown whenever it fits 16 past the tabs; the Problems tab keeps room for its count only while it has one |
| The Versions footer's buttons in a loose row, Undo's words in English | versions.inc (P1) | one row, equal widths, icons alone when narrow; Spanish |
| RAI's dial cut at its left end, a crowded strip under the chat | ai.inc (P2) | the dial whole, compact controls, a calm strip |

With these, the layout report finds nothing wrong in any of the states
above, in English or Spanish, and every dialog checked fits 1280 × 720.

## Still to do

| What | Where | Note |
|---|---|---|
| Settings is one long page per tab, with empty room under the AI page | settings.inc | the Settings redesign: a category list with search, folded lists, one-line descriptions (the polish queue's next item) |
| Settings > AI: the links block draws a white block across the dialog's face (light) | settings.inc | an `RMarkdownView` on the dialog's colour (also in Known gaps) |
| At 1280 × 720 the Toolbox and the Outline show two rows each | the dock's first layout | the bottom groups could keep a few rows (or the documents give some height) on a short screen |
| Set Up RAI stays 600 tall with half of it empty on its first page | aisetup.inc | the dialog as tall as its page |
| Keyboard Shortcuts and About are plain message boxes | shell.inc | a searchable list of shortcuts; an About card |
| The Help pane's search field has no magnifier, unlike Properties' and the Toolbox's | help pane | with the 26-px field height already in Known gaps |
| The bottom group's tab "Output" and the console's page "Output" say the same word twice | dock + console | name the group's tab by what is shown, or the first page "Program" |
| With RAI off, View > AI Assistant shows nothing new in the main window | ai.inc | to check: it should open Set Up RAI or say why |
| 200 % text | Studio | Studio has no text-size setting yet: the rule can't be checked until it has one |
| Dialogs the size test doesn't open yet | ai.inc, mcp.inc, versions.inc, consent.inc, debug.inc, prompts.inc, safety.inc | A New Program, What Was Sent, the review windows, Save a Version, the consent question, the debugger's text viewer and value editor, the save prompt, the safety note — each needs RAI set up or a state to reach |
| Smoothness: frame times and flips while scrolling, resizing, dragging | everywhere | the SMOOTH lane (`RAPIDR_TEST_FRAMES`) |

The pictures of this audit are not kept in the repository (56 at 2×); the
scenes that make them are the states above.
