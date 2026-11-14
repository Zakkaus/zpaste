// Screenshots and layout checks of the reading page, against the production build (`pnpm build` first).
// Serves dist/ with a fixture paste embedded the way the server's shell embeds it, then for each shot
// checks that the page never scrolls sideways, that wide tables scroll inside their own box with a
// first column of at most half of it, that no table cell wraps inside a name, that the outline marks
// the section on screen, and that the phone outline drawer takes focus and gives it back. A second,
// generated paste of 1,600 lines (longFixture.ts) checks that an outline taller than the desktop rail
// still follows the page and keeps the current entry in view, and measures scrolling through it. The
// theme checks: the Copy button paints its final colour from the first frame, the light/dark switch
// changes the theme, survives a reload without a frame of the other theme and follows the OS while no
// choice is stored. From tablet width up a table of up to three columns fits its box. Every role of
// the type scale renders the size and line height REQUIREMENTS.md records for it (typeScale.ts), at
// 390 and 1280 px, in the paste and in the committed Chinese fixtures, which have every role. The
// Traditional fixture is marked zh-Hant and drawn with a Traditional Chinese face, the Simplified one
// zh-Hans with a Simplified face, and an English paste keeps the page's language. The top bar's buttons
// share one size and icon size. On a slow link and processor with a cold cache, the top bar and the title
// row's actions paint at their final size and place from the first frame, and the page barely shifts.
//
//   node scripts/shoot.ts <out-dir>        ZPASTE_FIXTURE=<file.md> picks the paste; CHROME=<path> the browser;
//                                          ZPASTE_DIST=<dir> another build (default web/dist).
//
// Exit 1 when a check fails, 2 when it cannot run.
import {createServer, type Server} from 'node:http';
import {mkdirSync, readFileSync} from 'node:fs';
import {dirname, extname, join, normalize} from 'node:path';
import {chromium, type Browser, type Page} from '@playwright/test';
import {THEME_ATTRIBUTE, THEME_STORAGE_KEY} from '../src/chrome/themeMode.ts';
import {en} from '../src/i18n/en.ts';
import {DEFAULT_FIXTURE, embeddedPaste, fixturePaste, withEmbeddedPaste} from './fixture.ts';
import {longMarkdown} from './longFixture.ts';
import {TYPE_ROLES, readTypeScale} from './typeScale.ts';

const DIST = process.env.ZPASTE_DIST ?? new URL('../dist/', import.meta.url).pathname;
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
/**
 * Tablet, laptop and desktop widths at which a table of at most FIT_COLUMNS columns fits its box without
 * scrolling. The code face is the OS's (SF Mono, Consolas, ...), so a table that fits only just here can
 * scroll on another system; 768 px keeps a margin for that.
 */
const FIT_WIDTHS = [768, 1010, 1280];
const FIT_COLUMNS = 3;
/** Where the generated long paste is served. */
const LONG_PATH = '/long';
/**
 * The committed Chinese fixtures, one note in each script with prose, a table and commented code, each
 * served at `/<name>`: the language pasteLanguage.ts gives it and the Chinese face it must be drawn with.
 */
const CHINESE = [
  {name: 'zh-hant', file: 'chinese-hant.md', lang: 'zh-Hant', face: /\bTC\b|JhengHei/},
  {name: 'zh-hans', file: 'chinese-hans.md', lang: 'zh-Hans', face: /\bSC\b|YaHei/},
] as const;
/** Shots of the Chinese fixtures, also used for their type-scale check. */
const TYPE_SHOTS = [
  {width: 1280, height: 800, scheme: 'light'},
  {width: 390, height: 844, scheme: 'dark'},
] as const;
/** Line heights are fractions of the size (1.6471 of 17 px is 28.0007 px); REQUIREMENTS.md rounds them. */
const LEADING_TOLERANCE = 0.05;
const OUTLINE = `nav[aria-label="${en['outline.label']}"]`;
const COPY = `[role="group"][aria-label="${en['action.actions']}"] button`;
/** The paste's text, marked with its own language (ReaderPage). */
const CONTENT = 'div:has(> .astryx-markdown, > .astryx-code-block)';
/** Heights of the header shots: the top bar, the title row and the paste's first heading. */
const HEADER_HEIGHT = {wide: 256, narrow: 380};
/**
 * The first-paint check's link and processor, a remote reader's: Chrome DevTools' Fast 3G (562.5 ms
 * round trip, 1.44 Mbit/s down, 675 kbit/s up) and a CPU 4x slower, cold cache.
 */
