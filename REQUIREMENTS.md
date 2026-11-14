# REQUIREMENTS.md: numbers, gates and exceptions

Commands, numbers, gates and exceptions for this repository. The rules themselves are in `AGENTS.md`,
`.github/AGENTS.md`, `web/AGENTS.md` and `crates/AGENTS.md` and are not repeated here. When another
file in this repository disagrees with this one on a command, a number, a gate or an exception, this
file wins.

`scripts/check-requirements.py` checks this file in the merge gate. It reads files and runs
nothing. Write a pipe inside a cell as `\|`. Write a command in backticks; the check confirms that
a script named in it exists.

## Rule files

Every `AGENTS.md` in this repository, by path. The check reads rule groups only from the files
marked `yes`, and fails on an `AGENTS.md` it finds that is not listed. It searches every directory
except `.git`, `node_modules`, `vendor`, `third_party` and hidden directories other than
`.github`. Mark a file `no`, with the reason in Note, when its rules are not this project's (a
vendored package, a generated tree).

| File | Checked | Note |
| --- | --- | --- |
| AGENTS.md | yes | |
| .github/AGENTS.md | yes | |
| web/AGENTS.md | yes | `web/` holds the first reading screen (P1); rows without a gate yet are under "Not yet enforced" |
| crates/AGENTS.md | yes | |

## Commands

One row per tier in `.github/AGENTS.md`, "Tiers, jobs and gates". Write the exact command that a
person and CI both run. Fill in this order: the command; then three runs on the same host, recorded
as the baseline; then the limit, as the baseline plus a margin (start at 20%).

