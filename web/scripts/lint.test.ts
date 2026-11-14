// The literal-value gates catch what web/AGENTS.md bans and pass what it allows.
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {ESLint} from 'eslint';
import {describe, expect, it} from 'vitest';
import {literalsIn} from './lint-css.ts';

const eslint = new ESLint({cwd: join(dirname(fileURLToPath(import.meta.url)), '..')});
const banned = async (code: string) =>
  (await eslint.lintText(code, {filePath: 'src/reader/Probe.tsx'}))[0]!.messages.filter(message => message.ruleId === 'no-restricted-syntax').length;

describe('literal values in TypeScript', () => {
  it('bans lengths in any unit, also inside min(), max(), calc() and clamp(), and colours', async () => {
    for (const value of ['16px', '1.5rem', '2em', '50%', '30vw', '0.5rem', 'min(100%, 16rem)', 'max(0px, 12px)', 'calc(100% - 2rem)', 'clamp(1rem, 2vw, 3rem)', '#fff', 'rgb(0 0 0)', 'oklch(70% 0.1 200)']) {
      expect(await banned(`export const x = {width: '${value}'};`), value).toBe(1);
    }
    expect(await banned('export const x = `calc(${"a"} + 8px)`;')).toBe(1);
  });

  it('passes tokens, zero and the whole box', async () => {
    for (const value of ['100%', '100dvh', 'var(--spacing-6)', 'calc(var(--_app-shell-header-height, 0rem) + var(--spacing-6))', '1 / 1', 'calc(100dvh - 2 * var(--spacing-6))']) {
      expect(await banned(`export const x = {width: '${value}'};`), value).toBe(0);
    }
  });
});

describe('literal values in stylesheets', () => {
  it('finds lengths and colours, outside comments', () => {
    expect(literalsIn('.a {\n  /* 12px in a comment */\n  margin: 12px;\n  color: #333;\n  width: calc(50cqi - var(--spacing-2));\n}')).toEqual([
      '3: margin: 12px;',
      '4: color: #333;',
      '5: width: calc(50cqi - var(--spacing-2));',
    ]);
  });

  it('passes tokens, zero, 100% and a declaration marked literal-ok', () => {
    expect(literalsIn('.a {\n  max-width: 100%;\n  margin: 0;\n  padding: var(--spacing-2);\n}')).toEqual([]);
    expect(literalsIn('/* literal-ok: the theme breakpoint */\n@media (width < 640px) {\n  .a { margin: 4px; }\n}')).toEqual(['3: .a { margin: 4px; }']);
    expect(literalsIn('.a {\n  /*\n   * literal-ok: geometry\n   */\n  background:\n    radial-gradient(at 100% 50%, red) right / 3px;\n  margin: 2px;\n}')).toEqual(['7: margin: 2px;']);
  });
});
