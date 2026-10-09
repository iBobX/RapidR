# Markdown in RapidR

RapidR shows **Markdown** as a reader sees it: *italic*, **bold**, ~~struck out~~,
`inline code` and [a link to RapidR](https://github.com/iBobX/RapidR).
A [heading link](#tables) scrolls, and [another file](notes.md) opens in Studio.

## Lists

- First item
- Second item with `code`
  1. A numbered sub-item
  2. Another one
- Third item

> A block quote: what someone said,
> on two lines.

## Code

```basic
' Hello in RapidR
DIM Name AS STRING
Name = "World"
PRINT "Hello, " + Name   ' a comment
```

## Tables

| Component | Kind | Runs on |
|---|:---:|--:|
| `RButton` | visual | everywhere |
| `RMarkdownView` | visual | everywhere |
| `RTimer` | not visual | desktop and web |

---

### A third-level heading

Plain text after a rule, long enough to wrap across the width of the view so the line breaking is seen.
