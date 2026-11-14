import {Code} from '@astryxdesign/core/Code';
import {CodeBlock} from '@astryxdesign/core/CodeBlock';
import {Markdown, type MarkdownComponents} from '@astryxdesign/core/Markdown';
import {MARKDOWN_PLUGINS} from './markdownPlugins';
import css from './MarkdownView.module.css';

/**
 * A fenced block in full: Markdown's default collapses long blocks, which would hide most of a paste
 * made of files. Lines wrap so a phone reads them without scrolling each line sideways. A language
 * CodeBlock does not know renders as plain text with its name in the header.
 */
function FencedCode({code, language}: {code: string; language?: string}) {
  return <CodeBlock code={code} language={language ?? 'plaintext'} highlightMode="spans" width="100%" isWrapped isCollapsible={false} />;
}

/**
 * Inline code as Astryx draws it, with a break opportunity (`<wbr>`, which adds no text, so a copied
 * path is the path as written) after each `/`. The stylesheet lets those opportunities act only in a
 * phone table's fixed first column, where a long path wraps at its separators instead of mid-name;
 * elsewhere they are hidden and inline code wraps as before. Supplying this renderer replaces the one
 * Astryx uses in table cells, whose only change keeps a token whole; the stylesheet restores that.
 */
function InlineCode({children}: {children: string}) {
  const parts = children.split(/(?<=\/)(?=.)/);
  return <Code>{parts.flatMap((part, index) => (index === 0 ? [part] : [<wbr key={index} />, part]))}</Code>;
}

const COMPONENTS: MarkdownComponents = {code: FencedCode, inlineCode: InlineCode};

/**
 * A Markdown paste. Astryx Markdown renders React elements only and never raw HTML, so no sanitizer
 * runs here. `#` maps to h2 because the page title row holds the page's h1.
 */
export function MarkdownView({source}: {source: string}) {
  return (
    <Markdown
      className={css.body}
      contentWidth="100%"
      headingLevelStart={2}
      autolink="gfm"
      plugins={MARKDOWN_PLUGINS}
      components={COMPONENTS}>
      {source}
    </Markdown>
  );
}
