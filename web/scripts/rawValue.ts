// What web/AGENTS.md ("Components and modularity") bans outside the theme: a literal colour, or a
// literal length in any unit, also inside calc(), min(), max() and clamp(). Zero, `100%` and the full
// viewport (`100dvh`, `100vw`, ...) are not sizes and pass. Shared by eslint.config.js (TypeScript)
// and lint-css.ts (stylesheets); its source has no `/`, so ESLint's selector syntax can hold it.
export const RAW_VALUE =
  /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|hwb|oklch|oklab)\(|(?<![\w.-])(?!0(?![.\d]))(?!100[dsl]?v[wh]\b)\d*\.?\d+(?:px|r?em|ch|ex|pt|cq[iwhb]|[dsl]?v(?:[whib]|min|max))\b|(?<![\w.-])(?!100%)\d*\.?\d+%/;
