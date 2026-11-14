import {ErrorBoundary} from './chrome/ErrorBoundary';
import {ErrorPage} from './reader/ErrorPage';
import {ReaderPage} from './reader/ReaderPage';
import {readEmbeddedPaste} from './shell/embedded';
import {pasteIdFromPath} from './shell/location';

/**
 * Only the reading page of an embedded paste exists so far; other pages follow the first screen's
 * approval. A page without a readable paste, or one that fails to draw, shows the error state instead
 * of a blank page. The error state links the raw text only when the page did carry the paste, which the
 * server embeds for an ordinary paste alone: a page with nothing embedded may be for a paste that is
 * gone, or for a burn-after-reading paste, which `GET /raw/<id>` consumes.
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
  if (read.state === 'found') return <ReaderPage data={read.data} />;
  return <ErrorPage reason="missing" id={read.state === 'malformed' ? id : null} />;
}
