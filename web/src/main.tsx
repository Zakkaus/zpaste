import '@astryxdesign/core/reset.css';
import '@astryxdesign/core/astryx.css';
import '@gentoozh/astryx-theme/gentoo.css';
import './theme/figtree.css';
import {StrictMode} from 'react';
import {createRoot} from 'react-dom/client';
import {App} from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
