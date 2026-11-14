import {renderToString} from 'react-dom/server';
import {describe, expect, it} from 'vitest';
import type {EmbeddedPaste} from '../shell/embedded';
import {PasteHeader} from './PasteHeader';

const data = (overrides: Partial<EmbeddedPaste['paste']> = {}): EmbeddedPaste => ({
  paste: {
    id: 'abc123',
    extension: 'md',
    title: null,
    encrypted: false,
    burn_after_reading: false,
    created_at: '2026-11-14T08:00:00Z',
    expires_at: null,
    version: 1,
    version_id: 'f'.repeat(64),
    ...overrides,
  },
  content: '# Title',
  etag: `"${'f'.repeat(64)}"`,
  url_extension: null,
});

const hrefs = (html: string) => [...html.matchAll(/href="([^"]+)"/g)].map(match => match[1]);

describe('PasteHeader actions', () => {
  it('links Raw to /raw/<id> and Download to /dl/<id>.<ext>', () => {
    const html = renderToString(<PasteHeader data={data()} extension="md" />);
    expect(hrefs(html)).toEqual(['/raw/abc123', '/dl/abc123.md']);
  });

  it('downloads under the extension that picked the renderer, or the bare id without one', () => {
    expect(hrefs(renderToString(<PasteHeader data={data()} extension="rs" />))).toContain('/dl/abc123.rs');
    expect(hrefs(renderToString(<PasteHeader data={data({extension: null})} extension={null} />))).toEqual([
      '/raw/abc123',
      '/dl/abc123',
    ]);
  });

  it('groups the actions under one label', () => {
    expect(renderToString(<PasteHeader data={data()} extension="md" />)).toMatch(/role="group" aria-label="Paste actions"/);
  });
});

describe('PasteHeader copy action', () => {
  it('keeps every visible state in the button, so its width does not change, and names the action', () => {
    const html = renderToString(<PasteHeader data={data()} extension="md" />);
    expect(html).toMatch(/<button[^>]*aria-label="Copy"/);
    for (const label of ['>Copy<', '>Copied<', '>Failed<']) expect(html).toContain(label);
  });
});

describe('PasteHeader edit action', () => {
  it('hides Edit when the shell does not say the reader may edit', () => {
    expect(hrefs(renderToString(<PasteHeader data={{...data(), can_edit: false}} extension="md" />))).not.toContain('/abc123/edit');
    expect(renderToString(<PasteHeader data={data()} extension="md" />)).not.toContain('>Edit<');
  });

  it('links Edit to /<id>/edit, after Copy and before Raw and Download', () => {
    const html = renderToString(<PasteHeader data={{...data(), can_edit: true}} extension="md" />);
    expect(hrefs(html)).toEqual(['/abc123/edit', '/raw/abc123', '/dl/abc123.md']);
    expect(html.indexOf('Copy')).toBeLessThan(html.indexOf('Edit'));
  });
});

describe('PasteHeader title and details', () => {
  it('names an untitled paste after its id and draws the page h1', () => {
    const html = renderToString(<PasteHeader data={data()} extension="md" />);
    expect(html.match(/<h1[\s>]/g)).toHaveLength(1);
    expect(html).toContain('Paste abc123');
  });

  it('shows a stored title as written', () => {
    expect(renderToString(<PasteHeader data={data({title: 'Release notes'})} extension="md" />)).toContain('Release notes');
  });

  it('says a paste without an expiry never expires and leaves out a missing creation date', () => {
    const html = renderToString(<PasteHeader data={data({created_at: null})} extension={null} />);
    expect(html).toContain('Never expires');
    expect(html).not.toContain('Created');
    expect(html).toContain('Plain text');
  });
});
