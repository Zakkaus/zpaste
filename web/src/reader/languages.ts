// Display names of paste languages, keyed by extension. Names are proper nouns and are not translated.
const NAMES: Readonly<Record<string, string>> = {
  md: 'Markdown',
  markdown: 'Markdown',
};

/** The extension that picks the renderer: the URL's first, then the stored one (docs/design-language.md, "Renderers"). */
export function pasteExtension(urlExtension: string | null, storedExtension: string | null): string | null {
  return urlExtension ?? storedExtension;
}

/** Extensions the Markdown renderer draws. */
const MARKDOWN = new Set(['md', 'markdown']);

export function isMarkdown(extension: string | null): boolean {
  return extension != null && MARKDOWN.has(extension.toLowerCase());
}

/** The name shown in the metadata row: a known name, the extension as written, or null for plain text. */
export function languageName(extension: string | null): string | null {
  return extension == null ? null : (NAMES[extension.toLowerCase()] ?? extension);
}
