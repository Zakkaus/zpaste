// The outline of a Markdown paste: a sticky rail beside the column on wide screens (gentoozh.org's
// OutlineRow), a Drawer opened from the top bar below `media.narrow`.
import {useEffect, useRef, useState} from 'react';
import * as stylex from '@stylexjs/stylex';
import {spacingVars} from '@astryxdesign/core/theme/tokens.stylex';
import {Button} from '@astryxdesign/core/Button';
import {Drawer, DrawerHeader} from '@astryxdesign/core/Drawer';
import {Icon} from '@astryxdesign/core/Icon';
import {VStack} from '@astryxdesign/core/Layout';
import {Outline, type OutlineItem} from '@astryxdesign/core/Outline';
import {Text} from '@astryxdesign/core/Text';
import {BUTTON, SPACE} from '@gentoozh/astryx-theme/scale';
import {t} from '../i18n/messages';
import {layout, media} from '../theme/layout.stylex';
import css from './PasteOutline.module.css';

const styles = stylex.create({
  rail: {
    display: {default: 'block', [media.narrow]: 'none'},
    position: 'sticky',
    top: `calc(var(--_app-shell-header-height, 0rem) + ${spacingVars['--spacing-6']})`,
    alignSelf: 'flex-start',
    flexShrink: 0,
    width: layout.outlineWidth,
    // A long outline scrolls inside the rail instead of running past the viewport.
    maxHeight: `calc(100dvh - var(--_app-shell-header-height, 0rem) - 2 * ${spacingVars['--spacing-6']})`,
    overflowY: 'auto',
    overscrollBehavior: 'contain',
    scrollbarWidth: 'thin',
  },
  narrowOnly: {display: {default: 'none', [media.narrow]: 'block'}},
});

/** The rail's label sits level with the page title, so the rail starts where the column does. */
export function OutlineRail({items}: {items: OutlineItem[]}) {
  return (
    <VStack as="aside" gap={SPACE.tight} xstyle={styles.rail} aria-labelledby={RAIL_LABEL_ID}>
      <Text as="h2" type="label" weight="semibold" color="secondary" id={RAIL_LABEL_ID}>
        {t('outline.label')}
      </Text>
      <Outline items={items} label={t('outline.label')} density="compact" />
    </VStack>
  );
}

const RAIL_LABEL_ID = 'zpaste-outline-label';

/** The top-bar button and the Drawer it opens; shown below `media.narrow` only. */
export function OutlineDrawer({items}: {items: OutlineItem[]}) {
  const [isOpen, setOpen] = useState(false);
  const target = useRef<string | null>(null);
  // The modal drawer locks page scrolling and returns focus to its button as it closes, so the jump to
  // the chosen heading waits until it has closed, then moves focus to the heading.
  useEffect(() => {
    const id = target.current;
    if (isOpen || id == null) return;
    target.current = null;
    const frame = requestAnimationFrame(() => {
      const heading = document.getElementById(id);
      if (heading == null) return;
      heading.scrollIntoView({block: 'start'});
      heading.tabIndex = -1;
      heading.focus({preventScroll: true});
    });
    return () => cancelAnimationFrame(frame);
  }, [isOpen]);
  return (
    <VStack xstyle={styles.narrowOnly}>
      <Button
        variant="ghost"
        size={BUTTON.toolbar}
        label={t('outline.open')}
        icon={<Icon icon="menu" />}
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
      />
      <Drawer isOpen={isOpen} onOpenChange={setOpen} label={t('outline.label')} side="end">
        <DrawerHeader title={t('outline.label')} onOpenChange={setOpen} />
        <Outline
          items={items}
          label={t('outline.label')}
          density="default"
          className={css.drawer}
          hasScrollOnClick={false}
          onNavigateStart={id => {
            target.current = id;
            setOpen(false);
          }}
        />
      </Drawer>
    </VStack>
  );
}