| Tier | Command | Runs (where, when) | Wall-time limit | Baseline (date, host, load) |
| --- | --- | --- | --- | --- |
| Pre-commit | | | | |
| Merge gate | `cargo clippy -- -Dwarnings`; `cargo fmt --all --check`; `cargo test --all-targets`; `python3 -I scripts/check-requirements.py`; `python3 -I scripts/test_check_requirements.py`; `scripts/test_check_version_tag.sh`; `./build-docker-image.sh`; in `web/`: `pnpm install --frozen-lockfile`; `pnpm lint` (ESLint and `node web/scripts/lint-css.ts`); `pnpm typecheck`; `pnpm test`; `pnpm build`; `pnpm check:size`; then, after `pnpm build`, `node web/scripts/shoot.ts <out-dir> --ci` once per fixture as the main paste (`web/fixtures/sample.md`, `web/fixtures/chinese-hant.md`, `web/fixtures/chinese-hans.md`); locally the same without `--ci`, which adds the first-paint check (slow link and CPU, CLS under 0.01), and with `ZPASTE_FIXTURE` set to the real paste `r8BaRKfkXfi`, kept outside the repository | GitHub Actions on every pull request and every push to main: `rust.yml` (jobs "Test and lint", "Requirements check" and "Version tag check") `image.yml` (job "Build image", which builds the same Dockerfile target as the script and, on main, also pushes it) and `web.yml` (job "Lint, test and build", and job "Screenshots", one run per fixture in Playwright's pinned Chromium with `fonts-noto-cjk`); the same commands run locally | 25 s for the cargo and Python commands, cold target directory; 170 s for the image build with no build cache (CI uses a layer cache, so its time is lower); 8 s for the `web/` commands with no `node_modules` and a warm pnpm store; 58 s for one `shoot.ts --ci` run of the sample paste; all on the desktop below; CI wall time not measured | cargo and Python: 21.1 s median of 3 cold runs (21.3, 20.8, 21.1 s), 2026-11-14, desktop `gentoo` (32 threads), load 9.4 to 14.1; tests alone 3.8, 3.8, 3.9 s warm. Image: 135.4 s median of 3 runs of `./build-docker-image.sh --no-cache --network host` on rootless Docker (132.2, 137.8, 135.4 s; the limit is 20% over the slowest, rounded up), 2026-11-14, desktop `gentoo` (32 threads), load 4.9 to 8.8. Web: 6.2 s median of 3 runs (6.2, 5.8, 6.6 s; the limit is 20% over the slowest, rounded up), 2026-11-15, desktop `gentoo` (32 threads), load 3.6 to 4.0. `shoot.ts --ci`, sample paste: 42.8 s median of 3 runs (47.8, 42.8, 41.0 s; the limit is 20% over the slowest, rounded up), Chrome 157, 2026-11-15, desktop `gentoo`, load 13 to 19 |
| Scheduled run on main | | | | |
| Release | `scripts/check-version-tag.sh "$TAG"`; `scripts/export-executables.sh dist --image IMAGE` | `release.yml`, on a `v*` tag: checks the tag against Cargo.toml and main, builds and pushes the image once, copies the executables out of that image, creates the release from `CHANGELOG.md` with the image digest in its notes and attaches the x86_64 musl executables; `image.yml` never pushes a version | 170 s for the image build; the push, pull, export and release steps are not measured, and CI wall time is not measured | the merge-gate image build: 135.4 s median of 3 runs (132.2, 137.8, 135.4 s), 2026-11-14, desktop `gentoo` (32 threads), load 4.9 to 8.8 |

## Numbers

Every limit has a measured baseline next to it. Record the date, the host and its load for each
baseline. Leave the limit empty until it has been measured, and list it under "Not yet enforced"
meanwhile.

| Item | Limit | Baseline (date, host, load) | Enforced by |
| --- | --- | --- | --- |
| Type scale (role: size / line height) | top-bar name: 17 / 24; page title: 20 / 28; paste #: 24 / 32; paste ##: 20 / 28; paste ###: 17 / 24; paste ####: 14 / 20; body: 17 / 28; details: 12 / 20; table cell: 14 / 20; table header: 14 / 20; outline label: 14 / 20; outline entry: 14 / 20; button: 14 / 20; inline code: 14 / 23.06; code block: 14 / 20 | the same values: computed font size and line height in px of the reading page, production build, Chrome 157, identical at 390 and 1280 px and in the synthetic, Chinese and real (`r8BaRKfkXfi`) pastes, 2026-11-15, desktop `gentoo` (32 threads), load 2.8 to 3.7. Each role is a theme token; the roles, the tokens and the comparison with gentoozh.org are in `docs/design-language.md`, "Type scale" | design system (`@gentoozh/astryx-theme` and Astryx roles); `node web/scripts/shoot.ts` reads this row and fails when a role renders another size or line height at 390 or 1280 px; it runs in the merge gate (`web.yml`, job "Screenshots") |
| Spacing scale (section, card, inside card, label to field) | in px: page padding 24; title row to paste 24; title row to its rule 16; title to details 4; between actions 8; column to outline rail 32; rail label to outline 8; paste blocks, as Astryx `Markdown` spaces them: 12 between paragraphs and from a heading to its text, 8 from a heading to a code block, 16 before a table, a quote or a `####` heading, 24 before a `##` or `###` heading; table cell 6 block, 8 inline; cards and label-to-field: none on the reading page | the same values: computed gaps and padding of the reading page, production build, Chrome 157, identical at 390 and 1280 px, 2026-11-15, desktop `gentoo` (32 threads), load 2.8 to 3.7; each is a `SPACE` step of `@gentoozh/astryx-theme` or Astryx `Markdown` spacing | design system (`SPACE` steps of `@gentoozh/astryx-theme`); review |
| Page frame width; readable column width | | | e2e page-width check |
| Supported widths (phone, laptop, wide) | | | screenshots |
| Full end-to-end run, wall time | | | |
| Slowest test package | 2.5 s (`wastebin_core`) | 2.01 s, `wastebin_core`, identical in 3 warm `cargo test --all-targets` runs (2.01, 2.01, 2.01 s; next slowest `wastebin` 1.1 s), 2026-11-14, desktop `gentoo` (32 threads), load 7.1 to 9.9 | review of the `cargo test` output |
| Frontend entry chunk, gzip | 324,000 bytes | 269,809 bytes: `web/dist` entry script, gzip level 9, identical in 3 builds of the first reading screen (React, Astryx `Markdown`, `CodeBlock`, `Outline`, `Drawer`, `AppShell`), 2026-11-15, desktop `gentoo` (32 threads), load 3.6 to 4.0; the limit is 20% over, rounded up | `node web/scripts/check-size.ts` |
| Frontend total JS + CSS, gzip | 464,000 bytes | 386,441 bytes: every `.js` and `.css` file in `web/dist/assets`, lazy chunks included, gzip level 9, identical in the same 3 builds; fonts excluded | `node web/scripts/check-size.ts` |
| Paste size the reading page renders | Markdown with its outline up to 100,000 characters; plain text up to 1,000,000 characters; past that only the start, cut after a line (`web/src/reader/renderPlan.ts`) | the longest main-thread task while the page loads, production build, Chrome 157, CPU 4x slower, median of 3 runs, generated code-heavy Markdown (`longFixture.ts` repeated): as Markdown 0.30 s at 2 KB, 1.0 s at 64 KB, 1.3 s at 96 KB, 1.6 s at 128 KB, 2.0 s at 256 KB, 3.9 s at 512 KB, 7.8 s at 1 MB, 15 s at 2 MB, 32 s at 4 MB; as plain text 0.21 s at 256 KB, 0.50 s at 1 MB, 1.6 s at 4 MB, 5.9 s at 16 MB; 2026-11-15, desktop `gentoo` (32 threads), load 2 to 4. The Markdown limit keeps the plan's 76 KB sample paste (`r8BaRKfkXfi`) as Markdown, near the 1 s mark; with the guard, 4,000,000 characters block the page for 0.54 s | `pnpm --dir web test` (`renderPlan.test.ts`); `node web/scripts/shoot.ts` fails when a paste past a limit is not shown as plain text under its notice, or blocks the page for 2 s or more at 4x CPU |
| Release server executable size | 13,300,000 bytes | 11,028,216 bytes (`zpaste`, copied out of the amd64 image: static x86_64 musl, release profile, `strip = "debuginfo"`, `lto = "fat"`, built with zig); identical sha256 in 3 cold image builds, 2026-11-14, desktop `gentoo` (32 threads), load 4.9 to 8.8; `wastebin-ctl` 3,748,520 bytes | review |
| Source file size, p90 of this repository (pass bar) | 437 lines | 437 lines: nearest-rank p90 of the 63 tracked `.rs`, `.html`, `.css`, `.js` and `.sql` files under `crates/` (largest: `style.css` 1,221 lines), 2026-11-14, desktop `gentoo`; recount when the upstream templates and CSS are removed | review |
| Wait timeout before a stuck step is recorded and left | | a setting, not measured | agent |
| Context handoff limit (share of the model's window) | | a setting, not measured | orchestrator |
| Status check interval while several jobs run | | a setting, not measured | orchestrator |
| Commit subject length | 72 characters | a setting, not measured | commit hook |

## Enforcement

One row per rule group of every file checked under "Rule files": each `###` heading, named
`path: heading`, and each rule tagged `[rule: <id>]`, named `path#id`. The check reports a group
with no row, and a row whose group no longer exists, so renaming a heading means renaming its row.

Fill one or both of:

- Gate command: the command, in backticks, that turns red on a violation, run by a tier in
  "Commands";
- Review criterion: the `REVIEW.md` section that checks it, with the file name in backticks.

Or fill only "Does not apply because" with what the project lacks (no admin panel, no i18n). A
tagged rule marked as not applying needs the owner's approval, quoted in the commit body.

| Rule group | Gate command | Review criterion | Does not apply because |
| --- | --- | --- | --- |
| AGENTS.md: Change risk | | `REVIEW.md`, "Scope" | |
| AGENTS.md: Workflow | | `REVIEW.md`, "Pass bar" | |
| AGENTS.md: When unsure | | `REVIEW.md`, "Before you start" | |
| AGENTS.md: Waiting | | | a rule about an agent's session, not a diff; no gate or review can observe it |
| AGENTS.md: Reporting | | `REVIEW.md`, "CI, docs and commits" | |
| AGENTS.md: Code quality | `cargo clippy -- -Dwarnings`; `cargo fmt --all --check` | `REVIEW.md`, "Pass bar" | |
| AGENTS.md: Impact and edge cases | | `REVIEW.md`, "Scope" | |
| AGENTS.md: Secrets | | `REVIEW.md`, "Checking against the rules", Sensitive data | |
| AGENTS.md: Performance baseline | | `REVIEW.md`, "Every finding has evidence" | |
| AGENTS.md: Tests (if present) | `cargo test --all-targets`; `python3 -I scripts/test_check_requirements.py` | `REVIEW.md`, "CI, docs and commits" | |
| AGENTS.md: Merging | | `REVIEW.md`, "Report" | |
| AGENTS.md: Model usage | | `REVIEW.md`, "Before you start" | |
| AGENTS.md: Documentation | | `REVIEW.md`, "CI, docs and commits" | |
| AGENTS.md: Writing (docs, comments, reports, PR and issue text) | | `REVIEW.md`, "CI, docs and commits" | |
| AGENTS.md: Commits and pull requests | | `REVIEW.md`, "CI, docs and commits" | |
| AGENTS.md: Frontend–backend mapping (when both exist) | | | |
| .github/AGENTS.md: Tiers, jobs and gates | `python3 -I scripts/check-requirements.py` | `REVIEW.md`, "CI, docs and commits" | |
| web/AGENTS.md: Components and modularity | `pnpm --dir web lint`, which runs ESLint and `node web/scripts/lint-css.ts` | | |
| web/AGENTS.md: Consistency | | | |
| web/AGENTS.md: Layout, type and loading | | | |
| web/AGENTS.md: i18n (if present) | | | |
| web/AGENTS.md: Usability | | | |
| web/AGENTS.md: Accessibility | | | |
| web/AGENTS.md: Performance | `node web/scripts/check-size.ts` | | |
| crates/AGENTS.md: APIs and modularity | | `REVIEW.md`, "Pass bar" | |
| crates/AGENTS.md: Compatibility | | `REVIEW.md`, "Checking against the rules", Migrations | |
| crates/AGENTS.md: Logging | | `REVIEW.md`, "Checking against the rules", Sensitive data | |
| crates/AGENTS.md: Security (if present) | | `REVIEW.md`, "Checking against the rules", Authorization | |
| crates/AGENTS.md#authz | | `REVIEW.md`, "Checking against the rules", Authorization | |
| crates/AGENTS.md#sensitive-values | | `REVIEW.md`, "Checking against the rules", Sensitive data | |
| crates/AGENTS.md: Money and state (if present) | | `REVIEW.md`, "Checking against the rules", Money and state (retried and concurrent requests; Zpaste moves no money) | |
| crates/AGENTS.md#money-idempotent | | | Zpaste handles no payments or balances |
| crates/AGENTS.md#ledger | | | Zpaste has no ledger or balances |
| crates/AGENTS.md: Background work (if present) | | `REVIEW.md`, "Checking against the rules" | |
| crates/AGENTS.md: Operations (if present) | | `REVIEW.md`, "Checking against the rules" | |
| crates/AGENTS.md: Admin panel (if present) | | | Zpaste has no admin panel; the plan rules out an account system and quotas (plan v11, section 六, the "not doing" list) |
| crates/AGENTS.md: Performance | | `REVIEW.md`, "Pass bar" | |
| crates/AGENTS.md: Verification | | `REVIEW.md`, "Checking against the rules" | |

## Exceptions

Backend endpoints with no frontend caller:

| Endpoint | Caller | Reason |
| --- | --- | --- |

Intentional designs that look like defects:

| What | Why | Owner ruling (date, quote) |
| --- | --- | --- |
| A wide Markdown table in a paste scrolls horizontally inside its own box at phone width instead of fitting or stacking (`web/AGENTS.md`, "Phone width"). The page itself never scrolls horizontally, the first column stays fixed, and an edge shadow shows that more columns are off screen. | Stacking a data table into one card per row breaks comparison down a column, and the reader does not control the table's width. | 2026-11-14, 「可以。」 on the proposed exception, confirmed as horizontal scrolling (「左右滑動？」) |

## Not yet enforced

A row of "Commands", "Numbers" or "Enforcement" that is not filled yet goes here, named
`Section: row`, for example `Numbers: Slowest test package`. The merge gate fails on:

- an unfilled row that is not listed here;
- a listed row that is already filled, or that does not exist;
- a Due date that has passed.

Due holds one date (YYYY-MM-DD). Moving it later needs the owner's approval, quoted in the commit
body.

| Row | Current state | Plan | Due |
| --- | --- | --- | --- |
| Commands: Pre-commit | no pre-commit hook exists; the fast read-only check would be `cargo fmt --all --check` | add a versioned hook that runs it and a script that installs it | 2026-12-14 |
| Commands: Scheduled run on main | no scheduled workflow exists | add one for the heavy suites once any exist (full end-to-end in P1) | 2027-01-31 |
| Numbers: Page frame width; readable column width | not measured; `web/` holds only the first reading screen, awaiting the owner's approval | set with the design language in P1; checked by the end-to-end page-width check (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Supported widths (phone, laptop, wide) | not measured; `web/` holds only the first reading screen, awaiting the owner's approval | plan names 320 to 1280 px with no horizontal page scroll, 390 px for tables; confirm against the first screen (plan v11, sections 三 and 六) | 2027-01-31 |
| Numbers: Full end-to-end run, wall time | not measured; `web/` holds only the first reading screen, awaiting the owner's approval | measure after the first end-to-end suite lands in P1 (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Wait timeout before a stuck step is recorded and left | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Numbers: Context handoff limit (share of the model's window) | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Numbers: Status check interval while several jobs run | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Enforcement: AGENTS.md: Frontend–backend mapping (when both exist) | no gate yet; `web/` holds only the first reading screen, awaiting the owner's approval | fill when the first `/v1` endpoint and its frontend caller exist, with the contract test from plan section 四 (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Consistency | no gate yet; `web/` holds only the first reading screen, awaiting the owner's approval | add the frontend gates and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Layout, type and loading | partly checked: `node web/scripts/shoot.ts` runs in the merge gate (type scale, no sideways page scroll from 320 to 1280 px, tables, outline tracking, actions, large pastes, error state); the first-paint check runs only locally; no review row yet | add the page-width numbers and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: i18n (if present) | no gate yet; `web/` holds only the first reading screen, awaiting the owner's approval | add the locale check for zh-CN, zh-TW and en (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Usability | no gate yet; `web/` holds only the first reading screen, awaiting the owner's approval | add the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Accessibility | no gate yet; `web/` holds only the first reading screen, awaiting the owner's approval | add the accessibility check and the review row (plan v11, section 六, P1) | 2027-01-31 |
