# Zpaste logo: license and attribution

The Zpaste logo is a derivative of the Gentoo logo through the gentoo-zh logo
and is licensed under [CC BY-SA 2.5](https://creativecommons.org/licenses/by-sa/2.5/),
not under the MIT license that covers Zpaste's code. Keep this notice, or an
equivalent credit, with any copy or further adaptation.

## Files

This notice covers:

- the SVG files in this directory: the signet (`icon-*.svg`), the signet with
  the "Zpaste" lettering (`lockup-*.svg`) and the simplified favicon
  (`favicon-*.svg`), each in a light-page and a dark-page colourway;
- the copies drawn by the web app: `web/src/chrome/Logo.tsx` (the lockup's
  shapes) and `web/src/theme/brand.stylex.ts` (its colours);
- `web/public/favicon.svg` (the favicon in both colourways, switched by the
  OS colour scheme), `web/public/favicon-32.png` and
  `web/public/apple-touch-icon.png`, rendered from the files here.

## Attribution chain

1. **Gentoo logo.** The vector versions of the Gentoo logo are copyright
   Gentoo Foundation and Lennart Andre Rolland, licensed under
   [CC BY-SA 2.5](https://creativecommons.org/licenses/by-sa/2.5/)
   ([original vector signet](https://www.gentoo.org/assets/img/logo/gentoo-signet.svg)).
2. **gentoo-zh logo.** The gentoo-zh community adapted the Gentoo logo, with
   its own colours and composition and the 中 and 文 seal glyphs, under the same
   license ([gentoozh.org logo page](https://gentoozh.org/logo/)).
3. **Zpaste logo.** Adapted from the gentoo-zh artwork for Zpaste in 2026:
   flat colours in place of the gradients, shading and bevels; two cuts in the
   signet as a cue for pasted text; new hand-drawn "Zpaste" lettering (outlines,
   no font); and a simplified favicon without the seal glyphs. Licensed under
   [CC BY-SA 2.5](https://creativecommons.org/licenses/by-sa/2.5/).

The 中 and 文 seal glyphs come from
[中-seal.svg](https://commons.wikimedia.org/wiki/File:%E4%B8%AD-seal.svg) and
[文-seal.svg](https://commons.wikimedia.org/wiki/File:%E6%96%87-seal.svg) on
Wikimedia Commons, whose file pages state: "This SVG representation of an
ancient script is in the public domain."

## Colours

| Colour | Use |
| --- | --- |
| `#514776` | signet and lettering on a light page; seal glyphs on a dark page |
| `#C4BEDB` | signet and lettering on a dark page |
| `#EEEAF4` | seal glyphs on a light page |

## Rendering the PNG files

From `web/`, with librsvg and ImageMagick:

```sh
rsvg-convert -w 32 -h 32 src/brand/favicon-light.svg | magick - -strip public/favicon-32.png
rsvg-convert -w 160 -h 160 -b '#FFFFFF' src/brand/icon-light.svg \
  | magick - -background '#FFFFFF' -gravity center -extent 180x180 -alpha off -strip public/apple-touch-icon.png
```

The touch icon is opaque because iOS fills a transparent one with black.
