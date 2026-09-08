<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# FerroCHART brand

This directory is the authority for the FerroCHART mark and palette. The
FerroHEALTH family site copies from here and never edits its copy, so a change
to the mark is made in this repository and the copy refreshed
(`website/landing/assets/products/README.md` there records where each copy came
from).

FerroCHART follows the file set, naming, variant list, and raster pipeline the
other three products use.

## The mark

A form, filled in and checked: two rows and a tick, inside a sheet.

The rows are the fields an operational template compiles to, one full and one
shorter, because a form is a set of fields of different lengths and that is
what makes it legible as a form rather than a document. The tick is the
validation that happens before a COMPOSITION is built. That is the part a
clinician actually feels, because it is the difference between an error on the
field they got wrong and one rejection for the whole document.

**No vertical strokes.** The first drawing put a text caret beside the middle
row. At 32 px and below the caret, the row above it and the row beside it read
as letters, which is why it is gone. Any future edit that adds a vertical
stroke inside the sheet will do the same thing; check it at 16 px before
keeping it.

## The palette, "Rose & Iron"

Rose is the hue no other product in the family owns. It was chosen by measuring
rather than by eye: every hue was scored for its worst-case CIEDE2000 distance
from FerroEHR's rust, FerroTERM's teal, and FerroBRIDGE's indigo, in both
themes, under normal, deuteranope, and protanope vision. Rose at 330 degrees
won at a worst case of 14.9. Green and violet both fell below 10, which is why
the obvious next-colour choices are not here.

Every value carries its measured contrast against both grounds, and what that
ratio permits. "Text" is 4.5:1 or better, "graphics" is 3:1 or better and
covers a mark, a rule, or a large heading, and "no" means the pair is not
usable. Read the figure before using a token somewhere new.

| Token | Value | On surface `#f8fafc` | On tile `#0b1220` |
|---|---|---|---|
| `--ferrochart-rose` | `#C2185B` | 5.61, text | 3.19, graphics |
| `--ferrochart-rose-deep` | `#9D1E52` | 7.32, text | 2.44, no |
| `--ferrochart-rose-light` | `#E04F84` | 3.57, graphics | 5.01, text |
| `--ferrochart-blush` | `#F48FB1` | 2.13, no | 8.39, text |

So the mark colour and the text colour are not the same value on either ground,
which is why `tokens.css` carries `--ferrochart-brand` and
`--ferrochart-brand-text` separately. Reaching for `--ferrochart-brand` to
colour a link on a light ground gives 5.61 and is fine; doing it on a dark
ground gives 3.19 and is not.

### The interface tokens derived from it

The renderer's stylesheet (`app/ferrochart-renderer/style/tailwind.css`) takes
its accent from the four values above and derives the pairs an interface
needs: a tinted fill with text on it, and a solid fill with text on it. Those
derived values are measured the same way, and
`app/ferrochart-renderer/src/tokens.rs` re-measures every one of them from the
stylesheet at test time, so a figure here cannot drift from the colour that
ships.

| Pair | Light | Dark | Bar |
|---|---|---|---|
| accent on the page ground | `#C2185B` on `#f8fafc`, 5.61 | `#F48FB1` on `#0f172a`, 8.00 | text |
| accent on a raised panel | `#C2185B` on `#ffffff`, 5.87 | `#F48FB1` on `#1e293b`, 6.56 | text |
| accent-ink on accent-subtle | `#9D1E52` on `#FCE4EC`, 6.37 | `#F48FB1` on `#4a0f27`, 6.78 | text |
| on-accent on solid accent | `#ffffff` on `#C2185B`, 5.87 | `#3f0d22` on `#F48FB1`, 7.28 | text |

**The dark accent is blush, not rose-light.** The table above measures
rose-light at 5.01 against the iron tile, and that figure holds. The interface
raises its panels to `#1e293b`, where the same value reads 3.92 and can no
longer carry text, so the dark theme spends blush where the brand spends
rose-light. A mark drawn on the tile still uses rose-light; a token that has
to carry a word does not.

## Files

| File | What it is |
|---|---|
| `ferrochart-icon.svg` | the mark, for light or quiet surfaces |
| `ferrochart-icon-dark.svg` | the mark on the iron tile, for busy or light-photographic grounds |
| `ferrochart-icon-mono.svg` | one colour, taken from the surrounding text |
| `ferrochart-lockup-auto.svg` | mark and wordmark, switching on `prefers-color-scheme` |
| `ferrochart-lockup-light.svg`, `ferrochart-lockup-dark.svg` | the fixed-theme lockups |
| `favicon.svg` | the favicon master, rows and tick in mist on a rose tile |
| `favicon-32.png`, `favicon-16.png`, `favicon.ico` | raster favicons |
| `apple-touch-icon.png` | the 180x180 home-screen icon |
| `ferrochart-social.svg`, `ferrochart-social.png` | 1200x630 social card |
| `tokens.css` | the palette as CSS custom properties |

