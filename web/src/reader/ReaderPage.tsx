import {useMemo} from 'react';
import * as stylex from '@stylexjs/stylex';
import {AppShell} from '@astryxdesign/core/AppShell';
import {Banner} from '@astryxdesign/core/Banner';
import {CodeBlock} from '@astryxdesign/core/CodeBlock';
import {HStack, VStack} from '@astryxdesign/core/Layout';
import {Section} from '@astryxdesign/core/Section';
import {Divider} from '@astryxdesign/core/Divider';
import {SPACE} from '@gentoozh/astryx-theme/scale';
import {TopBar} from '../chrome/TopBar';
import type {EmbeddedPaste} from '../shell/embedded';
import {t} from '../i18n/messages';
import {layout} from '../theme/layout.stylex';
import {isMarkdown, pasteExtension} from './languages';
import {outlineItems} from './markdownPlugins';
import {MarkdownView} from './MarkdownView';
import {OutlineDrawer, OutlineRail} from './PasteOutline';
import css from './PasteContent.module.css';
import {pasteLanguage} from './pasteLanguage';
import {PasteHeader} from './PasteHeader';
import {MARKDOWN_LIMIT, renderPlan, TEXT_LIMIT} from './renderPlan';

const styles = stylex.create({
  // Beside the rail the column only shrinks; minWidth 0 keeps wide content from widening the page.
  column: {width: '100%', flexShrink: 1, minWidth: 0},
});

const NOTICES = {
  plain: () => t('large.plain', {limit: MARKDOWN_LIMIT.toLocaleString('en')}),
  truncated: () => t('large.truncated', {limit: TEXT_LIMIT.toLocaleString('en')}),
} as const;

/**
 * The reading page of an ordinary paste whose text the shell embedded. A paste too large to draw
 * quickly (renderPlan.ts) is shown as plain text under a notice, with no outline.
 */
export function ReaderPage({data}: {data: EmbeddedPaste}) {
  const extension = pasteExtension(data.url_extension, data.paste.extension);
  const plan = useMemo(() => renderPlan(data.content, isMarkdown(extension)), [data.content, extension]);
  const outline = useMemo(() => (plan.renderer === 'markdown' ? outlineItems(data.content) : []), [plan, data.content]);
  const hasOutline = outline.length > 0;
  // The paste's own language, on its text only: the chrome stays in the interface language.
  const language = useMemo(() => pasteLanguage(data.content), [data.content]);

  return (
    <AppShell
      height="auto"
      variant="surface"
      mobileNav={false}
      topNav={<TopBar>{hasOutline ? <OutlineDrawer items={outline} /> : null}</TopBar>}>
      <HStack gap={SPACE.group} vAlign="start" hAlign="center" width="100%">
        <Section maxWidth={layout.proseMaxWidth} padding={SPACE.page} xstyle={styles.column}>
          <VStack gap={SPACE.sectionHeading}>
            <VStack gap={SPACE.block}>
              <PasteHeader data={data} extension={extension} />
              <Divider />
            </VStack>
            {plan.renderer === 'text' && plan.notice != null ? <Banner status="info" title={t('large.title')} description={NOTICES[plan.notice]()} /> : null}
            <div lang={language} className={css.content}>
              {plan.renderer === 'markdown' ? (
                <MarkdownView source={data.content} />
              ) : (
                <CodeBlock code={plan.text} language="plaintext" width="100%" hasLineNumbers />
              )}
            </div>
          </VStack>
        </Section>
        {hasOutline ? <OutlineRail items={outline} /> : null}
      </HStack>
    </AppShell>
  );
}
