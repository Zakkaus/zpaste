// How much of a paste the page renders, and with which renderer. The whole paste is parsed and laid
// out on the main thread in one task, so its size bounds the time the page is frozen. Measured on the
// production build with a CPU 4x slower (REQUIREMENTS.md, "Paste size the reading page renders"):
// Markdown's longest task grows about 10 ms per KB of code-heavy text (1.0 s at 64 KB, 1.6 s at 128 KB,
// 8 s at 1 MB); the plain-text renderer's about 0.5 s per MB.

/** The most characters drawn as Markdown, with an outline. */
export const MARKDOWN_LIMIT = 100_000;

/** The most characters drawn as plain text; past it, only that many from the start are shown. */
export const TEXT_LIMIT = 1_000_000;

export type RenderPlan =
  | {renderer: 'markdown'}
  /** `notice` says why a paste is not drawn as it would be: `plain` (Markdown too large), `truncated` (only the start is shown). */
  | {renderer: 'text'; text: string; notice: 'plain' | 'truncated' | null};

export function renderPlan(content: string, markdown: boolean): RenderPlan {
  if (content.length > TEXT_LIMIT) return {renderer: 'text', text: textStart(content, TEXT_LIMIT), notice: 'truncated'};
  if (!markdown) return {renderer: 'text', text: content, notice: null};
  if (content.length > MARKDOWN_LIMIT) return {renderer: 'text', text: content, notice: 'plain'};
  return {renderer: 'markdown'};
}

/** At most `limit` characters from the start of `text`, cut after a line when one ends in the second half, and never inside a surrogate pair. */
function textStart(text: string, limit: number): string {
  const line = text.lastIndexOf('\n', limit - 1);
  if (line >= limit / 2) return text.slice(0, line + 1);
  const code = text.charCodeAt(limit - 1);
  return text.slice(0, code >= 0xd800 && code <= 0xdbff ? limit - 1 : limit);
}
