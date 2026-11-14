// Screenshots and layout checks of the reading page, against the production build (`pnpm build` first).
// Serves dist/ with a fixture paste embedded the way the server's shell embeds it, then for each shot
// checks that the page never scrolls sideways, that wide tables scroll inside their own box with a
// first column of at most half of it, that no table cell wraps inside a name, that the outline marks
// the section on screen, and that the phone outline drawer takes focus and gives it back. A second,
// generated paste of 1,600 lines (longFixture.ts) checks that an outline taller than the desktop rail
// still follows the page and keeps the current entry in view, and measures scrolling through it.
//
//   node scripts/shoot.ts <out-dir>        ZPASTE_FIXTURE=<file.md> picks the paste; CHROME=<path> the browser.
//
// Exit 1 when a check fails, 2 when it cannot run.
import {createServer, type Server} from 'node:http';
import {mkdirSync, readFileSync} from 'node:fs';
import {extname, join, normalize} from 'node:path';
import {chromium, type Browser, type Page} from '@playwright/test';
import {en} from '../src/i18n/en.ts';
import {embeddedPaste, fixturePaste, withEmbeddedPaste} from './fixture.ts';
import {longMarkdown} from './longFixture.ts';

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
const FIRST_COLUMN_MAX = 0.5;
/** Where the generated long paste is served. */
const LONG_PATH = '/long';
const OUTLINE = `nav[aria-label="${en['outline.label']}"]`;

/** Serves `pages[path]` for each page path (`/` and LONG_PATH) and dist/assets for the rest. */
function serve(pages: Record<string, string>): Promise<Server> {
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
    const page = pages[path];
    if (page == null) response.writeHead(404).end();
    else response.writeHead(200, {'content-type': 'text/html; charset=utf-8'}).end(page);
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

/** Scrolls the page so the heading `id` sits 40 px above the line a jump to it lands on, its text on screen. */
function scrollPast(page: Page, id: string): Promise<void> {
  return page.evaluate(target => {
    const heading = document.getElementById(target);
    if (heading == null) return;
    window.scrollBy(0, heading.getBoundingClientRect().top - parseFloat(getComputedStyle(heading).scrollMarginTop) + 40);
  }, id);
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
  const later = await outlineEntry(page, 2);
  await scrollPast(page, later);
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

/**
 * Where a table cell's text wraps inside a name: not at a space, not after `/` or `-`, and not beside
 * CJK text, which may break between any two characters. Read from the rendered lines: each character's
 * box against the one before it. Only in the fixed first column, and only inside a piece that is on
 * its own wider than the column's cap (`max` of the table's box), is such a wrap allowed; those are
 * listed apart.
 */
function wrapsInsideNames(page: Page, max: number): Promise<{inside: string[]; oversize: string[]}> {
  return page.evaluate(max => {
    const breaksAfter = /[\s/\-\u2010-\u2014]/;
    const cjk = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}\u3000-\u303f\uff00-\uffef]/u;
    const joins = (before: string, after: string) => !breaksAfter.test(before) && !/\s/.test(after) && !cjk.test(before) && !cjk.test(after);
    const inside: string[] = [];
    const oversize: string[] = [];
    const range = document.createRange();
    for (const cell of document.querySelectorAll<HTMLElement>('.astryx-markdown table :is(th, td)')) {
      // The cell's text, and each character's line box and element.
      let text = '';
      const tops: number[] = [];
      const bottoms: number[] = [];
      const parents: (Element | null)[] = [];
      const walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node != null; node = walker.nextNode()) {
        const data = node.textContent ?? '';
        for (let at = 0; at < data.length; at++) {
          range.setStart(node, at);
          range.setEnd(node, at + 1);
          const box = range.getClientRects()[0];
          text += data.charAt(at);
          tops.push(box?.top ?? NaN);
          bottoms.push(box?.bottom ?? NaN);
          parents.push(node.parentElement);
        }
      }
      let previous = -1;
      for (let index = 0; index < text.length; index++) {
        const char = text.charAt(index);
        const top = tops[index] ?? NaN;
        if (/\s/.test(char) || Number.isNaN(top)) continue;
        const wraps = previous >= 0 && top > ((tops[previous] ?? 0) + (bottoms[previous] ?? 0)) / 2;
        if (wraps && index === previous + 1 && joins(text.charAt(previous), char)) {
          let start = previous;
          while (start > 0 && joins(text.charAt(start - 1), text.charAt(start))) start--;
          let end = index + 1;
          while (end < text.length && joins(text.charAt(end - 1), text.charAt(end))) end++;
          const piece = text.slice(start, end);
          const probe = document.createElement('span');
          probe.textContent = piece;
          probe.style.cssText = 'position: absolute; visibility: hidden; white-space: nowrap';
          parents[index]?.append(probe);
          const width = probe.getBoundingClientRect().width;
          probe.remove();
          // The cap on the column's text box: `max` of the scroll box, less the cell's padding and borders.
          const style = getComputedStyle(cell);
          const content = cell.clientWidth - parseFloat(style.paddingInlineStart) - parseFloat(style.paddingInlineEnd);
          const wrapper = cell.closest('.astryx-table-scroll-wrapper');
          const first = cell.previousElementSibling == null && wrapper != null;
          const cap = first ? max * wrapper.clientWidth - (cell.getBoundingClientRect().width - content) : Infinity;
          const found = `${piece} (${Math.round(width)} px, cap ${Math.round(cap)} px)`;
          const list = width > cap + 0.5 ? oversize : inside;
          if (!list.includes(found)) list.push(found);
        }
        previous = index;
      }
    }
    return {inside, oversize};
  }, max);
}

