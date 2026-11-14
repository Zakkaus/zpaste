// The contract between the server's HTML shell and the app: for an ordinary paste the shell embeds
// the paste in the page, so the app renders it without a second request (docs/architecture.md,
// "Serving a page"). The Rust shell must write exactly this shape and escaping; the dev server and
// the screenshot script write it with `serializeEmbeddedPaste` below.

/** The id of the `<script type="application/json">` element that carries the paste. */
export const EMBEDDED_PASTE_ID = 'zpaste-paste';

/** The paste resource, with the field names of `GET /v1/pastes/{id}`. */
export interface PasteMeta {
  /** 6 or 11 characters, without the extension. */
  id: string;
  /** The extension stored at creation, such as `md` or `rs`; null for none. */
  extension: string | null;
  title: string | null;
  encrypted: boolean;
  burn_after_reading: boolean;
  /** RFC 3339, UTC; null for a paste created before the column existed. */
  created_at: string | null;
  /** RFC 3339, UTC; null for a paste that never expires. */
  expires_at: string | null;
  version: number;
  version_id: string;
}

/** What the shell embeds for an ordinary paste. Encrypted and burn-after-reading pastes are never embedded. */
export interface EmbeddedPaste {
  paste: PasteMeta;
  /** The paste text, UTF-8. */
  content: string;
  /** The `ETag` the content was served with. */
  etag: string;
  /** The extension in the reading URL (`/<id>.<ext>`), which picks the renderer before the stored one; null for none. */
  url_extension: string | null;
  /**
   * Whether the reader may edit this paste, which shows the Edit action. The shell sets it from the
   * request's owner credentials (cookie or Bearer, docs/architecture.md "Serving a page") from P1
   * slice 4; until then it is absent, which means false. The server still checks every write.
   */
  can_edit?: boolean;
}

/**
 * JSON safe to place between `<script type="application/json">` and `</script>`: `<`, `>` and `&` become
 * `\u003c`, `\u003e` and `\u0026`, so content holding `</script>` or `<!--` cannot end or alter the
 * element, and U+2028 and U+2029 are escaped as well. JSON.parse reads the result back unchanged.
 */
export function serializeEmbeddedPaste(data: EmbeddedPaste): string {
  return JSON.stringify(data).replace(/[<>&\u2028\u2029]/g, char => `\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`);
}

/** The element the shell writes, ready to insert into the page. */
export function embeddedPasteElement(data: EmbeddedPaste): string {
  return `<script type="application/json" id="${EMBEDDED_PASTE_ID}">${serializeEmbeddedPaste(data)}</script>`;
}

/**
 * What the page carries: the paste, nothing (`missing`), or an element that is not the paste's JSON
 * (`malformed`), such as a shell cut off mid-element or written with other escaping.
 */
export type EmbeddedRead = {state: 'found'; data: EmbeddedPaste} | {state: 'missing'} | {state: 'malformed'};

export function readEmbeddedPaste(doc: Document): EmbeddedRead {
  const text = doc.getElementById(EMBEDDED_PASTE_ID)?.textContent;
  if (text == null || text === '') return {state: 'missing'};
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return {state: 'malformed'};
  }
  return isEmbeddedPaste(data) ? {state: 'found', data} : {state: 'malformed'};
}

/** The fields the reading page cannot do without. */
function isEmbeddedPaste(value: unknown): value is EmbeddedPaste {
  if (typeof value !== 'object' || value == null) return false;
  const {paste, content} = value as Partial<EmbeddedPaste>;
  return typeof content === 'string' && typeof paste === 'object' && paste != null && typeof paste.id === 'string';
}
