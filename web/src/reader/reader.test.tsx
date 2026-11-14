import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {renderToString} from 'react-dom/server';
import {describe, expect, it} from 'vitest';
import {isMarkdown, languageName, pasteExtension} from './languages';
import {outlineItems} from './markdownPlugins';
import {MarkdownView} from './MarkdownView';
import SAMPLE from '../../fixtures/sample.md?raw';

describe('renderer choice', () => {
  it('prefers the URL extension over the stored one', () => {
    expect(pasteExtension('md', 'txt')).toBe('md');
    expect(pasteExtension(null, 'md')).toBe('md');
    expect(pasteExtension(null, null)).toBeNull();
  });

  it('names known languages and shows others as written', () => {
    expect(isMarkdown('MD')).toBe(true);
    expect(isMarkdown('rs')).toBe(false);
    expect(languageName('markdown')).toBe('Markdown');
    expect(languageName('rs')).toBe('rs');
    expect(languageName(null)).toBeNull();
  });
});

describe('Markdown paste', () => {
  const html = renderToString(<MarkdownView source={SAMPLE} />);

  it('lists the title and the sections, and every entry has its heading', () => {
    const items = outlineItems(SAMPLE);
    expect(items.map(item => [item.label, item.level])).toEqual([
      ['Release checklist', 1],
      ['Before the tag', 2],
      ['Gate summary', 3],
      ['Code samples', 2],
      ['Rust', 3],
      ['Python', 3],
      ['TOML', 3],
      ['Shell', 3],
      ['After the tag', 2],
    ]);
    for (const item of items) expect(html).toContain(`id="${item.id}"`);
  });

  it('keeps the page h1 for the title row', () => {
    expect(html).not.toMatch(/<h1[\s>]/);
    expect(html).toMatch(/<h2[^>]*>.*Release checklist/);
  });

  it('renders the table and every fenced block in full', () => {
    expect(html).toContain('<table');
    expect(html).toContain('max_body_size');
    expect(html).not.toContain('aria-expanded');
  });
});

describe('wide tables', () => {
  // MarkdownView.module.css fixes the first column and draws the edge shadow on Astryx's own scroll
  // wrapper; an Astryx upgrade that renames the wrapper would silently drop both.
  const WRAPPER = 'astryx-table-scroll-wrapper';
  const MARKDOWN_CSS = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'MarkdownView.module.css'), 'utf8');
  const html = renderToString(<MarkdownView source={'| a | b |\n| --- | --- |\n| 1 | 2 |\n'} />);

  it('renders each table inside the wrapper the stylesheet targets', () => {
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const table = doc.querySelector('table');
    expect(table).not.toBeNull();
    expect(table?.closest(`.${WRAPPER}`)).not.toBeNull();
  });

  it('lets a path in inline code wrap after each slash without adding text', () => {
    const doc = new DOMParser().parseFromString(renderToString(<MarkdownView source={'| `a/b/c/` | x |\n| --- | --- |\n| y | z |\n'} />), 'text/html');
    const code = doc.querySelector('th code');
    expect(code?.innerHTML).toBe('a/<wbr>b/<wbr>c/');
    expect(code?.textContent).toBe('a/b/c/');
  });

  it('keeps the sticky first column and the edge shadow on that wrapper', () => {
    expect(MARKDOWN_CSS).toMatch(new RegExp(`\\.${WRAPPER}\\) :is\\(th, td\\):first-child \\{[^}]*position: sticky`));
    expect(MARKDOWN_CSS).toMatch(new RegExp(`\\.${WRAPPER}\\) \\{[^}]*background:`));
  });
});
