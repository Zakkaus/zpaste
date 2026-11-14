import {ErrorBoundary} from './chrome/ErrorBoundary';
import {ErrorPage} from './reader/ErrorPage';
import {ReaderPage} from './reader/ReaderPage';
import {readEmbeddedPaste} from './shell/embedded';
import {pasteIdFromPath} from './shell/location';

/**
 * Only the reading page of an embedded paste exists so far; other pages follow the first screen's
 * approval. A page without a readable paste, or one that fails to draw, shows the error state instead
 * of a blank page.
 */
export function App() {
  const id = pasteIdFromPath(window.location.pathname);
  return (
    <ErrorBoundary fallback={<ErrorPage reason="render" id={id} />}>
      <Reader id={id} />
    </ErrorBoundary>
  );
}

function Reader({id}: {id: string | null}) {
  const read = readEmbeddedPaste(document);
  return read.state === 'found' ? <ReaderPage data={read.data} /> : <ErrorPage reason="missing" id={id} />;
}
