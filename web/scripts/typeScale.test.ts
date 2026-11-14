import {describe, expect, it} from 'vitest';
import {TYPE_ROLES, documentedTypeScale, readTypeScale} from './typeScale.ts';

describe('the type scale row', () => {
  it('names the roles shoot.ts checks, each with a size and a line height', () => {
    const scale = readTypeScale();
    expect([...scale.keys()]).toEqual(Object.keys(TYPE_ROLES));
    for (const {size, leading} of scale.values()) {
      expect(size).toBeGreaterThan(0);
      expect(leading).toBeGreaterThanOrEqual(size);
    }
  });

  it('rejects an entry that is not "role: size / line height"', () => {
    const row = '| Type scale (role: size / line height) | body: 17 / 28; title 20 | baseline | gate |';
    expect(() => documentedTypeScale(row)).toThrow('title 20');
  });
});
