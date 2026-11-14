import {readFileSync} from 'node:fs';
import {renderToStaticMarkup} from 'react-dom/server';
import {describe, expect, it} from 'vitest';
import {Logo} from './Logo';
import {TopBar} from './TopBar';

const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');
const parse = (markup: string, type: DOMParserSupportedType) => new DOMParser().parseFromString(markup, type);

/** Every group and path of a drawing, in order, with the attributes that place and shape it. */
function shapes(root: Document | Element): string[] {
  return [...root.querySelectorAll('g, path')].map(element =>
    [element.tagName.toLowerCase(), element.getAttribute('transform') ?? '', element.getAttribute('d') ?? ''].join(' '),
  );
}

describe('Logo', () => {
  it('draws the shapes of src/brand/lockup-light.svg in the same places', () => {
    const artwork = parse(read('../brand/lockup-light.svg'), 'image/svg+xml');
    const drawn = parse(renderToStaticMarkup(<Logo />), 'text/html');
    expect(shapes(drawn.body)).toEqual(shapes(artwork));
  });

  it('takes its colours from the light and dark artwork', () => {
    const colours = (file: string) => {
      const artwork = parse(read(file), 'image/svg+xml');
      return {shape: artwork.querySelector('path[fill-rule]')?.getAttribute('fill'), seal: artwork.querySelector('path[stroke]')?.getAttribute('stroke')};
    };
    const light = colours('../brand/lockup-light.svg');
    const dark = colours('../brand/lockup-dark.svg');
    const tokens = read('../theme/brand.stylex.ts');
    expect(tokens).toContain(`shape: 'light-dark(${light.shape}, ${dark.shape})'`);
    expect(tokens).toContain(`seal: 'light-dark(${light.seal}, ${dark.seal})'`);
  });

  it('is decorative inside the top bar link named Zpaste that leads home', () => {
    const page = parse(renderToStaticMarkup(<TopBar />), 'text/html');
    const link = page.querySelector('a:has(svg)');
    expect(link?.getAttribute('aria-label')).toBe('Zpaste');
    expect(link?.getAttribute('href')).toBe('/');
    expect([...(link?.querySelectorAll('svg') ?? [])].every(svg => svg.getAttribute('aria-hidden') === 'true')).toBe(true);
  });
});
