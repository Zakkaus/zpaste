import {useMemo} from 'react';
import * as stylex from '@stylexjs/stylex';
import {AppShell} from '@astryxdesign/core/AppShell';
import {CodeBlock} from '@astryxdesign/core/CodeBlock';
import {HStack, VStack} from '@astryxdesign/core/Layout';
import {Section} from '@astryxdesign/core/Section';
import {Text} from '@astryxdesign/core/Text';
import {TopNav} from '@astryxdesign/core/TopNav';
import {Divider} from '@astryxdesign/core/Divider';
import {SPACE} from '@gentoozh/astryx-theme/scale';
import {ThemeToggle} from '../chrome/ThemeToggle';
import type {EmbeddedPaste} from '../shell/embedded';
import {t} from '../i18n/messages';
import {layout, media} from '../theme/layout.stylex';
import {isMarkdown, pasteExtension} from './languages';
import {outlineItems} from './markdownPlugins';
import {MarkdownView} from './MarkdownView';
import {OutlineDrawer, OutlineRail} from './PasteOutline';
import {PasteHeader} from './PasteHeader';

const styles = stylex.create({
  // Beside the rail the column only shrinks; minWidth 0 keeps wide content from widening the page.
  column: {width: '100%', flexShrink: 1, minWidth: 0},
  // An opaque bar with a hairline under it, so the chrome reads as its own region; its edges line up
  // with the page frame's content edges at every width.
  topBar: {
    backgroundColor: 'var(--color-background-surface)',
    borderBlockEndWidth: 'var(--border-width)',
    borderBlockEndStyle: 'solid',
    borderBlockEndColor: 'var(--color-border)',
    paddingInline: {default: layout.barInset, [media.narrow]: layout.barInsetNarrow},
  },
});

/** The reading page of an ordinary paste whose text the shell embedded. */
export function ReaderPage({data}: {data: EmbeddedPaste}) {
  const extension = pasteExtension(data.url_extension, data.paste.extension);
  const markdown = isMarkdown(extension);
  const outline = useMemo(() => (markdown ? outlineItems(data.content) : []), [markdown, data.content]);
  const hasOutline = outline.length > 0;

  return (
    <AppShell
      height="auto"
      variant="surface"
      mobileNav={false}
      topNav={
        <TopNav
          xstyle={styles.topBar}
          label={t('nav.label')}
          heading={
            <Text type="large" weight="semibold">
              {t('app.name')}
            </Text>
          }
          endContent={
            <HStack gap={SPACE.toolbar} vAlign="center">
              <ThemeToggle />
              {hasOutline ? <OutlineDrawer items={outline} /> : null}
            </HStack>
          }
        />
      }>
      <HStack gap={SPACE.group} vAlign="start" hAlign="center" width="100%">
        <Section maxWidth={layout.proseMaxWidth} padding={SPACE.page} xstyle={styles.column}>
          <VStack gap={SPACE.sectionHeading}>
            <VStack gap={SPACE.block}>
              <PasteHeader data={data} extension={extension} />
              <Divider />
            </VStack>
            {markdown ? (
              <MarkdownView source={data.content} />
            ) : (
              <CodeBlock code={data.content} language="plaintext" width="100%" hasLineNumbers />
            )}
          </VStack>
        </Section>
        {hasOutline ? <OutlineRail items={outline} /> : null}
      </HStack>
    </AppShell>
  );
}
