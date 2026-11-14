import type {ReactNode} from 'react';
import * as stylex from '@stylexjs/stylex';
import {HStack} from '@astryxdesign/core/Layout';
import {Text} from '@astryxdesign/core/Text';
import {TopNav} from '@astryxdesign/core/TopNav';
import {SPACE} from '@gentoozh/astryx-theme/scale';
import {t} from '../i18n/messages';
import {layout, media} from '../theme/layout.stylex';
import {ThemeToggle} from './ThemeToggle';

const styles = stylex.create({
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

/** The top bar of every page: the app's name, the theme switch, then the page's own controls (`children`). */
export function TopBar({children}: {children?: ReactNode}) {
  return (
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
          {children}
        </HStack>
      }
    />
  );
}
