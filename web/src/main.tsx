import '@astryxdesign/core/reset.css';
import '@astryxdesign/core/astryx.css';
import '@gentoozh/astryx-theme/gentoo.css';
import '@fontsource/figtree/400.css';
import '@fontsource/figtree/500.css';
import '@fontsource/figtree/600.css';
import {StrictMode} from 'react';
import {createRoot} from 'react-dom/client';
import {App} from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
