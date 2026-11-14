// The run-time style hashes: the texts are the ones Astryx inserts, and a bundle without an injector is caught.
import {createHash} from 'node:crypto';
import {describe, expect, it} from 'vitest';
import {cspHash, missingInjectors, styleTexts, svgStyleTexts} from './style-hashes.ts';

describe('run-time style hashes', () => {
  it('hashes the highlight colours and the slide rules', async () => {
    const [highlight, slide] = await styleTexts();
    expect(highlight).toContain('code::highlight(astryx-keyword)');
    expect(slide).toContain('@position-try --astryx-layer-slide-');
    expect(cspHash('a')).toBe(`'sha256-${createHash('sha256').update('a').digest('base64')}'`);
  });

  it('names an injector the bundle lacks', () => {
    expect(missingInjectors('data-astryx-highlight-styles astryx-layer-slide-rules')).toEqual([]);
    expect(missingInjectors('data-astryx-highlight-styles')).toEqual(['layer slide rules']);
  });

  it('reads the text of each style element of an SVG file', () => {
    expect(svgStyleTexts('<svg><style>path{fill:red}</style><path/><style type="text/css">\na{}\n</style></svg>')).toEqual(['path{fill:red}', '\na{}\n']);
    expect(svgStyleTexts('<svg><path/></svg>')).toEqual([]);
  });
});
