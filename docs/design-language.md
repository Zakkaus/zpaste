# Zpaste design language

This document holds the values and visual rules the Zpaste web app follows,
so that a screen that differs from a rule is changed, not the rule. All of it is
**target**: the web app does not exist yet, and the pages served today are
upstream wastebin's. Parts marked **needs owner approval** wait until the owner
has approved a first screen (see [Approval gates](#approval-gates)); they are
not to be spread to other pages before that.

Structure, data and the API are in [architecture.md](architecture.md). The
frontend rules every change follows are in [web/AGENTS.md](../web/AGENTS.md),
and the numbers (widths, type scale, spacing, bundle budgets) in
[REQUIREMENTS.md](../REQUIREMENTS.md). This document does not repeat either;
it decides what those rules leave to the product. Items the plan does not
decide are marked **open** and listed in [Open items](#open-items).

## Positioning

Zpaste is a reading tool. A person opens a link to read a plan, a log or a
review that someone else, often an AI agent, wrote. Every choice below follows
one consequence: **the text of the paste is the page, and nothing else on the
screen competes with it.** Controls, the outline and metadata stay outside the
reading column or collapse out of the way.

Zpaste shares tokens with gentoozh.org, not its layout. The home page of
gentoozh.org is a navigation surface; its large type and card grid would push a
1,600-line document below the fold and break it into boxes it does not have.

## Reading first

- **Phone first.** Every page is designed at phone width first. No width from
  320 to 1280 px scrolls the whole page sideways.
- **Readable column.** Prose sits in one readable column. Its width is a
  number in `REQUIREMENTS.md`, not set yet; it is decided with the P1 first
  screen.
- **Wide content scrolls in its own container.** A table, a code block or a
  diagram wider than the column scrolls inside its container; the page does not.
  At 390 px a Markdown table stays a table. Table headers stay fixed while the
  table scrolls (plan section 三). Astryx 0.7.0 `Table` offers sticky columns
  but no sticky header, so how the header stays fixed is open.
- **Outline.** A Markdown paste gets an outline from Astryx `Outline`, with
  heading anchors and scroll tracking. On wide screens it sits beside the
  column; below 1024 px it opens in an Astryx `Drawer` from the top bar.
- **Links to a place.** Headings and code lines have anchors, so a link can
  point to a line.
- **Pastes are not indexed.** Every paste page is `noindex`.

This conflicts with one line of [web/AGENTS.md](../web/AGENTS.md), "Phone
width": "A table either fits or switches to a stacked layout". Rendered
Markdown tables are user content with arbitrary columns, so the plan keeps them
as tables that scroll in their container. Recording this as an exception is
open (see [Open items](#open-items)).

## Tokens

Every colour, radius, type size and spacing value comes from the shared theme
package `gentoo-zh/gentoozh-astryx-theme`, pinned by git tag. Zpaste writes no
value of its own; when it needs a value the package lacks, the package gets the
token. The package keeps only tokens shared with gentoozh.org; values used only
by the site's home page stay in the site.

The names below are Astryx token names, which the package sets. Which of them
the package exports is open until its first tag. Values are never copied into
this document; read them from the package. The "Use in Zpaste" column is a
proposal and needs owner approval with the P1 first screen.

| Token | Use in Zpaste |
| --- | --- |
| `--color-background-body` | Page background. |
| `--color-background-surface` | Panels that must stand apart from the page: the side outline, the workspace list. |
| `--color-background-muted` | Code block background. |
| `--color-background-popover` | Menus and the phone outline drawer. |
| `--color-overlay` | Scrim behind the drawer and dialogs. |
| `--color-border` | Hairlines: between table rows, the line between outline and text. |
| `--color-border-emphasized` | Focused fields, the current outline entry's marker and the line under a table's header row. |
| `--color-text-primary` | Body text and headings. |
| `--color-text-secondary` | Metadata: expiry, version, line numbers. |
| `--color-text-disabled` | Disabled controls only. |
| `--color-accent` | The one primary action per view. |
| `--color-text-accent` | Links in rendered content and the current outline entry. |
| `--color-accent-muted` | Background of the current outline entry and a highlighted line. |
| Syntax tokens (`keyword`, `string`, `comment`, `number`, `function`, `type`, …) | Code colouring. One theme for every language. |
| `--radius-*` set | Each name keeps the use the package gives it; Zpaste writes no radius literal. |
| `--text-*` type scale | Page title, section heading, body, small or meta, per the roles in `web/AGENTS.md`. |

The rest of the axes the positioning leaves to decide:

| Axis | Decision | State |
| --- | --- | --- |
| Density | Reading comfort over scanning density: body line height for long text, CJK line height for Chinese. | Decided by the positioning; values need owner approval |
| Separation | Hairlines, no elevation and no boxes: an opaque top bar with a `--color-border` line under it, a line under the title row, and the outline rail set off by its label and track. | Proposed with the P1 first screen |
| Hues | The accent plus the semantic set; the mind map adds a few low-saturation branch colours (see below). | Branch colour tokens open |
| Motion | None decided beyond `web/AGENTS.md`. | Open |
| Theme | Light, dark and follow-system, switched as on gentoozh.org: one icon button in the top bar names the mode it switches to (a moon in light mode, a sun in dark mode), and pressing it on the OS's own mode goes back to following the OS. A chosen mode is kept in `localStorage` (`zpaste-theme`) and set as `data-theme` on `<html>` by `public/theme-boot.js`, a blocking script in the head, before the first paint; the Content Security Policy allows no inline script. Without a choice the page follows the OS, including a switch while it is open. Upstream's theme cookie is not read. | Proposed with the P1 first screen |

## The reading page's first screen

Proposed for the owner's approval with the P1 first screen; each value is a theme token or role.

- **Top bar.** Opaque `--color-background-surface` with a hairline under it. Its
  content edges are the page frame's: the product name starts where the paste
  text starts, at every width.
- **Title row.** The page's h1 takes the heading-2 role, one step above the body
  and one below the paste's own first heading, which carries the page. Details
  take the supporting role in `--color-text-secondary`. Copy is the view's one
  primary button; Edit, Raw and Download are secondary buttons of the same size,
  in that order: the two actions on the paste first, then the two that open it
  elsewhere. None carries an icon, as in gentoozh.org's toolbar rows. Copy's
  feedback ("Copied", "Failed") replaces its label without changing its width.
  Edit shows only when the shell says the reader may edit (`can_edit`). On a
  phone the buttons take their own row.
- **Theme switch.** At the end of the top bar, before the phone outline button
  (see the Theme row above).
- **Headings in the paste.** The paste's `#` renders as h2, because the title
  row holds the h1, but keeps the look of the level the author wrote: h2 takes
  the heading-1 role, h3 heading-2, and so on.
- **Tables.** Cells use the label role, not the reading body size, with compact
  padding, aligned to the top. Rows are told apart by `--color-border` hairlines
  alone: no column rules and no outer box, and the first and last columns line
  up with the text around the table. The header row is the label in
  `--color-text-secondary` on the page's own surface, over one
  `--color-border-emphasized` line. Inline code in a cell has the code face
  without its chip. Columns take the widths automatic layout gives their
  content, and code in a cell may wrap after `/` and `-`, so a table of up to
  three columns fits the column from tablet width up; a wider table scrolls in
  its box with an edge shadow. On a phone the first column stays on an opaque
  surface while the rest scrolls. It is as wide as its longest
  unbreakable piece and at most half of the table's visible box, padding
  included. Text in it wraps at spaces, after `/` and after `-`, never inside a
  name such as `REQUIREMENTS.md`; only a piece wider than half the box on its
  own, such as a long hash, breaks inside itself.
- **Outline.** The rail carries an "On this page" label level with the title.
  It lists the paste's `#` title and its sections down to `####`, so the entry
  marked current is the section on screen from the first load. An outline
  taller than the rail scrolls inside it, below the label, while the entry
  marked current still follows the page and stays in the rail's view.
  In the phone drawer each entry is at least the theme's touch height (44 px).

## Renderers

The reading page picks a renderer by type: the extension in the URL, else the
extension stored at creation, else plain text. Renderers load on demand; a paste
without mermaid never downloads mermaid.

| Type | Renderer | Rules |
| --- | --- | --- |
| `.md` | Astryx `Markdown` | Outline and heading anchors; GFM alerts; wide tables scroll in their container with a fixed header; a ` ```markdown ` block can switch between source and rendered. Raw HTML is never rendered. |
| Code | Astryx `CodeBlock` | Colouring, line numbers, line anchors, copy. Colouring uses classes, not inline styles (`highlightMode` `spans`). Copied text equals the visible text. |
| Logs and plain text | Plain text view | Line numbers and a wrap toggle. |
| `.diff`, `.patch` | Unified diff | Added and removed lines use the semantic colours, with the `+`/`-` sign kept, so colour is not the only carrier. |
| Diagrams (P1b) | mermaid | ` ```mermaid ` blocks and `.mermaid` files, including flowcharts, sequence diagrams and `mindmap`; `securityLevel: 'strict'`. |
| Mind map view (P1b) | Own drawing over a layout library | Any `.md` switches to it in one action; the switch lives in the URL (`?view=mindmap`). |

Astryx 0.7.16's built-in tokenizer covers JavaScript and TypeScript, JSON,
HTML, XML and SVG, CSS, SCSS and Less, Python, shell, PHP and Hack, YAML,
Markdown, C and C++, SQL, Rust, Kotlin, Swift and Go; a ` ```diff ` fence
renders as a unified diff. Any other language renders as plain text, with its
name in the block's header. Tokenizers for more languages come later, loaded
per language.

A component Astryx lacks is reported with `astryx gap-report`. Component source
is not copied and changed.

## Mind map

Needs owner approval (P1b first screen). A layout library (markmap or
`d3-flextree`; open, decided with the P1b screen) computes positions only;
nodes, edges and colours are drawn with the theme tokens.

- **Colour.** First-level branches use a few low-saturation colours; deeper
  levels only lighten the branch colour, never change it. Edges are thin,
  one colour, with a small curve. These colours are tokens in the package
  (open).
- **Text.** Nodes use the body size with CJK line height and wrap at a maximum
  node width.
- **Wide screens.** A horizontal map, two levels open by default, with a
  "fit to screen" control.
- **Narrow screens.** A vertical collapsible tree. A tap opens the next level
  and each node shows its child count. No zooming is needed to read it.
- **Navigation.** A node jumps to its section in the text, and the text's
  outline jumps back to the node.
- **Keyboard.** Arrow keys move, Enter expands, Escape collapses.
- **Export.** SVG and PNG.

## States

Each state has its own screen; none is a skeleton that never resolves.

| State | Screen |
| --- | --- |
| Loading | The page's own skeleton at its final geometry, from the first paint. An ordinary paste is embedded in the page and needs none. |
| Not found or expired | One screen for both, because the API does not tell them apart. |
| Password required | A password field; a wrong password (401) shows the error next to the field. |
| Burn confirmation | Says the paste is deleted after this read and has one button that reads it. Nothing is fetched before the press. |
| Encrypted and burn | Password first, then the confirmation; the paste is consumed only after both. |
| Burn failed | Says the paste may already have been read. There is no retry button. |
| Network error | Says the request failed and what to do next. A failed burn read uses the burn-failed screen instead. |
| Empty workspace (P2b) | Says the workspace holds no pastes yet. |
| Edit conflict (P2) | A 412 on save says the paste changed since it was loaded. What the page offers next is open. |

Copy for every state is in the locale catalogues; the error screens say what
happened and what to do next, per [web/AGENTS.md](../web/AGENTS.md).

## Languages

- Locales: zh-CN, zh-TW and en, one catalogue each with the same keys.
- Each Chinese catalogue keeps one script and its own vocabulary: zh-TW uses
  Taiwan terms, zh-CN mainland terms.
- API field names, gzpaste JSON keys and error `type` values are never
  translated.
- Layout is measured in the longest locale.
- How the locale is chosen (browser language, a cookie, the URL) is open.

## Accessibility

The baseline is [web/AGENTS.md](../web/AGENTS.md), "Accessibility". Zpaste adds:

- The outline drawer traps focus, closes on Escape and returns focus to the
  button that opened it.
- A line anchor or heading anchor moves focus to its target, not only the
  scroll position.
- The mind map's keyboard map above is complete: every node is reachable
  without a pointer.
- The burn confirmation button is a real `button`; reading never happens on
  focus or hover.
- Code and table containers that scroll are focusable, so the keyboard can
  scroll them.

## Approval gates

The plan requires one screen to be approved before a pattern spreads.

| Gate | Screen | Waits on it |
| --- | --- | --- |
| P1 reading page | `r8BaRKfkXfi` rendered at 1280 px light and 390 px dark | The type scale, spacing, readable column width, outline placement, table and code treatment; then the error, password and burn screens, the three locales and the new-paste page. |
| P1b mind map | markmap's default style and the token-drawn style side by side, 1280 px light and 390 px dark | The layout library choice and every rule in [Mind map](#mind-map). |
| P2b workspace | A workspace with several plans and results, 1280 px light and 390 px dark | The list grouping, the two-pane wide layout and the two-level phone flow. |

## Deliberately not doing

| Not doing | Why | Checked by |
| --- | --- | --- |
| Rendering or highlighting on the server | The upstream sample turned 76 KB of text into 754 KB of HTML; the browser renders the text instead. | Route size check (P1, `REQUIREMENTS.md`) |
| A sanitizer of our own | Astryx `Markdown` never renders raw HTML and handles unsafe URLs; mermaid runs strict; mind map text is React text. A second sanitizer would be a second set of rules to keep in step. The only escaping we write is the shell's JSON, with its `</script>` test. | Review; the shell test |
| Copying Astryx component source to patch it | A copied component stops receiving upstream fixes. Gaps go to `astryx gap-report`. | Review |
| Inline scripts or inline styles in the shell | The CSP forbids inline script; class-based colouring keeps styles out of the markup. | CSP header test (P1) |
| Loading mermaid or the mind map up front | Their size would land on every paste. | Route size check (P1b) |
| Zoom-to-read on phones | A mind map that must be pinched to read is not readable; the phone gets a collapsible tree. | P1b screen review |
| Stacking Markdown tables into cards on phones | User tables have arbitrary columns; stacking loses the column relation. | Screenshot at 390 px |
| A home-page look (large hero type, card grids) | It pushes the text below the fold; see [Positioning](#positioning). | Review |
| Indexing pastes | Pastes are unlisted by nature. | `noindex` test (P1) |
| WYSIWYG editing, real-time collaboration, full-text search | Out of scope for the product (plan section 六). | Review |

Deferred, not refused: ANSI colours and jump-to-error in logs, CSV and TSV
tables, side-by-side diffs, splitting multi-file pastes.

## Open items

| # | Item | Blocks |
| --- | --- | --- |
| D1 | How table headers stay fixed while Astryx 0.7.0 `Table` has no sticky header (gap report, an Astryx upgrade, or our own CSS). | P1 |
| D3 | The tokenizer for languages Astryx 0.7.16 does not cover (INI, TOML and others), loaded per language. | P1 |
| D4 | Which Astryx token names the theme package exports, and the tokens for mind map branch colours. | P1, P1b |
| D5 | Separation device (hairlines or elevation) and motion. | P1 first screen |
| D6 | Locale resolution before first paint. The theme is resolved before first paint and follows an OS switch (see Theme), pending the owner's approval. | P1 |
| D7 | Recording Markdown tables on phones as an exception to `web/AGENTS.md`, "Phone width", in `REQUIREMENTS.md`, "Exceptions". | P1 |
| D8 | Layout library for the mind map: markmap or `d3-flextree`. | P1b |
| D9 | What the edit page offers after a 412: reload, show a diff, or overwrite. gzpaste's behaviour is decided (plan section 五); the page's is not. | P2 |
