// The Content Security Policy sources for the two `<style>` elements Astryx inserts at run time:
// CodeBlock's highlight colours and the layer slide rules (docs/architecture.md, "Serving a page").
// The reading page's policy admits exactly these texts by their sha256 hash, never 'unsafe-inline'.
// The texts come from the Astryx modules the bundle is built from, run here as the page runs them.
//
//   node scripts/style-hashes.ts     after `vite build`; `pnpm build` runs both
//
// Writes dist/style-src.txt, one `'sha256-...'` source per line, which the server's build embeds
// (crates/wastebin_server/build.rs) and scripts/shoot.ts serves. A hash that does not match what the
// page inserts shows as a policy violation in shoot.ts. Also writes dist/svg-style-src.txt, one line
// per SVG file with `<style>` elements: its route and the hash of each, which the server admits on
// that file's own response only. Exit 1 when the bundle lacks one of the injectors, 2 when the hashes
// cannot be written.
import {createHash} from 'node:crypto';
import {readFileSync, readdirSync, writeFileSync} from 'node:fs';
import {dirname, join, sep} from 'node:path';
import {fileURLToPath} from 'node:url';

export const DIST = join(dirname(fileURLToPath(import.meta.url)), '..', 'dist');
export const STYLE_SRC_FILE = 'style-src.txt';
export const SVG_STYLE_SRC_FILE = 'svg-style-src.txt';

/** Each run-time style: the Astryx module that inserts it and a string the bundle has only when that module is in it. */
const STYLES = [
  {name: 'CodeBlock highlight colours', module: 'CodeBlock/highlightStyles.js', marker: 'data-astryx-highlight-styles'},
  {name: 'layer slide rules', module: 'Layer/layerSlideRules.js', marker: 'astryx-layer-slide-rules'},
] as const;

/** A CSP hash source for `text`, as the browser computes it over the element's text. */
export function cspHash(text: string): string {
  return `'sha256-${createHash('sha256').update(text, 'utf8').digest('base64')}'`;
}

/** The styles whose injector `bundle` (the built JavaScript) does not contain. */
export function missingInjectors(bundle: string): string[] {
  return STYLES.filter(style => !bundle.includes(style.marker)).map(style => style.name);
}

/** The text of each `<style>` element in `svg`, as the browser hashes it. */
export function svgStyleTexts(svg: string): string[] {
  return [...svg.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/g)].map(match => match[1]!);
}

let texts: Promise<string[]> | undefined;

/** The text of each style, in STYLES order. The highlight module inserts its element once per process, so this runs once. */
export function styleTexts(): Promise<string[]> {
  texts ??= (async () => {
    const dist = dirname(fileURLToPath(import.meta.resolve('@astryxdesign/core')));
    const [highlight, layer] = STYLES.map(style => join(dist, style.module));
    const inserted: string[] = [];
    const real = Object.getOwnPropertyDescriptor(globalThis, 'document');
    // The highlight module writes its element into `document.head`; a stand-in document records the text.
    Object.defineProperty(globalThis, 'document', {
      configurable: true,
      value: {createElement: () => ({setAttribute() {}, textContent: ''}), head: {appendChild: (element: {textContent: string}) => inserted.push(element.textContent)}},
    });
    try {
      (await import(highlight!)).ensureHighlightStyles();
    } finally {
      if (real == null) Reflect.deleteProperty(globalThis, 'document');
      else Object.defineProperty(globalThis, 'document', real);
    }
    const slideRules = (await import(layer!)).slideRulesCss() as string;
    if (inserted.length !== 1 || inserted[0] == null) throw new Error('the highlight module inserted no style');
    return [inserted[0], slideRules];
  })();
  return texts;
}

async function main() {
  let bundle: string;
  try {
    const assets = join(DIST, 'assets');
    bundle = readdirSync(assets)
      .filter(file => file.endsWith('.js'))
      .map(file => readFileSync(join(assets, file), 'utf8'))
      .join('\n');
  } catch (error) {
    console.error(`style-hashes: no build in ${DIST}: ${String(error)}`);
    process.exit(2);
  }
  const missing = missingInjectors(bundle);
  if (missing.length > 0) {
    console.error(`style-hashes: the bundle has no ${missing.join(', ')}; check the Astryx version and STYLES`);
    process.exit(1);
  }
  const sources = (await styleTexts()).map(cspHash);
  writeFileSync(join(DIST, STYLE_SRC_FILE), `${sources.join('\n')}\n`);
  console.log(`style-hashes: ${sources.length} sources in ${STYLE_SRC_FILE}`);
  const svgs = readdirSync(DIST, {recursive: true, encoding: 'utf8'})
    .filter(file => file.endsWith('.svg'))
    .map(file => ({route: `/${file.split(sep).join('/')}`, sources: svgStyleTexts(readFileSync(join(DIST, file), 'utf8')).map(cspHash)}))
    .filter(svg => svg.sources.length > 0);
  writeFileSync(join(DIST, SVG_STYLE_SRC_FILE), svgs.map(svg => `${[svg.route, ...svg.sources].join(' ')}\n`).join(''));
  console.log(`style-hashes: ${svgs.length} SVG files with styles in ${SVG_STYLE_SRC_FILE}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch(error => {
    console.error(error);
    process.exit(2);
  });
}