/**
 * At every width the page never scrolls sideways. At 390 px each wide table scrolls in its own box with
 * its first column fixed. At FIRST_COLUMN_WIDTHS the fixed column takes at most FIRST_COLUMN_MAX of the
 * box and copies as written, and no table cell wraps inside a name.
 */
async function checkWidthsAndTables(browser: Browser, url: string, name: string) {
  const context = await browser.newContext({viewport: {width: 320, height: 800}});
  const page = await context.newPage();
  await page.goto(url, {waitUntil: 'networkidle'});
  await page.evaluate(() => document.fonts.ready);
  for (const width of WIDTHS) {
    await page.setViewportSize({width, height: 800});
    const overflow = await pageOverflow(page);
    check(overflow <= 0, `${name} ${width}px: the page scrolls sideways by ${overflow}px`);
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
  check(wide.every(box => box.scrolls && box.inMain), `${name} 390px: a wide table does not scroll inside its own box`);
  check(wide.every(box => box.sticks), `${name} 390px: the first column of a wide table scrolls away`);
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
        const left = box.getBoundingClientRect().left;
        return [{width: end - left, ratio: (end - left) / box.clientWidth, altered}];
      }),
    );
    for (const column of columns) {
      check(column.ratio <= FIRST_COLUMN_MAX, `${name} ${width}px: a fixed first column takes ${(column.ratio * 100).toFixed(1)}% of its table's box`);
      check(column.altered === 0, `${name} ${width}px: ${column.altered} first-column cells copy as different text`);
    }
    const wraps = await wrapsInsideNames(page, FIRST_COLUMN_MAX);
    for (const piece of wraps.inside) check(false, `${name} ${width}px: a table cell wraps inside ${piece}`);
    console.log(
      `${name} ${width}px: fixed first column ${columns.map(column => `${Math.round(column.width)} px (${(column.ratio * 100).toFixed(1)}%)`).join(', ') || 'none'} of the table box; ` +
        `${wraps.inside.length} wraps inside a name, ${wraps.oversize.length} inside a piece wider than the cap${wraps.oversize.length > 0 ? `: ${wraps.oversize.join('; ')}` : ''}`,
    );
  }
  console.log(`${name}: checked ${WIDTHS.join(', ')} px; ${boxes.length} tables, ${wide.length} wider than the column at 390 px`);
  await context.close();
}

