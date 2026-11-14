// Applies the stored light/dark choice before the first paint, so a reader who chose dark never sees a
// light frame. The key and attribute are src/chrome/themeMode.ts's THEME_STORAGE_KEY and THEME_ATTRIBUTE.
// A blocking file rather than an inline script: the Content Security Policy allows no inline script
// (docs/architecture.md, invariant 5).
(function () {
  try {
    var choice = localStorage.getItem('zpaste-theme');
    if (choice !== 'light' && choice !== 'dark') return;
    document.documentElement.setAttribute('data-theme', choice);
    // The canvas before the stylesheet applies (a development server injects CSS from JavaScript).
    var meta = document.querySelector('meta[name="color-scheme"]');
    if (meta != null) meta.setAttribute('content', choice);
  } catch {
    // Storage can be blocked; the page then follows the OS.
  }
})();
