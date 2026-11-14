import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {afterEach, describe, expect, it} from 'vitest';
import {appliedChoice, applyChoice, nextChoice, parseMode, resolveMode, THEME_ATTRIBUTE, THEME_STORAGE_KEY} from './themeMode';

const BOOT = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'public', 'theme-boot.js'), 'utf8');
const root = document.documentElement;

/** A Storage whose every call throws, as in a window with site data blocked. */
const blocked = (): Storage => {
  throw new DOMException('blocked', 'SecurityError');
};

afterEach(() => {
  root.removeAttribute(THEME_ATTRIBUTE);
  localStorage.clear();
  document.head.innerHTML = '';
});

describe('theme choice', () => {
  it('reads only light and dark as a stored mode', () => {
    expect(parseMode('dark')).toBe('dark');
    expect(parseMode('light')).toBe('light');
    expect(parseMode('system')).toBeNull();
    expect(parseMode(null)).toBeNull();
  });

  it('shows the OS mode until the reader chooses one', () => {
    expect(resolveMode('system', 'dark')).toBe('dark');
    expect(resolveMode('light', 'dark')).toBe('light');
  });

  it('switches to the other mode, and back to following the OS on the OS mode', () => {
    expect(nextChoice('system', 'light')).toBe('dark');
    expect(nextChoice('dark', 'light')).toBe('system');
    expect(nextChoice('system', 'dark')).toBe('light');
    expect(nextChoice('light', 'dark')).toBe('system');
    // A stored mode that the OS has since moved to: the next press still shows the other mode.
    expect(nextChoice('dark', 'dark')).toBe('light');
  });

  it('applies and stores a mode, and clears both for system', () => {
    applyChoice(root, 'dark');
    expect(root.getAttribute(THEME_ATTRIBUTE)).toBe('dark');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark');
    expect(appliedChoice(root)).toBe('dark');
    applyChoice(root, 'system');
    expect(root.hasAttribute(THEME_ATTRIBUTE)).toBe(false);
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBeNull();
    expect(appliedChoice(root)).toBe('system');
  });

  it('still shows the choice when storage is blocked', () => {
    expect(() => applyChoice(root, 'light', blocked)).not.toThrow();
    expect(root.getAttribute(THEME_ATTRIBUTE)).toBe('light');
  });
});

describe('public/theme-boot.js', () => {
  const boot = () => new Function(BOOT)();

  it('uses the same storage key and attribute as themeMode.ts', () => {
    expect(BOOT).toContain(`'${THEME_STORAGE_KEY}'`);
    expect(BOOT).toContain(`'${THEME_ATTRIBUTE}'`);
  });

  it('applies a stored mode to <html> and the color-scheme meta before the app runs', () => {
    document.head.innerHTML = '<meta name="color-scheme" content="light dark">';
    localStorage.setItem(THEME_STORAGE_KEY, 'dark');
    boot();
    expect(root.getAttribute(THEME_ATTRIBUTE)).toBe('dark');
    expect(document.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe('dark');
  });

  it('leaves the page following the OS without a valid stored mode', () => {
    document.head.innerHTML = '<meta name="color-scheme" content="light dark">';
    localStorage.setItem(THEME_STORAGE_KEY, 'sepia');
    boot();
    expect(root.hasAttribute(THEME_ATTRIBUTE)).toBe(false);
    expect(document.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe('light dark');
  });
});
