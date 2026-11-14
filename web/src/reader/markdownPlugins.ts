import {createMarkdownHeadingLinks} from '@astryxdesign/core/Markdown/plugins';
import {parseOutlineFromMarkdown} from '@astryxdesign/core/Outline';
import type {OutlineItem} from '@astryxdesign/core/Outline';

/** One plugin list for the rendered document and its outline, so every outline entry lands on its heading. */
export const MARKDOWN_PLUGINS = [createMarkdownHeadingLinks()];

/**
 * Headings the outline lists: the document's title (`#`, drawn as a section heading because the page
 * title row holds the h1), its sections and their subsections. The title entry is the section a reader
 * sees on first load; without it Outline's scroll tracking marked the first `##` current while the
 * title was on screen. Outline indents `#` and `##` alike, so the title adds a row, not a level.
 */
const OUTLINE_LEVELS = {min: 1, max: 4};

export function outlineItems(markdown: string): OutlineItem[] {
  return parseOutlineFromMarkdown(markdown, {plugins: MARKDOWN_PLUGINS}).filter(
    item => item.level >= OUTLINE_LEVELS.min && item.level <= OUTLINE_LEVELS.max,
  );
}
