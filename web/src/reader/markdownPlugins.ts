import {createMarkdownHeadingLinks} from '@astryxdesign/core/Markdown/plugins';
import {parseOutlineFromMarkdown} from '@astryxdesign/core/Outline';
import type {OutlineItem} from '@astryxdesign/core/Outline';

/** One plugin list for the rendered document and its outline, so every outline entry lands on its heading. */
export const MARKDOWN_PLUGINS = [createMarkdownHeadingLinks()];

/** Headings the outline lists: the document's sections and their subsections, not its title. */
const OUTLINE_LEVELS = {min: 2, max: 4};

export function outlineItems(markdown: string): OutlineItem[] {
  return parseOutlineFromMarkdown(markdown, {plugins: MARKDOWN_PLUGINS}).filter(
    item => item.level >= OUTLINE_LEVELS.min && item.level <= OUTLINE_LEVELS.max,
  );
}
