// The reading page's measures. Proposed with the P1 first screen and moved into REQUIREMENTS.md
// ("Page frame width; readable column width") once the owner approves it.
import * as stylex from '@stylexjs/stylex';

// Named once here, because the top bar's insets below are computed from them.
const PROSE_MAX_WIDTH = '800px';
const OUTLINE_WIDTH = '232px';

export const layout = stylex.defineConsts({
  /** The reading column; gentoozh.org's `proseMaxWidth`. */
  proseMaxWidth: PROSE_MAX_WIDTH,
  /** The outline rail beside the column on wide screens; gentoozh.org's `tocWidth`. */
  outlineWidth: OUTLINE_WIDTH,
  /**
   * The narrowest the title row gets before the actions wrap below it, so a long title keeps a readable
   * line next to them; never wider than the row itself.
   */
  titleMinWidth: 'min(100%, 16rem)',
  /**
   * The top bar's inline padding, which puts its edges on the page frame's content edges: the frame is
   * the column, the gap (`SPACE.group`) and the rail (gentoozh.org's `sidebarPageMaxWidth`), centred,
   * and the column is padded by `SPACE.page`. 100% is the bar's width.
   */
  barInset: `calc(max(0px, (100% - (${PROSE_MAX_WIDTH} + var(--spacing-8) + ${OUTLINE_WIDTH})) / 2) + var(--spacing-6))`,
  /** The same below `media.narrow`, where the frame is the column alone. */
  barInsetNarrow: `calc(max(0px, (100% - ${PROSE_MAX_WIDTH}) / 2) + var(--spacing-6))`,
});

export const media = stylex.defineConsts({
  /** Below this width the outline rail gives way to the drawer (gentoozh.org's `media.toc`). */
  narrow: '@media (max-width: 1024px)',
});
