# web/AGENTS.md: frontend rules

Rules for every change to the files under this directory. The root `AGENTS.md` (rule priority,
workflow, CI, writing, commits) applies as well; numbers are in `REQUIREMENTS.md`.

## Frontend

### Components and modularity

- Use the unified components; do not reinvent existing ones.
- Finish any existing modularization; no half-done work, such as theme colors or i18n only partly
  extracted with the rest hardcoded.
- No literal text, color or size outside the design tokens and the locale files.
- Start a page from the design system's components and templates.

### Consistency

- Animations (if present) are unified in duration, easing and triggers. Animate only transform and
  opacity, never make the user wait for an exit, and respect reduced motion.
- One visual style.
- Notification levels (if present) are unified and used for their real meaning: info, warning,
  error, success.

### Layout, type and loading

The values (sizes, gaps, widths, breakpoints) live in the design system or `REQUIREMENTS.md`. These
are the rules those values must satisfy.

- **Type scale.**
  - Every text size and line height comes from one named scale: page title, section heading, card
    title, body, small or meta.
  - A role has one size on every page.
  - No literal font size in a page.
- **Spacing.**
  - Every gap and padding comes from one spacing scale.
  - The same relationship gets the same gap everywhere: between page sections, between cards,
    inside a card, between a label and its field.
  - No literal pixel value in a page.
- **Page titles.**
  - Every page has exactly one title, drawn by the shared page frame.
  - The title has the same position, size and distance from the content on every page.
  - The page's actions sit at the end of the title row.
  - No page draws its own title.
- **One width per kind of page.**
  - All pages of one kind share the frame's width, so the title row and the body end on the same
    edge everywhere.
  - A form that reads badly when stretched sits in a shared readable column inside the frame.
  - Tables and lists span the frame.
- **No large blank areas.**
  - At every supported width (phone, laptop, wide desktop), the content fills its frame by its
    natural layout, such as a wider table or a side column. Otherwise the frame is narrower.
  - A half-empty screen is a defect, and so is a side gap that only one page has.
- **Alignment.**
  - Everything on a page lines up on a few shared edges.
  - A column's content aligns with its header: text and actions from the header's start, numbers
    and amounts at the end.
  - Cells in a row of unequal height align to the top.
  - Labels and fields of one row share a baseline.
- **Centring.**
  - Content meant to be centred is centred both horizontally and vertically in its container: a
    sign-in card, an empty state, a dialog, a one-column flow.
  - Centring is done by layout (flex or grid), never by guessed padding.
  - Icon and text inside a control are vertically centred. Check this with CJK text, whose metrics
    differ from Latin.
- **Values never break.**
  - Dates, times, amounts, ids and codes never wrap in the middle.
  - Truncated text shows its full value on hover, on focus and on touch (a phone has no hover).
- **Skeletons.** Page-level loading uses the page's skeleton; an action on a control (a button, a
  row) shows its loading state on that control. Neither uses a full-page spinner.
  - Each page has one skeleton: its own frame, title and sections at their final sizes, shown from
    the first paint.
  - Nothing moves when the data arrives: no layout shift, and no spinner followed by a skeleton.
  - Only content that loads gets a skeleton.
  - Empty and error states are designed states of their own, never a skeleton that stays.
- **Feedback.**
  - Every click visibly responds.
  - An async action shows its loading state.
  - Success and failure are obvious; no silent buttons.
  - One primary action per view.
  - Labels name the action; never "OK".
  - Irreversible destructive actions ask for confirmation.
- **Phone width.**
  - No horizontal page scroll and nothing cut off.
  - No information is reachable only by hover.
  - A table either fits or switches to a stacked layout.
- **Checked by screenshot.**
  - Before review, shoot each changed page at phone and desktop widths, in every theme (if
    present), with real data.
  - A shot of an error or empty state does not count as checking the page.
  - Show a new page or flow to the owner as one screen first, before building the rest.

### i18n (if present)

- No missing translations and no hardcoded strings.
- Consistent key naming; no duplicate or unused keys.

### Usability

- Generic and easy to use.
- Before designing a user or admin flow, compare how mainstream products do it (GitHub, Google,
  Apple, Microsoft; Stripe, Wise and Revolut for money). Link them, follow them, and justify any
  divergence.
- Compatible across browsers, screen sizes and themes (if present).

### Accessibility

- Every action works with the keyboard alone, with visible focus.
- After a route change, focus starts at the page title.
- WCAG AA contrast in every theme (if present): 4.5:1 for normal text, 3:1 for large text, 3:1 for
  UI components and focus indicators against their surroundings.

### Performance

- Watch bundle size, render cost and memory against the baseline.
- No unnecessary dependencies.
