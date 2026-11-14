import type {ReactNode} from 'react';
import * as stylex from '@stylexjs/stylex';
import {Heading} from '@astryxdesign/core/Heading';

const styles = stylex.create({
  // The page's h1 in the section-heading role: one step above body text, one below the paste's own
  // first heading, which carries the page (docs/design-language.md, "Positioning").
  heading: {
    fontSize: 'var(--text-heading-2-size)',
    fontWeight: 'var(--text-heading-2-weight)',
    lineHeight: 'var(--text-heading-2-leading)',
  },
});

/** The page's one h1, in the same role on every page. */
export function PageTitle({children}: {children: ReactNode}) {
  return (
    <Heading level={1} xstyle={styles.heading}>
      {children}
    </Heading>
  );
}
