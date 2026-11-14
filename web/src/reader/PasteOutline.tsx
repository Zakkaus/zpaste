// The outline of a Markdown paste: a sticky rail beside the column on wide screens (gentoozh.org's
// OutlineRow), a Drawer opened from the top bar below `media.narrow`.
import {useEffect, useRef, useState, type RefObject} from 'react';
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
    display: {default: 'flex', [media.narrow]: 'none'},
    position: 'sticky',
    top: `calc(var(--_app-shell-header-height, 0rem) + ${spacingVars['--spacing-6']})`,
    alignSelf: 'flex-start',
    flexShrink: 0,
    width: layout.outlineWidth,
    maxHeight: `calc(100dvh - var(--_app-shell-header-height, 0rem) - 2 * ${spacingVars['--spacing-6']})`,
  },
  // A long outline scrolls inside the rail, below its label, instead of running past the viewport.
  // The scroller is Outline's own root, not a wrapper: Outline tracks the nearest scrolling ancestor
  // of its root, so a scrolling wrapper (or the rail itself) would make it follow the rail's scroll
  // instead of the page's.
  railOutline: {
    minHeight: 0,
    flexShrink: 1,
    // Outline lays its list and its track out as one flex row. A single-line row in a box of fixed
    // height is only as tall as the box, so the track line ended one screen down the list; a wrapping
    // row is as tall as the list (the two items still fit on one line).
    flexWrap: 'wrap',
    overflowY: 'auto',
    overscrollBehavior: 'contain',
    scrollbarWidth: 'thin',
  },
  narrowOnly: {display: {default: 'none', [media.narrow]: 'block'}},
});

/** The rail's label sits level with the page title, so the rail starts where the column does. */
export function OutlineRail({items}: {items: OutlineItem[]}) {
  const outline = useRef<HTMLElement>(null);
  const [current, setCurrent] = useState<string>();
  useCurrentEntryInView(outline, current, true);
  return (
    <VStack as="aside" gap={SPACE.tight} xstyle={styles.rail} aria-labelledby={RAIL_LABEL_ID}>
      <Text as="h2" type="label" weight="semibold" color="secondary" id={RAIL_LABEL_ID}>
        {t('outline.label')}
      </Text>
      <Outline
        ref={outline}
        items={items}
        label={t('outline.label')}
        density="compact"
        xstyle={styles.railOutline}
        onActiveIdChange={setCurrent}
      />
    </VStack>
  );
}

const RAIL_LABEL_ID = 'zpaste-outline-label';

/**
 * Keeps the entry an outline marks current inside the visible part of what scrolls the outline: the
 * rail's own list, or the drawer's body. That box scrolls by the least amount that shows the entry;
 * the page never moves. Runs when page scrolling marks another entry and when the outline is shown.
 */
function useCurrentEntryInView(outline: RefObject<HTMLElement | null>, current: string | undefined, isShown: boolean) {
  useEffect(() => {
    const root = outline.current;
    if (!isShown || root == null) return;
    const frame = requestAnimationFrame(() => {
      const link = root.querySelector('[aria-current]');
      const box = scrollerOf(root);
      if (link == null || box == null) return;
      const outer = box.getBoundingClientRect();
      const inner = link.getBoundingClientRect();
      if (inner.top < outer.top) box.scrollTop -= outer.top - inner.top;
      else if (inner.bottom > outer.bottom) box.scrollTop += inner.bottom - outer.bottom;
    });
    return () => cancelAnimationFrame(frame);
  }, [outline, current, isShown]);
}

/** The element itself if it scrolls, else its nearest ancestor that scrolls, below the page's own scroller. */
function scrollerOf(element: HTMLElement): HTMLElement | null {
  for (let node: HTMLElement | null = element; node != null && node !== document.body; node = node.parentElement) {
    const {overflowY} = getComputedStyle(node);
    if ((overflowY === 'auto' || overflowY === 'scroll') && node.scrollHeight > node.clientHeight) return node;
  }
  return null;
}

/** The top-bar button and the Drawer it opens; shown below `media.narrow` only. */
export function OutlineDrawer({items}: {items: OutlineItem[]}) {
  const [isOpen, setOpen] = useState(false);
  const target = useRef<string | null>(null);
  const outline = useRef<HTMLElement>(null);
  const [current, setCurrent] = useState<string>();
  useCurrentEntryInView(outline, current, isOpen);
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
          ref={outline}
          items={items}
          label={t('outline.label')}
          density="default"
          onActiveIdChange={setCurrent}
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
