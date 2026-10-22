# Your own SVG icons

A program can draw its trees, lists, toolbars and pictures with its own SVG
icons. They are sharp at every screen scale, and they take the colours of
the theme, light, dark or high contrast, the way RapidR's own icons do. When
the theme changes, they are drawn again in the new colours.

| Light | Dark | High contrast |
|---|---|---|
| ![The example in the light theme](images/svg-icons/svg-icons-light.png) | ![The same icons in the dark theme](images/svg-icons/svg-icons-dark.png) | ![The same icons in high contrast](images/svg-icons/svg-icons-high-contrast.png) |

The example is [examples/gui/svg_icons.rr](../../examples/gui/svg_icons.rr),
with its icons in [examples/gui/icons/](../../examples/gui/icons). Run it with
`rapidr run gui/svg_icons.rr` from the examples folder.

## An image list of SVG icons

`RImageList.AddSVGFile(FileName [, Theme])` adds an SVG. It is drawn at the
list's `Width` × `Height`, whatever size the file says, and drawn again at
the screen's scale, so a 16-pixel icon stays crisp on a 2× screen.
`AddSVGFile` gives the new picture's index.

```basic
DIM Icons AS RImageList
Icons.Width = 16: Icons.Height = 16
Icons.AddSVGFile "icons/table.svg"     ' 0
Icons.AddSVGFile "icons/view.svg"      ' 1

CREATE Tree AS RTreeView
    Left = 12: Top = 12: Width = 190: Height = 200
END CREATE
Tree.Images = Icons
Tree.AddItems "orders"
Tree.Item(0).ImageIndex = 0
```

The `Theme` argument (`"dark"`, `"highcontrast"`, `"light"`, `"classic"`)
draws the icon in that theme's colours for good: it doesn't follow the
program's theme.

A missing file, a file that isn't an SVG, or a name that isn't a theme adds
nothing, and the runtime says why where its messages go (the terminal, the
browser's console).

## Colours that follow the theme

An SVG follows the theme when it is drawn with:

- **`currentColor`**: the theme's ink. That is slate (`#5B6478`) when light,
  mist (`#C9D1E3`) when dark, and white in high contrast. Use it for
  one-colour icons. Before, `currentColor` came out black, which can't be
  seen on a dark background.
- **RapidR's colour tokens**, written as their light-theme values, as
  RapidR's own icons are:

| Token | Write | Dark | High contrast |
|---|---|---|---|
| ink | `#5B6478` | `#C9D1E3` | white |
| paper | `#FFFFFF` | `#373A42` | black |
| blue / blue tint | `#2F5BFF` / `#E3E9FF` | `#7398FF` / `#24315C` | white / black |
| cyan / cyan tint | `#007F9C` / `#D7F3F9` | `#2CC9E9` / `#123C47` | white / black |
| teal / teal tint | `#08805E` / `#D6F2E8` | `#34CF98` / `#123D30` | white / black |
| amber / amber tint | `#B86E00` / `#FFEDC7` | `#FFB224` / `#45340F` | white / black |
| red / red tint | `#D92D36` / `#FCE1E2` | `#FF6B70` / `#4D1E22` | white / black |
| violet / violet tint | `#6F42E5` / `#EBE4FD` | `#AC8FFF` / `#33295E` | white / black |

Every token, and how RapidR's icons use them, is in RapidR's icon design
system: `design/icons/README.md` in RapidR's source.

The tokens are swapped only when every colour the SVG uses is a token. So
an illustration with colours of its own, a photo-like drawing or a logo,
keeps its colours. If it also uses `currentColor`, only that follows. A
root `<svg color="…">` keeps the colour it names. Masks drawn in `white` and
`black` are left as they are.

For a check you can do yourself: a light-theme icon written this way
looks right on its own in any SVG viewer, because its colours are the
light theme's.

## Pictures and buttons

The same rule applies to every SVG a program loads:

- `RImage.Load` and `LoadFromFile`;
- `RBitmap.LoadFromFile`;
- a button's `BMP`;
- `RImageList.AddBMPFile` (an SVG of the list's size follows the theme;
  one of another size is cut to the list's size in the colours of the
  theme it was added in).

```basic
Picture.Load "icons/database.svg"
```

## SVGs built into the program

`$RESOURCE` builds a file into the program. Its name then goes wherever a
picture file goes: `RImage.Load`, `LoadFromFile`, `AddSVGFile`, a `BMP`,
`BMPHandle` and an `Icon`.

```basic
$RESOURCE TABLE_SVG AS "icons/table.svg"
$RESOURCE DATABASE_SVG AS "icons/database.svg"

Icons.AddSVGFile TABLE_SVG
Picture.Load DATABASE_SVG
```

A built app then has its pictures wherever it is started from, including
pictures whose file names the program puts together while it runs.
(Builds copy only the files whose names appear in the source.) Before,
`RImage.Load` with a `$RESOURCE` said "could not load '65536'".

## Where it works

The same on the desktop (`rapidr run`, interpreted and native builds) and
on the web.
