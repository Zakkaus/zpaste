import {defineConfig} from 'vitest/config';
import {stylexPlugin} from './vite.config.ts';

export default defineConfig({
  // StyleX compiles the components under test as it does in the build. Its dev-server hook starts a
  // timer that only an HTTP server's close stops; Vitest's server has none, so the run would hang on
  // exit, and the hook is left out.
  plugins: [{...stylexPlugin(), configureServer: undefined}],
  test: {environment: 'jsdom', include: ['src/**/*.test.{ts,tsx}', 'scripts/**/*.test.ts']},
});
