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

/**
 * Preloads the Latin Figtree files src/theme/figtree.css declares, so they load beside the script rather
 * than after the stylesheet and the first render ask for them: their faces are `font-display: optional`,
 * and a face that is not there for the first paint is not used at all.
 */
function preloadFonts(): Plugin {
  return {
    name: 'zpaste-preload-fonts',
    apply: 'build',
    transformIndexHtml: {
      order: 'post',
      handler: (_html, context) =>
        Object.keys(context.bundle ?? {})
          .filter(file => /\/figtree-latin-\d+-normal-[\w-]+\.woff2$/.test(file))
          .sort()
          .map(file => ({
            tag: 'link',
            attrs: {rel: 'preload', href: `/${file}`, as: 'font', type: 'font/woff2', crossorigin: ''},
            injectTo: 'head' as const,
          })),
    },
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
    preloadFonts(),
  ],
  build: {
    target: 'es2023',
  },
});
