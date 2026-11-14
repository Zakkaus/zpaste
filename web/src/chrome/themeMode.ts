// The light/dark choice, as gentoozh.org keeps it (src/ui/chrome/themeMode.tsx there): no stored choice
// means the page follows the OS, which the theme's light-dark() tokens do on their own; a stored choice
// sets `data-theme` on <html>, which the theme reads (`html[data-theme]` sets `color-scheme`).
// public/theme-boot.js applies the stored choice before the first paint with the same key and attribute.

/** A colour mode the page can show. */
export type ColorMode = 'light' | 'dark';
/** What the reader chose: follow the OS, or one mode. */
export type ThemeChoice = 'system' | ColorMode;

/** localStorage key shared with public/theme-boot.js. */
export const THEME_STORAGE_KEY = 'zpaste-theme';
/** The attribute on <html> that the theme package reads. */
export const THEME_ATTRIBUTE = 'data-theme';
export const DARK_QUERY = '(prefers-color-scheme: dark)';

/** A stored or applied value as a mode; anything else means no choice. */
export function parseMode(value: string | null | undefined): ColorMode | null {
  return value === 'light' || value === 'dark' ? value : null;
}

/** The mode the page shows for `choice` while the OS is in `system` mode. */
export function resolveMode(choice: ThemeChoice, system: ColorMode): ColorMode {
  return choice === 'system' ? system : choice;
}

/**
 * The choice after one press of the toggle: the other mode, except that landing on the OS's mode goes
 * back to following the OS, so one button covers system, light and dark.
 */
export function nextChoice(choice: ThemeChoice, system: ColorMode): ThemeChoice {
  const next: ColorMode = resolveMode(choice, system) === 'dark' ? 'light' : 'dark';
  return next === system ? 'system' : next;
}

/** The choice the page carries: the attribute public/theme-boot.js set from storage, else system. */
export function appliedChoice(root: HTMLElement): ThemeChoice {
  return parseMode(root.getAttribute(THEME_ATTRIBUTE)) ?? 'system';
}

/**
 * Shows `choice` and remembers it. Storage can be blocked (a private window, cleared site data); the
 * choice then lasts for this page only.
 */
export function applyChoice(root: HTMLElement, choice: ThemeChoice, storage: () => Storage = () => window.localStorage) {
  if (choice === 'system') root.removeAttribute(THEME_ATTRIBUTE);
  else root.setAttribute(THEME_ATTRIBUTE, choice);
  try {
    if (choice === 'system') storage().removeItem(THEME_STORAGE_KEY);
    else storage().setItem(THEME_STORAGE_KEY, choice);
  } catch {
    // Nothing to do: the attribute above already shows the choice.
  }
}
