// Screenshots and layout checks of the reading page, against the production build (`pnpm build` first).
// Serves dist/ with a fixture paste embedded the way the server's shell embeds it, then for each shot
// checks that the page never scrolls sideways and that wide tables scroll inside their own box.
//
//   node scripts/shoot.ts <out-dir>        ZPASTE_FIXTURE=<file.md> picks the paste; CHROME=<path> the browser.
//
// Exit 1 when a check fails, 2 when it cannot run.
import {createServer, type Server} from 'node:http';
import {mkdirSync, readFileSync} from 'node:fs';
import {extname, join, normalize} from 'node:path';
import {chromium, type Page} from '@playwright/test';
import {fixturePaste, withEmbeddedPaste} from './fixture.ts';

const DIST = new URL('../dist/', import.meta.url).pathname;
const TYPES: Record<string, string> = {'.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2', '.woff': 'font/woff'};
const SHOTS = [
  {name: '1280-light', width: 1280, height: 800, scheme: 'light'},
  {name: '390-dark', width: 390, height: 844, scheme: 'dark'},
  {name: '390-light', width: 390, height: 844, scheme: 'light'},
  {name: '1280-dark', width: 1280, height: 800, scheme: 'dark'},
] as const;
const WIDTHS = [320, 360, 390, 768, 1024, 1280];

function serve(page: string): Promise<Server> {
  const server = createServer((request, response) => {
    const path = normalize(new URL(request.url ?? '/', 'http://x').pathname);
    if (path.startsWith('/assets/')) {
      try {
        const body = readFileSync(join(DIST, path));
        response.writeHead(200, {'content-type': TYPES[extname(path)] ?? 'application/octet-stream'}).end(body);
      } catch {
        response.writeHead(404).end();
      }
      return;
    }
    response.writeHead(200, {'content-type': 'text/html; charset=utf-8'}).end(page);
  });
  return new Promise(resolve => server.listen(0, '127.0.0.1', () => resolve(server)));
}

const failures: string[] = [];
function check(condition: boolean, message: string) {
  if (!condition) failures.push(message);
}

async function pageOverflow(page: Page): Promise<number> {
  return page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
}

async function main() {
  const out = process.argv[2];
  if (out == null) {
    console.error('usage: node scripts/shoot.ts <out-dir>');
    process.exit(2);
  }
  mkdirSync(out, {recursive: true});
  const html = withEmbeddedPaste(readFileSync(join(DIST, 'index.html'), 'utf8'), fixturePaste());
  const server = await serve(html);
  const address = server.address();
  const url = typeof address === 'object' && address != null ? `http://127.0.0.1:${address.port}/` : '';
  const browser = await chromium.launch({executablePath: process.env.CHROME ?? '/usr/bin/google-chrome-stable'});
  try {
    for (const shot of SHOTS) {
      const context = await browser.newContext({viewport: {width: shot.width, height: shot.height}, colorScheme: shot.scheme, deviceScaleFactor: 2});
      const page = await context.newPage();
      page.on('pageerror', error => failures.push(`${shot.name}: page error: ${error.message}`));
      await page.goto(url, {waitUntil: 'networkidle'});
      await page.evaluate(() => document.fonts.ready);
      check((await page.locator('h1').count()) === 1, `${shot.name}: expected one h1`);
      await page.screenshot({path: join(out, `${shot.name}.png`)});

      const table = page.locator('.astryx-markdown table').first();
      if ((await table.count()) > 0) {
        await table.scrollIntoViewIfNeeded();
        await page.evaluate(() => window.scrollBy(0, -80));
        await page.screenshot({path: join(out, `${shot.name}-table.png`)});
      }
      const code = page.locator('.astryx-markdown .astryx-code-block').first();
      if ((await code.count()) > 0) {
        // The block's start, with its header, below the top bar; the outline marker settles first.
        await code.evaluate(element => element.scrollIntoView({block: 'start'}));
        await page.evaluate(() => window.scrollBy(0, -160));
        await page.waitForTimeout(400);
        await page.screenshot({path: join(out, `${shot.name}-code.png`)});
      }
      if (shot.width < 1024) {
        await page.evaluate(() => window.scrollTo(0, 0));
        await page.getByRole('button', {name: 'Outline'}).click();
        const drawer = page.getByRole('dialog');
        await drawer.waitFor();
        await page.waitForTimeout(400);
        await page.screenshot({path: join(out, `${shot.name}-outline.png`)});
        // Touch targets of at least 44 px (the theme's phone sizes, web/AGENTS.md "Accessibility").
        const short = await drawer.getByRole('link').evaluateAll(links => links.filter(link => link.getBoundingClientRect().height < 44).length);
        check(short === 0, `${shot.name}: ${short} outline entries in the drawer are shorter than 44 px`);
        const entry = drawer.getByRole('link').nth(3);
        const target = decodeURIComponent((await entry.getAttribute('href'))?.slice(1) ?? '');
        await entry.click();
        await drawer.waitFor({state: 'hidden'});
        await page.waitForTimeout(800);
        const top = await page.evaluate(id => document.getElementById(id)?.getBoundingClientRect().top ?? -1, target);
        check(top >= 0 && top < shot.height / 3, `${shot.name}: outline entry #${target} left its heading at y=${Math.round(top)}`);
        await page.screenshot({path: join(out, `${shot.name}-outline-jump.png`)});
      }
      await context.close();
    }

    const context = await browser.newContext({viewport: {width: 320, height: 800}});
    const page = await context.newPage();
    await page.goto(url, {waitUntil: 'networkidle'});
    for (const width of WIDTHS) {
      await page.setViewportSize({width, height: 800});
      const overflow = await pageOverflow(page);
      check(overflow <= 0, `${width}px: the page scrolls sideways by ${overflow}px`);
    }
    await page.setViewportSize({width: 390, height: 800});
    const boxes = await page.evaluate(() =>
      [...document.querySelectorAll('.astryx-markdown table')].map(table => {
        let box: HTMLElement | null = table.parentElement;
        while (box != null && getComputedStyle(box).overflowX !== 'auto' && getComputedStyle(box).overflowX !== 'scroll') box = box.parentElement;
        const scrolls = box != null && box.scrollWidth > box.clientWidth;
        // The first column stays at the box's start edge while the box scrolls.
        const cell = table.querySelector('td');
        const before = cell?.getBoundingClientRect().left;
        if (scrolls && box != null) box.scrollLeft = box.scrollWidth;
        const sticks = cell == null || Math.abs((cell.getBoundingClientRect().left ?? 0) - (before ?? 0)) < 1;
        if (scrolls && box != null) box.scrollLeft = 0;
        return {wide: table.scrollWidth > (box?.clientWidth ?? Infinity), scrolls, sticks, inMain: box?.closest('.astryx-markdown') != null};
      }),
    );
    const wide = boxes.filter(box => box.wide);
    check(wide.every(box => box.scrolls && box.inMain), '390px: a wide table does not scroll inside its own box');
    check(wide.every(box => box.sticks), '390px: the first column of a wide table scrolls away');
    console.log(`checked ${WIDTHS.join(', ')} px; ${boxes.length} tables, ${wide.length} wider than the column at 390 px`);
    await context.close();
  } finally {
    await browser.close();
    server.close();
  }
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exit(failures.length === 0 ? 0 : 1);
}

main().catch(error => {
  console.error(error);
  process.exit(2);
});
