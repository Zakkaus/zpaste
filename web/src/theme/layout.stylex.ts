// The reading page's measures. Proposed with the P1 first screen and moved into REQUIREMENTS.md
// ("Page frame width; readable column width") once the owner approves it.
import * as stylex from '@stylexjs/stylex';

export const layout = stylex.defineConsts({
  /** The reading column; gentoozh.org's `proseMaxWidth`. */
  proseMaxWidth: '800px',
  /** The outline rail beside the column on wide screens; gentoozh.org's `tocWidth`. */
  outlineWidth: '232px',
  /**
   * The top bar's inline padding, which puts its edges on the page frame's content edges: the frame is
   * the column, the gap (`SPACE.group`) and the rail (gentoozh.org's `sidebarPageMaxWidth`), centred,
   * and the column is padded by `SPACE.page`. 100% is the bar's width.
   */
  barInset: 'calc(max(0px, (100% - (800px + var(--spacing-8) + 232px)) / 2) + var(--spacing-6))',
  /** The same below `media.narrow`, where the frame is the column alone. */
  barInsetNarrow: 'calc(max(0px, (100% - 800px) / 2) + var(--spacing-6))',
});

export const media = stylex.defineConsts({
  /** Below this width the outline rail gives way to the drawer (gentoozh.org's `media.toc`). */
  narrow: '@media (max-width: 1024px)',
});
