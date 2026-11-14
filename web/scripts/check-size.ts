// The frontend size gate: gzip sizes of the built entry chunk and of all JS and CSS, against the
// limits in REQUIREMENTS.md ("Numbers"), which stays the one place the numbers are written.
//
//   node scripts/check-size.ts            after `pnpm build`
//
// Exit 1 when a size is over its limit, 2 when the check cannot run (no build, no limit row).
import {readFileSync, readdirSync} from 'node:fs';
import {join} from 'node:path';
import {gzipSync} from 'node:zlib';

const WEB = new URL('..', import.meta.url).pathname;
const DIST = join(WEB, 'dist');
const REQUIREMENTS = join(WEB, '..', 'REQUIREMENTS.md');
const ROWS = {entry: 'Frontend entry chunk, gzip', total: 'Frontend total JS + CSS, gzip'};

function fail(message: string): never {
  console.error(`check-size: ${message}`);
  process.exit(2);
}

/** The Limit cell of a Numbers row, in bytes: "320,000 bytes". */
function limitOf(requirements: string, row: string): number {
  const line = requirements.split('\n').find(text => text.startsWith(`| ${row} |`));
  const match = line?.split('|')[2]?.match(/^\s*([\d,]+) bytes\s*$/);
  if (match?.[1] == null) fail(`no "N bytes" limit in the REQUIREMENTS.md row "${row}"`);
  return Number(match[1].replaceAll(',', ''));
}

function gzipped(path: string): number {
  return gzipSync(readFileSync(path), {level: 9}).length;
}

let html: string;
let assets: string[];
try {
  html = readFileSync(join(DIST, 'index.html'), 'utf8');
  assets = readdirSync(join(DIST, 'assets')).filter(name => /\.(js|css)$/.test(name));
} catch {
  fail('no build in web/dist; run `pnpm build` first');
}
const entry = html.match(/<script type="module"[^>]* src="\/assets\/([^"]+\.js)"/)?.[1];
if (entry == null || assets.length === 0) fail('no entry script in web/dist/index.html');

const requirements = readFileSync(REQUIREMENTS, 'utf8');
const sizes = {
  entry: gzipped(join(DIST, 'assets', entry)),
  total: assets.reduce((sum, name) => sum + gzipped(join(DIST, 'assets', name)), 0),
};
let over = false;
for (const key of ['entry', 'total'] as const) {
  const limit = limitOf(requirements, ROWS[key]);
  const ok = sizes[key] <= limit;
  over ||= !ok;
  console.log(`${ok ? 'ok  ' : 'OVER'} ${ROWS[key]}: ${sizes[key].toLocaleString('en')} of ${limit.toLocaleString('en')} bytes`);
}
process.exit(over ? 1 : 0);
