// The stylesheet half of the gate for web/AGENTS.md, "Components and modularity": no literal colour
// or size (rawValue.ts) in src/**/*.css outside src/theme/, which holds the measures. A genuine layout
// constant that no token can carry (a media query cannot read a custom property) is allowed by a
// comment holding `literal-ok: <reason>` on its line or on the lines just before its declaration.
//
//   node scripts/lint-css.ts        Exit 1 and list each literal when one is found.
import {readdirSync, readFileSync} from 'node:fs';
import {dirname, join, relative} from 'node:path';
import {fileURLToPath} from 'node:url';
import {RAW_VALUE} from './rawValue.ts';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const EXEMPT = 'src/theme/';
const ALLOW = 'literal-ok:';

/** Each literal in `css` as `line: text`, skipping comments and declarations marked ALLOW. */
export function literalsIn(css: string): string[] {
  const found: string[] = [];
  let inComment = false;
  let allowed = false;
  css.split('\n').forEach((line, index) => {
    // The line without its comments, and whether one of them is an allow marker.
    let code = '';
    let marked = false;
    for (let at = 0; at < line.length; ) {
      if (inComment) {
        const end = line.indexOf('*/', at);
        const comment = line.slice(at, end < 0 ? line.length : end);
        if (comment.includes(ALLOW)) marked = true;
        if (end < 0) break;
        inComment = false;
        at = end + 2;
      } else {
        const start = line.indexOf('/*', at);
        code += line.slice(at, start < 0 ? line.length : start);
        if (start < 0) break;
        inComment = true;
        at = start + 2;
      }
    }
    if (marked) allowed = true;
    if (!allowed && RAW_VALUE.test(code)) found.push(`${index + 1}: ${line.trim()}`);
    // A marker covers its declaration or block opener, which ends at `;` or `{`.
    if (allowed && /[;{]/.test(code)) allowed = false;
  });
  return found;
}

function stylesheets(dir: string): string[] {
  return readdirSync(dir, {withFileTypes: true}).flatMap(entry => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return stylesheets(path);
    return entry.name.endsWith('.css') ? [path] : [];
  });
}

if (import.meta.main) {
  let count = 0;
  for (const path of stylesheets(join(ROOT, 'src'))) {
    const name = relative(ROOT, path);
    if (name.startsWith(EXEMPT)) continue;
    for (const literal of literalsIn(readFileSync(path, 'utf8'))) {
      console.error(`${name}:${literal}  colours and sizes come from the theme tokens or src/theme/`);
      count++;
    }
  }
  process.exit(count === 0 ? 0 : 1);
}
