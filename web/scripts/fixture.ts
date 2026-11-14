// A paste for the dev server and the screenshot script, embedded the way the server's shell embeds it.
import {readFileSync} from 'node:fs';
import {basename, dirname, extname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {embeddedPasteElement, type EmbeddedPaste} from '../src/shell/embedded.ts';

/** The committed synthetic paste; ZPASTE_FIXTURE names another Markdown file, such as a real paste kept outside the repository. */
// Built from the module's path rather than `new URL(..., import.meta.url)`, which Vite (and so Vitest) rewrites as an asset URL.
export const DEFAULT_FIXTURE = join(dirname(fileURLToPath(import.meta.url)), '..', 'fixtures', 'sample.md');

export function fixturePaste(path: string = process.env.ZPASTE_FIXTURE ?? DEFAULT_FIXTURE): EmbeddedPaste {
  const extension = extname(path).slice(1) || null;
  const id = basename(path, extname(path));
  return {
    paste: {
      id,
      extension,
      title: null,
      encrypted: false,
      burn_after_reading: false,
      created_at: '2026-11-14T08:00:00Z',
      expires_at: '2026-12-12T08:00:00Z',
      version: 1,
      version_id: '0'.repeat(64),
    },
    content: readFileSync(path, 'utf8'),
    etag: `"${'0'.repeat(64)}"`,
    url_extension: extension,
  };
}

/** `html` with the paste element inserted before the app's root, as the shell will write it. */
export function withEmbeddedPaste(html: string, data: EmbeddedPaste): string {
  return html.replace('<div id="root"></div>', `${embeddedPasteElement(data)}<div id="root"></div>`);
}
