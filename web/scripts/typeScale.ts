// The reading page's type scale as REQUIREMENTS.md records it ("Numbers: Type scale"), and the element
// that shows each role. shoot.ts checks the rendered page against the row; typeScale.test.ts checks
// that the row and this list name the same roles, so neither can change alone.
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {en} from '../src/i18n/en.ts';

export const REQUIREMENTS = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'REQUIREMENTS.md');

const ROW = '| Type scale (role: size / line height) |';

/** Each role and the first visible element that shows it. A role a paste lacks (no `####`) is skipped. */
export const TYPE_ROLES: Record<string, string> = {
  'top-bar name': '[role="banner"] .astryx-top-nav .astryx-text',
  'page title': 'h1',
  // The paste's `#` renders as h2, because the title row holds the h1 (MarkdownView).
  'paste #': '.astryx-markdown h2',
  'paste ##': '.astryx-markdown h3',
  'paste ###': '.astryx-markdown h4',
  'paste ####': '.astryx-markdown h5',
  body: '.astryx-markdown-paragraph',
  details: 'h1 + [data-type="supporting"]',
  'table cell': '.astryx-markdown td',
  'table header': '.astryx-markdown th',
  'outline label': 'aside > h2',
  'outline entry': `nav[aria-label="${en['outline.label']}"] a`,
  button: `[role="group"][aria-label="${en['action.actions']}"] button`,
  'inline code': '.astryx-markdown-paragraph code',
  'code block': '.astryx-markdown .astryx-code-block code',
};

export interface TypeRole {
  /** Font size in px. */
  size: number;
  /** Line height in px. */
  leading: number;
}

/** The roles of the Type scale row in `markdown` (REQUIREMENTS.md), in the row's order. */
export function documentedTypeScale(markdown: string): Map<string, TypeRole> {
  const line = markdown.split('\n').find(row => row.startsWith(ROW));
  if (line == null) throw new Error(`REQUIREMENTS.md has no row starting with "${ROW}"`);
  const limit = line.slice(ROW.length).split(/(?<!\\)\|/)[0]?.trim() ?? '';
  const scale = new Map<string, TypeRole>();
  for (const entry of limit.split('; ')) {
    const match = /^(.+): (\d+(?:\.\d+)?) \/ (\d+(?:\.\d+)?)$/.exec(entry);
    if (match == null) throw new Error(`Type scale entry "${entry}" is not "role: size / line height"`);
    const [, role = '', size = '', leading = ''] = match;
    scale.set(role, {size: Number(size), leading: Number(leading)});
  }
  return scale;
}

export function readTypeScale(): Map<string, TypeRole> {
  return documentedTypeScale(readFileSync(REQUIREMENTS, 'utf8'));
}