### Where each file is used

`scripts/site/assemble.sh` names the files the published site needs, one by
one, so a missing file fails the assembly instead of publishing a page with a
broken mark. Nothing outside this directory holds a copy: the landing page
links `tokens.css` rather than restating its hex values, and the book's
`theme/favicon.svg` and `theme/favicon.png` are staged from here at assembly
time and are not committed.

| File | Used by |
|---|---|
| `favicon.svg`, `favicon-32.png`, `favicon-16.png` | the landing page's icon links, and the book's tab icon |
| `favicon.ico` | served at `/favicon.ico`, for a client that asks for it directly |
| `apple-touch-icon.png` | the landing page, for a page saved to a home screen |
| `ferrochart-icon.svg` | the landing page header |
| `ferrochart-social.png` | the landing page's `og:image` |
| `tokens.css` | linked by the landing page ahead of its own stylesheet |
| `ferrochart-lockup-auto.svg` | the repository README |

Five files are used by nothing here, deliberately:

- `ferrochart-icon-dark.svg` and `ferrochart-lockup-light.svg`,
  `ferrochart-lockup-dark.svg` exist for a surface that cannot run a
  `prefers-color-scheme` query. The auto lockup covers both themes on any
  surface that can, which is every one this project publishes, so the fixed
  pair is for a third-party listing, a slide, or a raster export.
- `ferrochart-icon-mono.svg` is for a surface that recolours the mark from the
  surrounding text.
- `ferrochart-social.svg` is the editable master of `ferrochart-social.png`;
  the raster is what a social card fetcher reads.

The GitHub repository social preview is an owner setting rather than a file in
the tree, so `ferrochart-social.png` is uploaded there by hand.

## Intrinsic size

Every icon declares `width` and `height` of **512** beside its
`viewBox="0 0 64 64"`. The viewBox is what the artwork is drawn in; the two
attributes are what a consumer that rasterizes the file takes as its natural
size. `favicon.svg` stays at 32, because a favicon wants a small natural size.
The lockups keep their own natural width.

## Typography

The wordmark is set in Bricolage Grotesque (700) with a system-sans fallback
stack, which is what the rest of the family uses. Outline the wordmark to paths
before any print use, so the artwork carries no font dependency.

The lockup box is 244 wide for a wordmark that occupies about 137 units in
Bricolage Grotesque. That surplus is deliberate: the fallback stack sets the
same string about 25 percent wider, and a box sized for the intended font clips
the wordmark on every machine that does not have it.

## Usage

- Keep clear space around the mark equal to the height of one row.
- The mark reads down to 24 px. Below that use `favicon.svg`, which drops the
  sheet outline, because a rounded rectangle inside a rounded tile turns to
  mush at 16 px.
- Put the colour mark on light or quiet surfaces, and
  `ferrochart-icon-dark.svg` on busy or light-photographic backgrounds.
- Use `ferrochart-icon-mono.svg` where one colour is required.
- Do not recolour the mark outside this palette, add a vertical stroke inside
  the sheet, stretch it, add effects, or rebuild the wordmark in another
  typeface.
- Rose is FerroCHART's alone. Do not spend it on FerroHEALTH surfaces except
  where FerroCHART is being named.
- The social card says the project is in its design phase. Keep that true: if
  the card ever implies a download that does not exist, it is wrong.

## Regenerating the rasters

The PNG and ICO files derive from the SVGs. Run these from the repository root
after any change to `favicon.svg` or `ferrochart-social.svg`:

```bash
rsvg-convert -w 32 -h 32 assets/brand/favicon.svg -o assets/brand/favicon-32.png
rsvg-convert -w 16 -h 16 assets/brand/favicon.svg -o assets/brand/favicon-16.png
rsvg-convert -w 48 -h 48 assets/brand/favicon.svg -o /tmp/favicon-48.png
magick /tmp/favicon-48.png assets/brand/favicon-32.png assets/brand/favicon-16.png assets/brand/favicon.ico
rsvg-convert -w 180 -h 180 assets/brand/favicon.svg -o assets/brand/apple-touch-icon.png
rsvg-convert -w 1200 -h 630 assets/brand/ferrochart-social.svg -o assets/brand/ferrochart-social.png
```

## Trademarks

openEHR is a registered trademark of the openEHR Foundation. HL7 and FHIR are
registered trademarks of Health Level Seven International. Neither organisation
endorses FerroCHART.
