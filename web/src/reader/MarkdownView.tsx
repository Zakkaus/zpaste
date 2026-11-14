import {useState} from 'react';
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
 * path is the path as written) after each `/`. The stylesheet lets those opportunities act only in
 * table cells, where a long path wraps at its separators instead of mid-name or widening the table;
 * elsewhere they are hidden and inline code wraps as before. Supplying this renderer replaces the one
 * Astryx uses in table cells, whose only change keeps a token whole; the stylesheet restores that.
 */
function InlineCode({children}: {children: string}) {
  const parts = children.split(/(?<=\/)(?=.)/);
  return <Code>{parts.flatMap((part, index) => (index === 0 ? [part] : [<wbr key={index} />, part]))}</Code>;
}

/**
 * An image from the paste, which may point anywhere: it loads only as it nears the screen, sends no
 * `Referer` (the paste's URL would reach the image's host), and one that fails to load, or that the
 * page's Content Security Policy blocks, gives way to its alt text in brackets, as Astryx draws an image
 * whose address it rejects. Supplying this renderer drops Astryx's block wrapper; the stylesheet
 * restores its spacing.
 */
function MarkdownImage({src, alt}: {src: string; alt: string}) {
  const [failed, setFailed] = useState(false);
  if (failed) return <span className={css.image}>{alt === '' ? null : `[${alt}]`}</span>;
  return <img className={css.image} src={src} alt={alt} loading="lazy" decoding="async" referrerPolicy="no-referrer" onError={() => setFailed(true)} />;
}

const COMPONENTS: MarkdownComponents = {code: FencedCode, inlineCode: InlineCode, image: MarkdownImage};

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
