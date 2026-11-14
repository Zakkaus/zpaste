// Gates for web/AGENTS.md, "Components and modularity": no literal colour or pixel value outside
// src/theme/, and no visible text outside the locale catalogues.
import reactHooks from 'eslint-plugin-react-hooks';
import tseslint from 'typescript-eslint';

const RAW_VALUE = String.raw`/#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|oklch)\(|\b\d+(?:\.\d+)?px\b/`;
const TEXT = String.raw`/\p{L}/u`;

export default tseslint.config(
  {ignores: ['dist/', 'node_modules/']},
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: {'react-hooks': reactHooks},
    rules: {
      ...reactHooks.configs.recommended.rules,
      'no-restricted-syntax': [
        'error',
        {selector: `Literal[value=${RAW_VALUE}]`, message: 'Colours and sizes come from the theme tokens or src/theme/.'},
        {selector: `TemplateElement[value.raw=${RAW_VALUE}]`, message: 'Colours and sizes come from the theme tokens or src/theme/.'},
        {selector: `JSXText[value=${TEXT}]`, message: 'Visible text comes from the locale catalogue (src/i18n/).'},
      ],
    },
  },
  {
    // The theme module holds the measures; tests may write any literal.
    files: ['src/theme/**', 'src/**/*.test.{ts,tsx}'],
    rules: {'no-restricted-syntax': 'off'},
  },
);
