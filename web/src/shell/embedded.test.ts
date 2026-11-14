import {describe, expect, it} from 'vitest';
import {EMBEDDED_PASTE_ID, embeddedPasteElement, readEmbeddedPaste, serializeEmbeddedPaste, type EmbeddedPaste} from './embedded';

const paste = (content: string): EmbeddedPaste => ({
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
  },
  content,
  etag: `"${'f'.repeat(64)}"`,
  url_extension: null,
});

/** The text of the element the browser would close at the first `</script>`, as an HTML parser reads it. */
function scriptText(element: string): string {
  const open = element.indexOf('>') + 1;
  return element.slice(open, element.indexOf('</script', open));
}

describe('serializeEmbeddedPaste', () => {
  it('keeps content that holds a closing script tag inside the element', () => {
    const hostile = 'a</script><script>alert(1)</script><!-- & \u2028\u2029 end';
    const element = embeddedPasteElement(paste(hostile));
    const text = scriptText(element);
    expect(text).not.toMatch(/[<>&\u2028\u2029]/);
    expect((JSON.parse(text) as EmbeddedPaste).content).toBe(hostile);
  });

  it('round-trips through the page', () => {
    const data = paste('# Title\n\nbody');
    document.body.innerHTML = embeddedPasteElement(data);
    expect(readEmbeddedPaste(document)).toEqual({state: 'found', data});
    expect(document.getElementById(EMBEDDED_PASTE_ID)?.getAttribute('type')).toBe('application/json');
  });

  it('reads nothing from a page without a paste', () => {
    document.body.innerHTML = '';
    expect(readEmbeddedPaste(document)).toEqual({state: 'missing'});
  });

  it('tells a cut-off or foreign element from a missing one', () => {
    const element = embeddedPasteElement(paste('# Title'));
    document.body.innerHTML = `${element.slice(0, element.indexOf('"content"'))}</script>`;
    expect(readEmbeddedPaste(document)).toEqual({state: 'malformed'});
    document.body.innerHTML = `<script type="application/json" id="${EMBEDDED_PASTE_ID}">{"content": 1}</script>`;
    expect(readEmbeddedPaste(document)).toEqual({state: 'malformed'});
  });

  it('writes plain JSON for ordinary text', () => {
    expect(JSON.parse(serializeEmbeddedPaste(paste('x')))).toEqual(paste('x'));
  });
});
