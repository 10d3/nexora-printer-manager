# Barcode Label Command Reference

How `src/barcode_printer.rs` talks to the label printer, and how to hand-write
raw commands yourself (TSPL, ZPL, or EPL) if you ever need to bypass the
layout engine.

The sidecar supports three printer command languages, selected by
`BarcodePrinterConfig.protocol`:

| Protocol | Used by (typically)      | Line terminator | Origin (0,0) |
|----------|---------------------------|------------------|---------------|
| **TSPL** (`build_tspl`) | TSC printers | `\r\n` | top-left |
| **ZPL**  (`build_zpl`)  | Zebra printers | `\n`  | top-left |
| **EPL**  (`build_epl`)  | Older Zebra/Eltron printers | `\n` | top-left |

All three are plain-text command streams sent raw over the printer's
connection (USB/serial/network) — no driver involved. Everything below is in
**dots**, not mm or inches (`mm_to_dots()` in `barcode_printer.rs:110`
converts using the configured `dpi`).

---

## 1. TSPL (TSC printers)

Reference manual: [TSPL/TSPL2 Programming Manual (TSC)](https://shop.mediaform.de/media/wysiwyg/downloads/armilla/TSC_TSPL_TSPL2_Programming.pdf)

### Label setup

```
SIZE 100 mm, 50 mm      ' physical label size
GAP 2 mm, 0 mm           ' gap between labels (0 mm offset)
DIRECTION 0               ' print direction: 0 = normal, 1 = mirrored/upside-down
CLS                        ' clear the image buffer before drawing
```

Emitted in `build_tspl()` at `barcode_printer.rs:441-444`.

### Text — `TEXT`

```
TEXT x,y,"font",rotation,x-mult,y-mult[,align],"content"
```

| Field | Meaning |
|---|---|
| `x,y` | top-left corner of the text, in dots |
| `"font"` | built-in font id: `"1"` (8×12), `"2"` (12×20), `"3"` (16×24, extendable), or `"0"` for the stretchable ASCII font |
| `rotation` | `0`, `90`, `180`, `270` — counter-clockwise |
| `x-mult`, `y-mult` | **integer scale multipliers, 1–10.** This is how you make text bigger without switching fonts — `TEXT 40,40,"3",0,**2**,**2**,"BIG"` prints Font 3 at 2× width and 2× height. Only font `"0"` supports true stretch scaling on both axes; fonts `"1"`–`"3"` are still readable scaled but blockier. |
| `align` (optional) | `0`/`1` left, `2` center, `3` right — lets the printer center text within the label width itself instead of you pre-computing `x` |
| `"content"` | the text, in double quotes |

Our code always calls this with `x-mult=1, y-mult=1` and pre-computed
`x`/`y` for centering (`barcode_printer.rs:467-472` for the name lines,
`:475-480` for the price line) — the app centers text itself via
`LabelLayout` rather than relying on the printer's `align` field. **To make
the price line even bigger than swapping fonts allows, bump its
`y-mult`/`x-mult` above `1`** instead of only picking a larger font id.

**New line:** TSPL has no `\n` inside a single `TEXT` command — each line is
a *separate* `TEXT` command at an incremented `y`. That's why
`LabelLayout::compute()` (`barcode_printer.rs:214`) pre-wraps long names into
multiple `TextLine { text, x, y }` entries and the builder loops over them
(`barcode_printer.rs:467-472`), each one `chosen_font.h + line_gap` dots
below the last.

**Bigger text — 3 ways, in order of what this codebase uses:**
1. Pick a larger built-in font id (`"1"` → `"2"` → `"3"`) — what `FONTS` in
   `barcode_printer.rs:198-202` does today.
2. Increase `x-mult`/`y-mult` on the same font (not currently used, but
   valid — e.g. `"3",0,2,2` doubles Font 3 in both dimensions).
3. Use font `"0"` with a TrueType font downloaded to the printer (`TT`
   command) plus explicit point size — overkill for this app.

### Barcode — `BARCODE`

```
BARCODE x,y,"type",height,readable,rotation,narrow,wide,"data"
```

`readable` is `0`/`1` — whether the printer prints its own human-readable
text under the bars. We always pass `0` (`barcode_printer.rs:456-463`,
verified by `test_build_tspl_readable_is_zero`) because we draw our own
`TEXT` line instead, so we control its font/position precisely.

### QR — `QRCODE`

```
QRCODE x,y,ECC,cell_size,mode,rotation,model,mask,"data"
```

We use `M` (medium error correction), model `M2`, mask `S3`
(`barcode_printer.rs:449-452`).

### Print

```
PRINT count,copies
```

`PRINT 1,3` prints 1 unique label, 3 copies. (`copies` = number of physical
labels output.)

### Centering, in general

TSPL's `align` field on `TEXT`/`BLOCK` can center for you, but this codebase
computes `x` manually in `LabelLayout::compute()` so all three protocols
(TSPL/ZPL/EPL) share one centering algorithm instead of using each
protocol's own (differently-behaved) native centering.

---

## 2. ZPL (Zebra printers)

Reference: [ZPL Command Reference (gist)](https://gist.github.com/haakym/0c788bdd5a7e6580617e524ebe7c0916), [ZPLPDF ZPL reference](https://zplpdf.com/en/zpl-reference), [Labelary ZPL intro](https://labelary.com/zpl.html)

Every label is wrapped `^XA ... ^XZ` (start/end format) —
`barcode_printer.rs:568` and `:606`.

### Position — `^FO`

```
^FOx,y
```

Sets the origin for the *next* field only (not persistent) — every text or
barcode command needs its own `^FO` right before it.

### Text — `^A` + `^FD...^FS`

```
^A0N,height,width^FDyour text^FS
```

| Field | Meaning |
|---|---|
| `^A0` | font `0` — the built-in scalable font (what we use everywhere: `barcode_printer.rs:592`, `:600`, `:626`, `:636`). ZPL also has fixed bitmap fonts `^AA`–`^AZ`. |
| `N` | orientation: `N` normal, `R` 90° CW, `I` 180°, `B` 270° CW |
| `height,width` (in dots) | **this is where "bigger text" lives in ZPL** — font `0` is infinitely scalable, so making the price line bigger is just passing a larger `height`/`width` pair, no font-swap needed. Our `FONTS` table's `h`/`char_w` (`barcode_printer.rs:198-202`) feed directly into these two numbers. |
| `^FD...^FS` | the field data (text content) and field separator (end of field) |

**New line in ZPL:** like TSPL, `^A`/`^FD` has no line-break escape — each
line is its own `^FO` + `^A0` + `^FD...^FS` block. We loop over
`l.text_lines` the same way as TSPL (`barcode_printer.rs:589-595`).

### Centering — `^FB` (Field Block)

```
^FBwidth,max_lines,line_spacing,justification,hanging_indent
```

This is the one place we lean on the *printer's* native centering instead of
pre-computed `x`: `^FO0,y ^FB{total_w},1,0,C,0 ^FD...^FS`
(`barcode_printer.rs:591-594`) — `justification=C` centers the text within a
`total_w`-dot-wide block starting at `x=0`, so we don't need to measure
string width for ZPL specifically (`char_w` estimation is still used for the
*shared* wrap/shrink logic in `LabelLayout`, but not for final ZPL x-position).
Other `justification` values: `L` left, `R` right, `J` justified.

### Barcode — `^BY` + `^BC` (Code 128)

```
^BYnarrow_width
^BCorientation,height,print_interpretation_line,print_above,check_digit
```

We pass `N,N,N` for the last three — no human-readable line, nothing
printed above the bars, no check digit — same "we draw text ourselves"
reasoning as TSPL (`barcode_printer.rs:579-585`).

### QR — `^BQ`

```
^BQorientation,model,magnification
```

then `^FD` with a mode char: `^FDMM,A<data>^FS` — `M` = auto error
correction, `A` = alphanumeric (`barcode_printer.rs:572-575`).

### Copies — `^PQ`

```
^PQquantity
```

`barcode_printer.rs:605`.

---

## 3. EPL / EPL2 (older Zebra/Eltron printers)

Reference: [EPL2 Programmer's Manual (Zebra)](https://dl.waspbarcode.com/kb/printer/epl2-programmer-manual.pdf)

Simplest of the three — no explicit `SIZE`/label-start command needed beyond
`N` (clear image buffer), `barcode_printer.rs:661`.

### Text — `A`

```
Ax,y,rotation,font,x-mult,y-mult,reverse,"content"
```

| Field | Meaning |
|---|---|
| `font` | `1`–`5`, built-in monospace bitmap fonts (we map via `epl_font` in `FontMetrics`, `barcode_printer.rs:198-202`) |
| `x-mult`, `y-mult` | same idea as TSPL — integer multipliers on the chosen font. We always pass `1,1` (`barcode_printer.rs:674`, `:682`) and rely on font selection alone for sizing; bump these for extra size without a bigger font id. |
| `reverse` | `N`/`R` — `R` prints white-on-black (inverted block behind the text) |
| `rotation` | `0`/`1`/`2`/`3` = `0°`/`90°`/`180°`/`270°` |

**New line:** same story — one `A` command per line, no embedded `\n`
(`barcode_printer.rs:672-677`).

### Barcode — `B`

```
Bx,y,rotation,type,narrow,wide,height,human_readable,"data"
```

`type=1` is Code128 (`barcode_printer.rs:665`); `human_readable` is `N`/`Y`
— we pass `N` for the same "we draw our own text" reason.

### Print — `P`

```
Pcopies
```

`barcode_printer.rs:687`.

### Centering

EPL has no native block-centering command like ZPL's `^FB` — x/y must
always be pre-computed, exactly like our TSPL path. This is why
`LabelLayout` computes one shared `x` per line that both TSPL and EPL reuse
directly.

---

## 4. Shared layout engine (`LabelLayout::compute`)

All three builders call the *same* `LabelLayout::compute()`
(`barcode_printer.rs:214-420`) before emitting any protocol-specific
commands, so "make text bigger / new line / center" behavior is consistent
across TSC and Zebra hardware. Key mechanics:

- **Auto-shrink** (`:242-252`): starts at the "ideal" font for the label
  width, and steps down through `FONTS` (Large → Medium → Small) until the
  text fits on one line, or bottoms out at Small.
- **Word wrap** (`:254-283`): if even the smallest font doesn't fit on one
  line, splits on whitespace into multiple `TextLine`s at that font size —
  this is the "new line" logic; single words longer than the label just clip
  rather than looping forever.
- **Price line sizing** (`:291-306`): starts one font size *larger* than
  whatever the product name landed on (capped at Large — there's nothing
  bigger), then shrinks only if the price string itself doesn't fit. This is
  the mechanism behind "price is bigger than the name."
- **Vertical centering** (`:326-340`): barcode height is capped at 55% of
  the printable area; the whole content block (barcode + text lines + price
  line) is centered as one unit in the remaining space.
- **Horizontal centering** (`:352-397`): computed per-line from estimated
  character width (`FontMetrics.char_w`) — this is what feeds TSPL/EPL's
  literal `x`, and what ZPL ignores in favor of native `^FB ... C` centering.

### `FONTS` table (`barcode_printer.rs:198-202`)

```rust
const FONTS: [FontMetrics; 3] = [
    FontMetrics { tspl: "3", epl: 4, h: 24, char_w: 16 }, // Large
    FontMetrics { tspl: "2", epl: 3, h: 20, char_w: 12 }, // Medium
    FontMetrics { tspl: "1", epl: 1, h: 12, char_w: 8 },  // Small
];
```

To add a size (e.g. an even-bigger "XL" for price-only labels), add an entry
here with its TSPL font id, EPL font id, pixel height (`h`), and estimated
per-character width (`char_w`, used only for the shrink/wrap/centering
math — ZPL doesn't need `char_w` since it uses `^FB` centering and scalable
font `0`). `char_w` is a rough monospace estimate; if real labels come out
mis-centered on TSPL/EPL, tightening this constant is the first thing to
check.

---

## Quick answers

- **"How do I make text bigger?"** — TSPL/EPL: bump `x-mult`/`y-mult` on the
  `TEXT`/`A` command (1–10), or pick a bigger font id. ZPL: just pass a
  bigger `height,width` to `^A0N,h,w` — font `0` is infinitely scalable, no
  multiplier needed.
- **"How do I go to a new line?"** — None of the three protocols support
  `\n` inside one text command. Emit a second text command (`TEXT`/`^A0.../A`)
  at `y + line_height + gap`. `LabelLayout.text_lines` / `.price_line`
  already do this — see `total_text_h`/`price_gap` math above.
- **"How do I center text?"** — TSPL/EPL: compute `x = margin + (printable_w - text_width) / 2`
  yourself (what `LabelLayout` does). ZPL: use `^FB{width},1,0,C,0` and let
  the printer center it — no manual `x` math needed.
- **"How do I rotate text?"** — TSPL: `rotation` field on `TEXT` (`0/90/180/270`,
  CCW). ZPL: orientation char on `^A` (`N/R/I/B`). EPL: `rotation` field on
  `A` (`0/1/2/3`). Not currently used anywhere in this codebase — all labels
  print at `0`/`N`.

---

Sources:
- [TSPL/TSPL2 Programming Manual (TSC/mediaform)](https://shop.mediaform.de/media/wysiwyg/downloads/armilla/TSC_TSPL_TSPL2_Programming.pdf)
- [TSPL/TSPL2 Programming Manual (ALTEC mirror)](https://files.altec.nl/20240314091328/tspl_tspl2_programming_2023_1_9_ALTEC.pdf)
- [ZD100/ZD230/ZD888 TSPL Programming Guide (Zebra)](https://www.zebra.com/content/dam/support-dam/en/documentation/unrestricted/guide/software/zd100series-zd230series-zd888series-proman-en.pdf)
- [ZPL Command Reference (gist)](https://gist.github.com/haakym/0c788bdd5a7e6580617e524ebe7c0916)
- [ZPL Reference 2026 — ZPLPDF](https://zplpdf.com/en/zpl-reference)
- [ZPL Commands Reference 2026 — ZPLPDF](https://zplpdf.com/en/common-zpl-commands)
- [An Introduction to ZPL — Labelary](https://labelary.com/zpl.html)
- [^FO ZPL Command: Field Origin Explained — Label Toolkit](https://www.labeltoolkit.com/blog/zpl-fo-field-origin-command/)
- [EPL2 Programmer's Manual (Zebra, via WASP)](https://dl.waspbarcode.com/kb/printer/epl2-programmer-manual.pdf)
- [EPL Programming Guide (Zebra, via servopack)](https://www.servopack.de/support/zebra/EPL2_Manual.pdf)