const SLOW_LINK = {offline: false, latency: 562.5, downloadThroughput: 180_000, uploadThroughput: 84_375};
const SLOW_CPU = 4;
/** The largest cumulative layout shift the reading page may have while it loads. */
const CLS_MAX = 0.01;
/** The controls that must paint at their final size and place: the top bar's and the title row's. */
const STEADY_CONTROLS = `[role="banner"] :is(button, a), [role="group"][aria-label="${en['action.actions']}"] > *`;

/** Serves `pages[path]` for each page path (`/` and LONG_PATH) and dist/ for the rest. */
function serve(pages: Record<string, string>): Promise<Server> {
  const server = createServer((request, response) => {
    const path = normalize(new URL(request.url ?? '/', 'http://x').pathname);
    if (pages[path] == null) {
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
  for (const width of FIT_WIDTHS) {
    await page.setViewportSize({width, height: 800});
    const tables = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('.astryx-markdown .astryx-table-scroll-wrapper')].map(box => ({
        columns: box.querySelector('tr')?.children.length ?? 0,
        overflow: box.scrollWidth - box.clientWidth,
        heading: box.closest('.astryx-markdown-table')?.previousElementSibling?.textContent?.slice(0, 40) ?? '',
      })),
    );
    const narrow = tables.filter(table => table.columns <= FIT_COLUMNS);
    for (const table of narrow) {
      check(table.overflow <= 0, `${name} ${width}px: a ${table.columns}-column table after "${table.heading}" scrolls by ${table.overflow}px`);
    }
    console.log(`${name} ${width}px: ${narrow.length} tables of at most ${FIT_COLUMNS} columns, ${tables.filter(table => table.overflow > 0).length} of ${tables.length} tables scroll`);
  }
  console.log(`${name}: checked ${WIDTHS.join(', ')} px; ${boxes.length} tables, ${wide.length} wider than the column at 390 px`);
  await context.close();
}

/**
 * Records, on every animation frame from the one where the Copy button first exists, its background and
 * the root's colour scheme: what that frame paints. Installed before the page's own scripts.
 */
function recordFirstFrames() {
  const frames: string[] = [];
  Object.assign(window, {zpasteFrames: frames});
  const tick = () => {
    const button = document.querySelector('[role="group"] button');
    if (button != null) frames.push(`${getComputedStyle(button).backgroundColor}|${getComputedStyle(document.documentElement).colorScheme}`);
    if (frames.length < 30) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

/** The frames recordFirstFrames saw, once 30 have passed. */
async function firstFrames(page: Page): Promise<string[]> {
  await page.waitForFunction(() => ((window as unknown as {zpasteFrames?: string[]}).zpasteFrames?.length ?? 0) >= 30);
  return page.evaluate(() => (window as unknown as {zpasteFrames: string[]}).zpasteFrames);
}

/** The Copy button's colour in a recordFirstFrames entry. */
const copyColour = (frame: string) => frame.split('|')[0];

/** What a frame of the settled page paints, in recordFirstFrames' terms, once colour transitions have ended. */
function paintedNow(page: Page): Promise<string> {
  return page.evaluate(async selector => {
    await Promise.all(document.getAnimations().map(animation => animation.finished.catch(() => undefined)));
    return `${getComputedStyle(document.querySelector(selector)!).backgroundColor}|${getComputedStyle(document.documentElement).colorScheme}`;
  }, COPY);
}

/**
 * The colour checks. In each OS mode the Copy button paints its final colour from the first frame it
 * exists in. The light/dark switch in the top bar has an accessible name and a 44 px touch target at
 * phone width; from the keyboard it switches a light page to dark, the choice survives a reload with no
 * light frame first, and a second press goes back to following the OS, which a page then follows live.
 */
async function checkTheme(browser: Browser, url: string, out: string) {
  const painted: Record<string, string> = {};
  for (const scheme of ['light', 'dark'] as const) {
    const context = await browser.newContext({viewport: {width: 1280, height: 800}, colorScheme: scheme});
    await context.addInitScript(recordFirstFrames);
    const page = await context.newPage();
    await page.goto(url, {waitUntil: 'networkidle'});
    const frames = await firstFrames(page);
    painted[scheme] = await paintedNow(page);
    const other = frames.filter(frame => frame !== painted[scheme]);
    check(other.length === 0, `theme ${scheme}: the first frames paint ${JSON.stringify([...new Set(other)])} before ${painted[scheme]}`);
    await context.close();
  }
  const lightCopy = copyColour(painted.light ?? '');
  const darkCopy = copyColour(painted.dark ?? '');
  check(lightCopy !== darkCopy, `theme: light and dark paint the same Copy button (${lightCopy})`);

  const context = await browser.newContext({viewport: {width: 390, height: 844}, colorScheme: 'light', deviceScaleFactor: 2});
  const page = await context.newPage();
  page.on('pageerror', error => failures.push(`theme: page error: ${error.message}`));
  await page.goto(url, {waitUntil: 'networkidle'});
  const toDark = page.getByRole('button', {name: en['theme.toDark'], exact: true});
  const size = await toDark.boundingBox();
  check(size != null && size.width >= 44 && size.height >= 44, `theme 390px: the switch is ${size?.width}x${size?.height} px, under 44 px`);
  check((await page.getByRole('link', {name: en['action.edit'], exact: true}).count()) === 1, 'theme 390px: no Edit action for a paste the reader may edit');
  await toDark.focus();
  await page.keyboard.press('Enter');
  check(copyColour(await paintedNow(page)) === darkCopy, `theme: after the switch the Copy button is ${await paintedNow(page)}, not dark ${darkCopy}`);
  await page.screenshot({path: join(out, '390-light-switched-dark.png')});

  await page.addInitScript(recordFirstFrames);
  await page.reload({waitUntil: 'networkidle'});
  const frames = await firstFrames(page);
  const settled = await paintedNow(page);
  check(copyColour(settled) === darkCopy, `theme: after a reload the page switched to dark paints ${settled}`);
  const light = frames.filter(frame => frame !== settled);
  check(light.length === 0, `theme: reloading a page switched to dark paints ${JSON.stringify([...new Set(light)])} first`);
  const toLight = page.getByRole('button', {name: en['theme.toLight'], exact: true});
  if ((await toLight.count()) !== 1) {
    check(false, 'theme: after the reload the switch does not offer light mode');
    await context.close();
    return;
  }
  await toLight.click();
  const stored = await page.evaluate(
    ([attribute, key]) => [document.documentElement.getAttribute(attribute), localStorage.getItem(key)],
    [THEME_ATTRIBUTE, THEME_STORAGE_KEY] as const,
  );
  check(stored.every(value => value == null), `theme: switching back to the OS mode left ${JSON.stringify(stored)}`);
  check(copyColour(await paintedNow(page)) === lightCopy, 'theme: switching back to the OS mode does not paint light');
  await page.emulateMedia({colorScheme: 'dark'});
  check(copyColour(await paintedNow(page)) === darkCopy, 'theme: following the OS, the page does not follow a switch to dark');
  check((await page.getByRole('button', {name: en['theme.toLight'], exact: true}).count()) === 1, 'theme: following the OS into dark, the switch does not offer light mode');
  console.log(`theme: Copy paints ${lightCopy} in light and ${darkCopy} in dark from the first frame; switch ${size?.width}x${size?.height} px at 390 px`);
  await context.close();
}

/**
 * The title row's actions are one set: every button has the same height and top edge within its row
 * (rows may wrap on a phone), and Copy keeps its width when its label turns into its feedback.
 */
async function checkActions(page: Page, name: string, wide: boolean) {
  const group = page.locator(`[role="group"][aria-label="${en['action.actions']}"]`);
  const boxes = await group.locator(':scope > *').evaluateAll(elements =>
    elements.map(element => {
      const box = element.getBoundingClientRect();
      return {text: element.textContent ?? '', top: Math.round(box.top), height: Math.round(box.height), width: box.width};
    }),
  );
  const heights = new Set(boxes.map(box => box.height));
  check(heights.size === 1, `${name}: the actions have heights ${JSON.stringify(boxes.map(box => [box.text, box.height]))}`);
  const rows = new Map<number, number>();
  for (const box of boxes) rows.set(box.top, (rows.get(box.top) ?? 0) + 1);
  // One row on a wide screen; on a phone the row may wrap once, each line level.
  check(rows.size <= (wide ? 1 : 2), `${name}: the actions sit at tops ${JSON.stringify(boxes.map(box => [box.text, box.top]))}`);
  const copy = group.locator('button').first();
  const before = await copy.evaluate(button => button.getBoundingClientRect().width);
  await copy.click();
  await page.waitForFunction(([selector, label]) => document.querySelector(selector)?.getAttribute('aria-label') !== label, [COPY, en['action.copy']] as const);
  const after = await copy.evaluate(button => [button.getBoundingClientRect().width, button.getAttribute('aria-label')] as const);
  check(Math.abs(after[0] - before) < 0.5, `${name}: Copy is ${before} px wide and ${after[0]} px as "${after[1]}"`);
  console.log(`${name}: ${boxes.length} actions ${[...heights].join('/')} px high in ${rows.size} row(s); Copy ${Math.round(before)} px as "Copy" and as "${after[1]}"`);
}

/**
 * The top bar's controls are one set, as on gentoozh.org: icon buttons of one width, height and icon
 * size, and at phone width at least the theme's 44 px touch height.
 */
async function checkTopBar(page: Page, name: string, wide: boolean) {
  const buttons = await page.locator('[role="banner"] button').evaluateAll(elements =>
    elements
      .filter(element => element.checkVisibility())
      .map(element => {
        const box = element.getBoundingClientRect();
        const icon = element.querySelector('svg')?.getBoundingClientRect();
        return {
          name: element.getAttribute('aria-label') ?? element.textContent ?? '',
          size: `${Math.round(box.width)}x${Math.round(box.height)}`,
          height: Math.round(box.height),
          icon: icon == null ? 'none' : `${Math.round(icon.width)}x${Math.round(icon.height)}`,
          text: element.textContent?.trim() ?? '',
        };
      }),
  );
  const describe = JSON.stringify(buttons.map(button => [button.name, button.size, button.icon]));
  check(buttons.length >= (wide ? 1 : 2), `${name}: the top bar shows ${buttons.length} buttons`);
  check(new Set(buttons.map(button => button.size)).size === 1, `${name}: the top-bar buttons differ in size ${describe}`);
  check(new Set(buttons.map(button => button.icon)).size === 1, `${name}: the top-bar icons differ in size ${describe}`);
  check(buttons.every(button => button.text === ''), `${name}: a top-bar button shows a text label ${describe}`);
  if (!wide) check(buttons.every(button => button.height >= 44), `${name}: a top-bar button is under 44 px high ${describe}`);
  console.log(`${name}: top bar ${describe}`);
}

/**
 * Records, from the page's first script, every layout shift with the controls of STEADY_CONTROLS it
 * moved, and on every animation frame where those controls exist their boxes. Installed before the
 * page's own scripts.
 */
function recordLoading(selector: string) {
  const shifts: {value: number; controls: string[]}[] = [];
  const layouts: string[] = [];
  const name = (element: Element) => element.getAttribute('aria-label') ?? element.textContent?.trim() ?? element.tagName;
  new PerformanceObserver(list => {
    for (const entry of list.getEntries() as (PerformanceEntry & {value: number; hadRecentInput: boolean; sources?: {node?: Node}[]})[]) {
      if (entry.hadRecentInput) continue;
      const controls = [...document.querySelectorAll(selector)];
      const moved = (entry.sources ?? []).flatMap(source => {
        const node = source.node;
        if (node == null) return [];
        return controls.filter(control => control.contains(node) || node.contains(control)).map(name);
      });
      shifts.push({value: entry.value, controls: moved});
    }
  }).observe({type: 'layout-shift', buffered: true});
  const tick = () => {
    const controls = [...document.querySelectorAll(selector)].filter(element => element.checkVisibility());
    if (controls.length > 0) {
      const layout = controls
        .map(element => {
          const box = element.getBoundingClientRect();
          return `${name(element)} ${[box.x, box.y, box.width, box.height].map(value => Math.round(value * 10) / 10).join(',')}`;
        })
        .join(' | ');
      if (layouts.at(-1) !== layout) layouts.push(layout);
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  Object.assign(window, {zpasteLoading: {shifts, layouts}});
}

/**
 * The page's first paint is its final layout. On a slow link and processor with a cold cache (SLOW_LINK,
 * SLOW_CPU), at 390 and 1280 px: the top bar's buttons and the title row's actions keep the size and
 * place of the first frame they appear in, through the web font's arrival, no layout shift moves them,
 * and the page's cumulative layout shift stays under CLS_MAX.
 */
async function checkFirstPaint(browser: Browser, url: string) {
  for (const width of [390, 1280]) {
    const name = `first paint ${width}px`;
    const context = await browser.newContext({viewport: {width, height: width < 1024 ? 844 : 800}, deviceScaleFactor: 2});
    await context.addInitScript(recordLoading, STEADY_CONTROLS);
    const page = await context.newPage();
    page.on('pageerror', error => failures.push(`${name}: page error: ${error.message}`));
    const cdp = await context.newCDPSession(page);
    await cdp.send('Network.enable');
    await cdp.send('Network.setCacheDisabled', {cacheDisabled: true});
    await cdp.send('Network.emulateNetworkConditions', SLOW_LINK);
    await cdp.send('Emulation.setCPUThrottlingRate', {rate: SLOW_CPU});
    await page.goto(url, {waitUntil: 'networkidle', timeout: 120_000});
    await page.evaluate(() => document.fonts.ready);
    // Frames after the fonts settle, so a late reflow would be recorded.
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const {shifts, layouts} = await page.evaluate(() => (window as unknown as {zpasteLoading: {shifts: {value: number; controls: string[]}[]; layouts: string[]}}).zpasteLoading);
    const cls = shifts.reduce((sum, shift) => sum + shift.value, 0);
    const moved = [...new Set(shifts.flatMap(shift => shift.controls))];
    check(layouts.length === 1, `${name}: the controls change layout while loading ${JSON.stringify(layouts)}`);
    check(moved.length === 0, `${name}: a layout shift moves ${JSON.stringify(moved)}`);
    check(cls < CLS_MAX, `${name}: cumulative layout shift ${cls.toFixed(4)}, at least ${CLS_MAX}`);
    console.log(`${name}: CLS ${cls.toFixed(4)} in ${shifts.length} shift(s); ${layouts.length} layout(s) of the controls`);
    await context.close();
  }
}

/**
 * A Chinese fixture is marked with its script's language on the paste's text only, and Chrome draws its
 * Chinese with a face of that script (CSS.getPlatformFontsForNode on the first paragraph). Returns the
 * face, so the caller can check that the two scripts get different faces.
 */
async function checkChineseFace(browser: Browser, url: string, fixture: (typeof CHINESE)[number]): Promise<string> {
  const context = await browser.newContext({viewport: {width: 1280, height: 800}, deviceScaleFactor: 1});
  const page = await context.newPage();
  await page.goto(url, {waitUntil: 'networkidle'});
  await page.evaluate(() => document.fonts.ready);
  const langs = await page.evaluate(selector => ({page: document.documentElement.lang, content: document.querySelector(selector)?.getAttribute('lang') ?? null, marked: document.querySelectorAll('[lang]').length}), CONTENT);
  check(langs.content === fixture.lang, `${fixture.name}: the paste's text is marked ${langs.content}, expected ${fixture.lang}`);
  // Only <html> and the paste's text carry a language: the chrome keeps the page's.
  check(langs.marked === 2, `${fixture.name}: ${langs.marked} elements carry lang, expected <html> and the paste's text`);
  const cdp = await context.newCDPSession(page);
  await cdp.send('DOM.enable');
  await cdp.send('CSS.enable');
  const {root} = await cdp.send('DOM.getDocument', {depth: -1});
  const {nodeId} = await cdp.send('DOM.querySelector', {nodeId: root.nodeId, selector: `${CONTENT} .astryx-markdown-paragraph`});
  const {fonts} = await cdp.send('CSS.getPlatformFontsForNode', {nodeId});
  // The face that draws most glyphs other than the Latin one.
  const chinese = fonts.filter(font => !/figtree/i.test(font.familyName)).sort((a, b) => b.glyphCount - a.glyphCount)[0];
  const face = chinese == null ? 'none' : `${chinese.familyName} (${chinese.postScriptName})`;
  check(chinese != null && fixture.face.test(chinese.familyName), `${fixture.name}: Chinese is drawn with ${face}, expected a face matching ${fixture.face}`);
  console.log(`${fixture.name}: lang ${langs.content}; faces ${fonts.map(font => `${font.familyName} ${font.glyphCount}`).join(', ')}`);
  await context.close();
  return face;
}

/** An English paste carries no language of its own: it keeps the page's. */
async function checkNoLanguage(browser: Browser, url: string, name: string) {
  const context = await browser.newContext({viewport: {width: 1280, height: 800}});
  const page = await context.newPage();
  await page.goto(url, {waitUntil: 'networkidle'});
  const lang = await page.evaluate(selector => document.querySelector(selector)?.getAttribute('lang') ?? null, CONTENT);
  check(lang == null, `${name}: the paste's text is marked ${lang}, expected the page's language`);
  await context.close();
}

/**
 * Each role of the type scale renders the size and line height REQUIREMENTS.md records, at 1280 px and
 * 390 px. A role measures its first visible element; on a phone the outline is measured in the open
 * drawer, and the rail's label does not show. `complete` (the Chinese fixture) requires every other
 * role; another paste skips the roles it lacks. Shots of the page go to `out` when it is given.
 */
async function checkTypeScale(browser: Browser, url: string, name: string, complete: boolean, out?: string) {
  const scale = readTypeScale();
  for (const shot of TYPE_SHOTS) {
    const label = `${name} ${shot.width}`;
    const context = await browser.newContext({viewport: {width: shot.width, height: shot.height}, colorScheme: shot.scheme, deviceScaleFactor: 2});
    const page = await context.newPage();
    page.on('pageerror', error => failures.push(`${label}: page error: ${error.message}`));
    await page.goto(url, {waitUntil: 'networkidle'});
    await page.evaluate(() => document.fonts.ready);
    if (out != null) await page.screenshot({path: join(out, `${name}-${shot.width}-${shot.scheme}.png`)});
    const wide = shot.width >= 1024;
    if (!wide) {
      await page.getByRole('button', {name: en['outline.open'], exact: true}).click();
      await page.getByRole('dialog').waitFor();
    }
    const rendered = await page.evaluate(roles => {
      const found: Record<string, {size: number; leading: number} | null> = {};
      for (const [role, selector] of Object.entries(roles)) {
        const element = [...document.querySelectorAll(selector)].find(candidate => candidate.checkVisibility());
        const style = element == null ? null : getComputedStyle(element);
        found[role] = style == null ? null : {size: parseFloat(style.fontSize), leading: parseFloat(style.lineHeight)};
      }
      return found;
    }, TYPE_ROLES);
    const report: string[] = [];
    for (const [role, expected] of scale) {
      const actual = rendered[role];
      if (actual == null) {
        check(!complete || (role === 'outline label' && !wide), `${label}: no visible element shows the type role "${role}" (${TYPE_ROLES[role]})`);
        continue;
      }
      const same = Math.abs(actual.size - expected.size) < 0.01 && Math.abs(actual.leading - expected.leading) <= LEADING_TOLERANCE;
      check(same, `${label}: type role "${role}" renders ${actual.size} / ${actual.leading} px, REQUIREMENTS.md records ${expected.size} / ${expected.leading}`);
      report.push(`${role} ${actual.size}/${Math.round(actual.leading * 100) / 100}`);
    }
    console.log(`${label}: type scale ${report.join('; ')}`);
    await context.close();
  }
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
    ...Object.fromEntries(CHINESE.map(fixture => [`/${fixture.name}`, withEmbeddedPaste(shell, fixturePaste(join(dirname(DEFAULT_FIXTURE), fixture.file)))])),
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
      const wide = shot.width >= 1024;
      await page.screenshot({path: join(out, `${shot.name}-header.png`), clip: {x: 0, y: 0, width: shot.width, height: wide ? HEADER_HEIGHT.wide : HEADER_HEIGHT.narrow}});
      await checkTopBar(page, shot.name, wide);

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
      if (wide) {
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
      await checkActions(page, shot.name, wide);
      await context.close();
    }

    await checkTypeScale(browser, url, fixturePaste().paste.id, false);
    const faces: string[] = [];
    for (const fixture of CHINESE) {
      await checkTypeScale(browser, `${url}${fixture.name}`, fixture.name, true, out);
      faces.push(await checkChineseFace(browser, `${url}${fixture.name}`, fixture));
    }
    check(new Set(faces).size === faces.length, `the Chinese fixtures share the face ${faces.join(' / ')}`);
    if (process.env.ZPASTE_FIXTURE == null) await checkNoLanguage(browser, url, fixturePaste().paste.id);
    await checkTheme(browser, url, out);
    await checkFirstPaint(browser, url);
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
