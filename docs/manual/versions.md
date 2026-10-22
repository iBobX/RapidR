# Versions: never lose your work

**Versions** keeps every saved state of your project, shows what changed
between any two, and takes you back to any of them. It is Git underneath
(the same repositories every Git tool opens), in plain words on top.

> **Where it is today.** The engine and the diff views are done. The
> Versions panel in RapidR Studio (the Timeline, the Save a version box,
> the Diff tab) comes next, in Preview 4. Until then, the diff views are
> yours to use in your own programs: [RDiffView](#rdiffview-three-ways-to-see-what-changed)
> below.

## The ideas, in plain words

Each plain word has its Git name beside it, so you learn real Git without
noticing.

| You say | What it means | Git calls it |
|---|---|---|
| **Start versioning** | The project's folder starts keeping versions. | `git init` |
| **Save a version** | A snapshot of your files, with a few words about what changed. You choose all changed files or some. | commit (with staging) |
| **Timeline** | Every version, newest first: its words, who saved it, when. | log |
| **What changed?** | The differences: since the last version, or between any two. | diff |
| **Go back** | The whole project as it was in an older version. It is saved as a *new* version, so the newer ones stay on the timeline. | revert / checkout (never a hard reset) |
| **Restore a file** | One file as it was in an older version. It becomes an unsaved change, which you save or undo. | restore |
| **Experiment** | A separate line of versions to try something, without touching the main one. Switching rewrites only the files that differ. | branch, switch |
| **Bring in** | An experiment's changes joined into the current one. | merge |
| **Shared folder** | A folder (a USB stick, a network drive) to share versions with others: *Get latest* takes theirs, *Share* gives yours. | a bare repository, fetch / push |

## Nothing is ever lost

- **Safety points.** Before anything that rewrites files or moves a
  version (going back, restoring, switching, bringing in, getting the
  latest), Versions quietly saves the whole folder as it is, unsaved
  changes and new files included. Safety points never show on the
  timeline and are never shared.
- **Undo.** Every action goes into a history, and **Undo** takes the last
  one back, even the next day. Undo refuses, changing nothing, when it would
  overwrite work you did afterwards: it tells you which file, so you can
  save first.
- **Unsaved changes are never thrown away.** Switching to an experiment or
  going back waits until your changes are saved in a version.
- **The AI's checkpoints.** Before the assistant changes your files, a
  checkpoint keeps them as they were. "Go back to before the AI's change"
  puts them back, and Undo brings the AI's change back again.

## A secret check before every save

Saving a password or a key in a version is a common mistake: versions are
often shared, and a secret stays in the timeline. Before each save,
Versions reads the lines you added and stops when one looks like a private
key, an API key, an access token or a password written in the code. It
says the file and the line, shows the secret masked (`"sk-p…"`), and lets
you save anyway when it isn't a real one. Keep real secrets out of your
code with [RSecrets](secrets.md).

## Safe with any folder

Opening a project never runs anything from it: no hooks, no filters, no
programs named in its settings. A project that belongs to another user
account on your computer (or comes from a USB stick or a network drive)
isn't opened for changes: Versions explains why and offers to open it
read-only, to look at its versions.

## RDiffView: three ways to see what changed

`RDiffView` compares two texts. Its `Kind` says how:

| Kind | What it shows |
|---|---|
| `"text"` (the default) | Line by line, side by side or in one column. The changed **words** are marked, and each run of changes (a hunk) has Accept and Reject buttons. |
| `"form"` | Two versions of a program's forms: the form **before and after**, drawn as the designer draws it, the changed components outlined (green added, red removed, blue changed), and the changes listed: "GreetButton: Caption "&Greet" → "&Say hello"; Width 90 → 110". |
| `"table"` | Two CSV files as one table: added rows green, removed rows red, a changed row as its old values over its new ones with the changed cells marked, a new column tinted. Rows are matched by their first column when its values are unique (an Id), else by their content. |

```basic
CREATE Diff AS RDiffView
    Left = 8: Top = 8: Width = 800: Height = 460
    Kind = "table"
END CREATE

Diff.LeftText = "Id,Name,Score" + CHR$(10) + "1,Ana,10" + CHR$(10)
Diff.RightText = "Id,Name,Score" + CHR$(10) + "1,Ana,12" + CHR$(10)
PRINT Diff.ChangeCount     ' 1
PRINT Diff.Summary         ' Row 1 (Id 1): Score 10 → 12
```

`Summary` says every change in words, one per line: what a screen reader
reads, and what you can show or log. `ChangeCount` counts them.

![The text view: the changed words marked](images/versions/diff-text.png)

![The form view: the form before and after, the changes listed](images/versions/diff-form.png)

![The table view: rows and cells](images/versions/diff-table.png)

The example is [examples/studio/diff_views.rr](../../examples/studio/diff_views.rr):
`rapidr run studio/diff_views.rr` from the examples folder.

The form view needs RapidR Studio's form designer to read the forms:
`rapidr run`, RapidR Studio and programs in the browser have it. An
executable built on its own shows a note there instead.

## For the curious

- The diff is RapidR's own: Myers' algorithm, written from his 1986 paper,
  the same for RDiffView, Versions and the assistant's changes. A
  4,500-line file is compared in well under 50 ms.
- The repositories are ordinary Git repositories (gitoxide, a Git engine
  written in Rust): other Git tools open them, and RapidR opens theirs.
- Safety points and checkpoints are commits under `refs/rapidr/`; the
  history of actions is `.git/rapidr/history.jsonl`.