/** Whether the entry the rail marks current lies inside the rail's visible part. */
function currentInRail(page: Page): Promise<boolean> {
  return page.locator(`aside ${OUTLINE}`).evaluate(nav => {
    const link = nav.querySelector('[aria-current]');
    if (link == null) return false;
    const box = nav.getBoundingClientRect();
    const entry = link.getBoundingClientRect();
    return entry.top >= box.top - 1 && entry.bottom <= box.bottom + 1;
  });
}

/**
 * The generated 1,600-line paste. At 1280 px its outline is taller than the rail, which scrolls on its
 * own: at three points down the page, after a click on a late entry and at the end, the entry marked
 * current is the section on screen and lies inside the rail's visible part. At 390 px the drawer marks
 * the section on screen and shows its entry. Then frame times of a scroll from top to bottom at 1280
 * px, reported only: REQUIREMENTS.md sets no budget for it yet.
 */
async function checkLongPaste(browser: Browser, url: string, out: string) {
  const context = await browser.newContext({viewport: {width: 1280, height: 800}, colorScheme: 'light', deviceScaleFactor: 2});
  const page = await context.newPage();
  page.on('pageerror', error => failures.push(`long: page error: ${error.message}`));
  await page.goto(url, {waitUntil: 'networkidle'});
  await page.evaluate(() => document.fonts.ready);
  const rail = await page.locator(`aside ${OUTLINE}`).evaluate(nav => ({
    entries: nav.querySelectorAll('a').length,
    outline: nav.scrollHeight,
    visible: nav.clientHeight,
  }));
  check(rail.outline > rail.visible, `long 1280: the outline (${rail.outline} px) fits the rail (${rail.visible} px), so a long outline is not covered`);
  // Outline's track (the line beside the entries) runs the length of the list, not one rail's height.
  const track = await page.locator(`aside ${OUTLINE}`).evaluate(nav => {
    const line = nav.querySelector(':scope > [aria-hidden="true"]')?.getBoundingClientRect().height ?? 0;
    return {line, list: nav.querySelector('ul')?.scrollHeight ?? 0};
  });
  check(track.line >= track.list - 1, `long 1280: the outline's track is ${track.line} px beside a ${track.list} px list`);
  console.log(`long: ${rail.entries} outline entries, ${rail.outline} px of outline in a ${rail.visible} px rail`);
  const first = await outlineEntry(page, 0);
  await checkCurrent(page, 'long 1280 on load', first);
  for (const fraction of [0.3, 0.6, 0.9]) {
    const id = await outlineEntry(page, Math.floor((rail.entries - 1) * fraction));
    await scrollPast(page, id);
    await checkCurrent(page, `long 1280 after scrolling to #${id}`, id);
    check(await currentInRail(page), `long 1280 after scrolling to #${id}: the current entry is outside the rail's visible part`);
    if (fraction === 0.6) {
      // Outline's marker beside the current entry slides there (a short transition) and then sits level with it.
      await page.waitForTimeout(400);
      const offset = await page.locator(`aside ${OUTLINE}`).evaluate(nav => {
        const marker = nav.querySelector(':scope > span[aria-hidden="true"]')?.getBoundingClientRect().top ?? NaN;
        return Math.abs(marker - (nav.querySelector('[aria-current]')?.getBoundingClientRect().top ?? NaN));
      });
      check(offset < 1, `long 1280 after scrolling to #${id}: the outline's marker sits ${offset} px from the current entry`);
      await page.screenshot({path: join(out, '1280-light-long.png')});
    }
  }
  const late = rail.entries - 5;
  const clicked = await outlineEntry(page, late);
  await page.locator(`aside ${OUTLINE} a`).nth(late).click();
  await scrollStopped(page);
  await checkCurrent(page, `long 1280 after clicking the rail entry #${clicked}`, clicked);
  check(await currentInRail(page), `long 1280 after clicking #${clicked}: the current entry is outside the rail's visible part`);

  await page.evaluate(() => window.scrollTo(0, 0));
  await settle(page);
  const scrolling = await page.evaluate(async () => {
    const longTasks: number[] = [];
    const observer = new PerformanceObserver(list => longTasks.push(...list.getEntries().map(entry => entry.duration)));
    observer.observe({type: 'longtask'});
    const step = 100;
    const frames: number[] = [];
    await new Promise<void>(resolve => {
      let last = performance.now();
      const tick = (now: number) => {
        frames.push(now - last);
        last = now;
        if (window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 1) resolve();
        else {
          window.scrollBy(0, step);
          requestAnimationFrame(tick);
        }
      };
      requestAnimationFrame(tick);
    });
    observer.disconnect();
    frames.shift();
    const sorted = [...frames].sort((a, b) => a - b);
    const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))] ?? 0;
    return {height: document.documentElement.scrollHeight, step, frames: frames.length, p50: at(0.5), p95: at(0.95), max: at(1), longTasks: longTasks.length, longest: Math.max(0, ...longTasks)};
  });
  await settle(page);
  const last = await outlineEntry(page, rail.entries - 1);
  const marked = await currentEntries(page);
  check(marked.every(id => id === last), `long 1280 at the end: the outline marks ${JSON.stringify(marked)}, expected the last entry #${last}`);
  check(await currentInRail(page), 'long 1280 at the end: the current entry is outside the rail\'s visible part');
  console.log(
    `long 1280: scrolled ${scrolling.height} px in ${scrolling.frames} frames of ${scrolling.step} px; frame time p50 ${scrolling.p50.toFixed(1)} ms, ` +
      `p95 ${scrolling.p95.toFixed(1)} ms, max ${scrolling.max.toFixed(1)} ms; ${scrolling.longTasks} long tasks (longest ${scrolling.longest.toFixed(0)} ms)`,
  );
  await context.close();

  const phone = await browser.newContext({viewport: {width: 390, height: 844}, colorScheme: 'light', deviceScaleFactor: 2});
  const small = await phone.newPage();
  small.on('pageerror', error => failures.push(`long 390: page error: ${error.message}`));
  await small.goto(url, {waitUntil: 'networkidle'});
  await small.evaluate(() => document.fonts.ready);
  const id = await outlineEntry(small, Math.floor((rail.entries - 1) * 0.6));
  await scrollPast(small, id);
  await settle(small);
  await small.getByRole('button', {name: en['outline.open'], exact: true}).click();
  const drawer = small.getByRole('dialog');
  await drawer.waitFor();
  await small.waitForTimeout(400);
  await checkCurrent(small, `long 390 with the drawer open at #${id}`, id);
  const shown = await drawer.evaluate(dialog => {
    const link = dialog.querySelector('[aria-current]');
    if (link == null) return false;
    const box = link.getBoundingClientRect();
    return box.top >= 0 && box.bottom <= window.innerHeight;
  });
  check(shown, `long 390 with the drawer open at #${id}: the current entry is off screen in the drawer`);
  await small.screenshot({path: join(out, '390-light-long-outline.png')});
  await phone.close();
}

async function main() {
  const out = process.argv[2];
  if (out == null) {
    console.error('usage: node scripts/shoot.ts <out-dir>');
    process.exit(2);
  }
  mkdirSync(out, {recursive: true});
  const shell = readFileSync(join(DIST, 'index.html'), 'utf8');
  const server = await serve({
    '/': withEmbeddedPaste(shell, fixturePaste()),
    [LONG_PATH]: withEmbeddedPaste(shell, embeddedPaste('long', 'md', longMarkdown())),
  });
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
        const opener = page.getByRole('button', {name: en['outline.open'], exact: true});
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

    await checkWidthsAndTables(browser, url, fixturePaste().paste.id);
    const long = `${url.slice(0, -1)}${LONG_PATH}`;
    await checkWidthsAndTables(browser, long, 'long');
    await checkLongPaste(browser, long, out);
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
