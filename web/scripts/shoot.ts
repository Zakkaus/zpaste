// Screenshots and layout checks of the reading page, against the production build (`pnpm build` first).
// Serves dist/ with a fixture paste embedded the way the server's shell embeds it, then for each shot
// checks that the page never scrolls sideways, that wide tables scroll inside their own box with a
// first column of at most 40% of it, that the outline marks the section on screen, and that the phone
// outline drawer takes focus and gives it back.
//
//   node scripts/shoot.ts <out-dir>        ZPASTE_FIXTURE=<file.md> picks the paste; CHROME=<path> the browser.
//
// Exit 1 when a check fails, 2 when it cannot run.
import {createServer, type Server} from 'node:http';
import {mkdirSync, readFileSync} from 'node:fs';
import {extname, join, normalize} from 'node:path';
import {chromium, type Page} from '@playwright/test';
import {en} from '../src/i18n/en.ts';
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
/** Phone widths at which a wide table's fixed first column takes at most FIRST_COLUMN_MAX of its box. */
const FIRST_COLUMN_WIDTHS = [360, 390];
const FIRST_COLUMN_MAX = 0.4;
const OUTLINE = `nav[aria-label="${en['outline.label']}"]`;

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

/** The heading ids each mounted outline (the rail, and the drawer's even while closed) marks current. */
function currentEntries(page: Page): Promise<(string | null)[]> {
  return page.evaluate(
    selector =>
      [...document.querySelectorAll(selector)].map(nav => {
        const link = nav.querySelector<HTMLAnchorElement>('[aria-current]');
        return link == null ? null : decodeURIComponent(link.hash.slice(1));
      }),
    OUTLINE,
  );
}

/**
 * The section on screen, read from the document rather than the outline: the last heading the outline
 * can list (`#` to `####`, drawn as h2 to h5) that has reached the line a jump to it lands on (its
 * scroll-margin-top, below the top bar), else the paste's first heading.
 */
function sectionOnScreen(page: Page): Promise<string> {
  return page.evaluate(() => {
    const headings = [...document.querySelectorAll<HTMLElement>('.astryx-markdown :is(h2, h3, h4, h5)[id]')];
    let current = headings[0]?.id ?? '';
    for (const heading of headings) {
      if (heading.getBoundingClientRect().top > parseFloat(getComputedStyle(heading).scrollMarginTop) + 1) break;
      current = heading.id;
    }
    return current;
  });
}

/** The heading id of entry `index` in the outline. */
function outlineEntry(page: Page, index: number): Promise<string> {
  return page.evaluate(
    ([selector, at]) => decodeURIComponent(document.querySelector(selector)?.querySelectorAll('a')[at]?.hash.slice(1) ?? ''),
    [OUTLINE, index] as const,
  );
}

/** Two frames, so the outline's rAF-throttled scroll tracking has run. */
function settle(page: Page): Promise<void> {
  return page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
}

/** Until the page has not scrolled for 20 frames (a smooth scroll has ended), at most 5 s. */
function scrollStopped(page: Page): Promise<void> {
  return page.evaluate(
    () =>
      new Promise<void>(resolve => {
        const deadline = performance.now() + 5000;
        let last = window.scrollY;
        let still = 0;
        const step = () => {
          still = window.scrollY === last ? still + 1 : 0;
          last = window.scrollY;
          if (still >= 20 || performance.now() > deadline) resolve();
          else requestAnimationFrame(step);
        };
        requestAnimationFrame(step);
      }),
  );
}

async function checkCurrent(page: Page, label: string, expected: string) {
  await settle(page);
  const onScreen = await sectionOnScreen(page);
  check(onScreen === expected, `${label}: the section on screen is #${onScreen}, expected #${expected}`);
  const current = await currentEntries(page);
  check(current.length > 0 && current.every(id => id === onScreen), `${label}: the outline marks ${JSON.stringify(current)}, the section on screen is #${onScreen}`);
}

/**
 * The outline marks the section on screen on first load (the paste's first heading), after the reader
 * scrolls to a later section, and after a jump through a link to a heading (a heading anchor). Run with
 * the page at the top; leaves it there.
 */
