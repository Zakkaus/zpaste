import {describe, expect, it} from 'vitest';
import {pasteIdFromPath} from './location';

describe('paste id from the reading URL', () => {
  it('reads 6- and 11-character ids, with or without an extension', () => {
    expect(pasteIdFromPath('/abc123')).toBe('abc123');
    expect(pasteIdFromPath('/abc123.md')).toBe('abc123');
    expect(pasteIdFromPath('/r8BaRKfkXfi.md')).toBe('r8BaRKfkXfi');
    expect(pasteIdFromPath('/a-b+c9')).toBe('a-b+c9');
  });

  it('finds none in other paths', () => {
    for (const path of ['/', '/long', '/abc12', '/abc1234', '/raw/abc123', '/abc123/edit', '/abc_12']) expect(pasteIdFromPath(path)).toBeNull();
  });
});
