import {readEmbeddedPaste} from './shell/embedded';
import {ReaderPage} from './reader/ReaderPage';

/** Only the reading page of an embedded paste exists so far; other pages follow the first screen's approval. */
export function App() {
  const data = readEmbeddedPaste(document);
  return data == null ? null : <ReaderPage data={data} />;
}
