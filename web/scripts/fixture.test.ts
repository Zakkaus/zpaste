import {mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {afterAll, describe, expect, it} from 'vitest';
import {readEmbeddedPaste, type PasteMeta} from '../src/shell/embedded.ts';
import {DEFAULT_FIXTURE, fixturePaste, withEmbeddedPaste} from './fixture.ts';
import {longMarkdown} from './longFixture.ts';

/** Every field of `GET /v1/pastes/{id}` the shell embeds, so a field dropped from the fixture fails here. */
const META_KEYS: Array<keyof PasteMeta> = [
  'id',
  'extension',
  'title',
  'encrypted',
  'burn_after_reading',
  'created_at',
  'expires_at',
  'version',
  'version_id',
];

describe('fixturePaste', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zpaste-fixture-'));
  afterAll(() => rmSync(dir, {recursive: true, force: true}));

  it('describes the committed sample as an ordinary Markdown paste', () => {
    const data = fixturePaste(DEFAULT_FIXTURE);
    expect(Object.keys(data.paste).sort()).toEqual([...META_KEYS].sort());
    expect(data.paste).toMatchObject({id: 'sample', extension: 'md', encrypted: false, burn_after_reading: false});
    expect(data.url_extension).toBe('md');
    expect(data.content).toMatch(/^# /);
    expect(data.etag).toMatch(/^"[0-9a-f]{64}"$/);
  });

  it('takes the id and extension from another file name', () => {
    const path = join(dir, 'r8BaRKfkXfi.rs');
    writeFileSync(path, 'fn main() {}\n');
    const data = fixturePaste(path);
    expect(data.paste.id).toBe('r8BaRKfkXfi');
    expect(data.paste.extension).toBe('rs');
    expect(data.content).toBe('fn main() {}\n');
  });

  it('gives a file without an extension no extension', () => {
    const path = join(dir, 'abc123');
    writeFileSync(path, 'text');
    expect(fixturePaste(path).paste.extension).toBeNull();
    expect(fixturePaste(path).url_extension).toBeNull();
  });
});

describe('withEmbeddedPaste', () => {
  it('puts the paste before the root, where the app reads it back unchanged', () => {
    const data = fixturePaste(DEFAULT_FIXTURE);
    const html = withEmbeddedPaste('<body><div id="root"></div></body>', data);
    expect(html.indexOf('zpaste-paste')).toBeLessThan(html.indexOf('id="root"'));
    document.documentElement.innerHTML = html;
    expect(readEmbeddedPaste(document)).toEqual({state: 'found', data});
  });
});

describe('longMarkdown', () => {
  it('holds at least 1,600 lines and 60 headings, more than the rail shows', () => {
    const source = longMarkdown();
    expect(source.split('\n').length).toBeGreaterThanOrEqual(1600);
    const headings = source.split('\n').filter(line => /^#{1,4} /.test(line));
    expect(headings.length).toBeGreaterThanOrEqual(60);
    expect(new Set(headings).size).toBe(headings.length);
  });
});
