import stylex from '@stylexjs/unplugin/vite';
import react from '@vitejs/plugin-react';
import {defineConfig, type Plugin} from 'vite';
import {fixturePaste, withEmbeddedPaste} from './scripts/fixture.ts';

/** The dev server embeds a fixture paste in the page, in the shell's format, so the reading page has content. */
function devFixture(): Plugin {
  return {
    name: 'zpaste-dev-fixture',
    apply: 'serve',
    transformIndexHtml: html => withEmbeddedPaste(html, fixturePaste()),
  };
}

/** The same StyleX options as gentoozh.org: own class prefix, rules after the Astryx theme layer. */
export function stylexPlugin() {
  return stylex({
    dev: false,
    runtimeInjection: false,
    enableInlinedConditionalMerge: true,
    treeshakeCompensation: true,
    classNamePrefix: 'z',
    unstable_moduleResolution: {type: 'commonJS'},
    useCSSLayers: {before: ['reset', 'astryx-base', 'astryx-theme']},
  });
}

export default defineConfig({
  plugins: [
    react(),
    stylexPlugin(),
    devFixture(),
  ],
  build: {
    target: 'es2023',
  },
});
