// Reading URLs are `/<id>[.<ext>]` (docs/architecture.md, "Serving a page"); an id is 6 or 11 characters
// of the server's alphabet (crates/wastebin_core/src/id.rs).
const READING_PATH = /^\/([A-Za-z0-9+-]{6}|[A-Za-z0-9+-]{11})(?:\.[^/]+)?$/;

/** The paste id in a reading URL's path, or null when the path is not one. */
export function pasteIdFromPath(pathname: string): string | null {
  return READING_PATH.exec(pathname)?.[1] ?? null;
}
