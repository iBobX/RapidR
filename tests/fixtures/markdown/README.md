# Pantry

A small RapidR program that keeps a list of what is in the pantry: type a
name and a quantity, press **Add to shelf**, and the grid shows it.
Everything is saved to `pantry.csv` when the window closes.

## Running it

1. Open `pantry.rr` in RapidR Studio.
2. Press **F5** (Run > Start).
3. Add a few items; close the window to save them.

Build it as an app with **Run > Build** — see
[Building apps](../../../docs/manual/building-apps.md).

## How it works

- The form is an `RForm` with an `REdit`, an `RButton` and an `RStringGrid`.
- `AddItem` runs when the button is clicked:

```basic
SUB AddItem
  Stock.AddRow NameEdit.Text, QtyEdit.Text
  NameEdit.Text = ""
END SUB
```

> The program runs the same on the desktop and in a browser: build it for
> the web with `rapidr build --web pantry.rr`.

## Files

| File | What it is |
|---|---|
| `pantry.rr` | the program |
| `pantry.csv` | the saved list (made on the first run) |
| [notes.md](notes.md) | ideas for later |

---

MIT licence. See [RapidR on GitHub](https://github.com/iBobX/RapidR).
