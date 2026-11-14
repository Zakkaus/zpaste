// The logo's colours and its size in the top bar. The colours are the artwork's own
// (src/brand/NOTICE.md): the light files' on a light page, the dark files' on a dark one. light-dark()
// follows `color-scheme`, which the theme sets from the OS or from the switch's `data-theme`.
import * as stylex from '@stylexjs/stylex';

export const brand = stylex.defineConsts({
  /** The signet and the lettering. */
  shape: 'light-dark(#514776, #C4BEDB)',
  /** The 中 and 文 strokes drawn over the signet. */
  seal: 'light-dark(#EEEAF4, #514776)',
  /**
   * The lockup's height: gentoozh.org's top-bar logo box (`LAYOUT.wordmarkHeight`, 40 px), with the
   * signet nearly filling it as gentoozh.org's does (38.6 of 40 px against 38.4).
   */
  height: '40px',
  /** The signet's width at that height. */
  markWidth: '38px',
  /** The lettering's width at that height, its gap to the signet included. */
  wordmarkWidth: '126px',
});

export const brandMedia = stylex.defineConsts({
  /**
   * Below this width the lettering gives way to the signet alone, so the top bar keeps a clear gap
   * before its buttons.
   */
  markOnly: '@media (max-width: 359px)',
});
