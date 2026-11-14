import * as stylex from '@stylexjs/stylex';
import {AppShell} from '@astryxdesign/core/AppShell';
import {Button} from '@astryxdesign/core/Button';
import {HStack, VStack} from '@astryxdesign/core/Layout';
import {Section} from '@astryxdesign/core/Section';
import {Text} from '@astryxdesign/core/Text';
import {BUTTON, SPACE} from '@gentoozh/astryx-theme/scale';
import {PageTitle} from '../chrome/PageTitle';
import {TopBar} from '../chrome/TopBar';
import {t} from '../i18n/messages';
import {layout} from '../theme/layout.stylex';

const styles = stylex.create({
  column: {width: '100%', minWidth: 0},
});

/**
 * The reading page's error state, in the reading page's frame: what happened and what to do next
 * (docs/design-language.md, "States"). `missing`: the page carries no readable paste; `render`: drawing
 * it failed. With the paste's id, the raw text is one link away; the caller passes the id only for an
 * ordinary paste (App.tsx).
 */
export function ErrorPage({reason, id}: {reason: 'missing' | 'render'; id: string | null}) {
  return (
    <AppShell height="auto" variant="surface" mobileNav={false} topNav={<TopBar />}>
      <HStack hAlign="center" width="100%">
        <Section maxWidth={layout.proseMaxWidth} padding={SPACE.page} xstyle={styles.column}>
          <VStack gap={SPACE.block} hAlign="start">
            <PageTitle>{t('error.title')}</PageTitle>
            <Text>
              {t(reason === 'missing' ? 'error.missing' : 'error.render')} {t(id == null ? 'error.nextReload' : 'error.nextRaw')}
            </Text>
            {id == null ? null : <Button variant="primary" size={BUTTON.toolbar} label={t('error.openRaw')} href={`/raw/${id}`} />}
          </VStack>
        </Section>
      </HStack>
    </AppShell>
  );
}