async function checkOutlineTracking(page: Page, name: string) {
  const first = await page.evaluate(() => document.querySelector('.astryx-markdown :is(h2, h3, h4, h5)[id]')?.id ?? '');
  await checkCurrent(page, `${name} on load`, first);
  // A later section, scrolled 40 px past the line its heading lands on, so its text is on screen.
  const later = await outlineEntry(page, 2);
  await page.evaluate(id => {
    const heading = document.getElementById(id);
    if (heading == null) return;
    window.scrollBy(0, heading.getBoundingClientRect().top - parseFloat(getComputedStyle(heading).scrollMarginTop) + 40);
  }, later);
  await checkCurrent(page, `${name} after scrolling to #${later}`, later);
  const anchored = await outlineEntry(page, 1);
  await page.evaluate(id => {
    window.location.hash = id;
  }, anchored);
  await scrollStopped(page);
  await checkCurrent(page, `${name} after the heading anchor #${anchored}`, anchored);
  await page.evaluate(() => {
    history.replaceState(null, '', window.location.pathname);
    window.scrollTo(0, 0);
  });
  await settle(page);
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
      await checkOutlineTracking(page, shot.name);
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
      if (shot.width >= 1024) {
        // A click on a rail entry: once the smooth scroll ends, its section is on screen and current.
        const entry = await outlineEntry(page, 3);
        await page.locator(`aside ${OUTLINE} a`).nth(3).click();
        await scrollStopped(page);
        await checkCurrent(page, `${shot.name} after clicking the rail entry #${entry}`, entry);
      } else {
        await page.evaluate(() => window.scrollTo(0, 0));
        await settle(page);
        const opener = page.getByRole('button', {name: en['outline.open']});
        await opener.click();
        const drawer = page.getByRole('dialog');
        await drawer.waitFor();
        await page.waitForTimeout(400);
        // Focus moves into the drawer as it opens and back to its button as it closes.
        check(await drawer.evaluate(dialog => dialog.contains(document.activeElement)), `${shot.name}: focus did not move into the outline drawer`);
        await checkCurrent(page, `${shot.name} with the drawer open at the top`, await outlineEntry(page, 0));
        await page.screenshot({path: join(out, `${shot.name}-outline.png`)});
        await page.keyboard.press('Escape');
        await drawer.waitFor({state: 'hidden'});
        check(await opener.evaluate(button => button === document.activeElement), `${shot.name}: focus did not return to the outline button`);
        await opener.click();
        await drawer.waitFor();
        await page.waitForTimeout(400);
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
        await checkCurrent(page, `${shot.name} after clicking the drawer entry #${target}`, target);
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
    for (const width of FIRST_COLUMN_WIDTHS) {
      await page.setViewportSize({width, height: 800});
      const columns = await page.evaluate(() =>
        [...document.querySelectorAll('.astryx-markdown table')].flatMap(table => {
          const box = table.closest<HTMLElement>('.astryx-table-scroll-wrapper');
          if (box == null || box.scrollWidth <= box.clientWidth) return [];
          const cells = [...table.querySelectorAll('tr > :first-child')];
          const end = Math.max(...cells.map(cell => cell.getBoundingClientRect().right));
          // Copying a cell gives its text as written: wrapping inserted no characters.
          const selection = window.getSelection();
          const altered = cells.filter(cell => {
            const range = document.createRange();
            range.selectNodeContents(cell);
            selection?.removeAllRanges();
            selection?.addRange(range);
            const copied = selection?.toString() ?? '';
            return copied !== cell.textContent || /[\u00ad\u200b]/.test(copied);
          }).length;
          selection?.removeAllRanges();
          return [{ratio: (end - box.getBoundingClientRect().left) / box.clientWidth, altered}];
        }),
      );
      for (const column of columns) {
        check(column.ratio <= FIRST_COLUMN_MAX, `${width}px: a fixed first column takes ${(column.ratio * 100).toFixed(1)}% of its table's box`);
        check(column.altered === 0, `${width}px: ${column.altered} first-column cells copy as different text`);
      }
      console.log(`${width}px: fixed first column ${columns.map(column => `${(column.ratio * 100).toFixed(1)}%`).join(', ') || 'none'} of the table box`);
    }
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
