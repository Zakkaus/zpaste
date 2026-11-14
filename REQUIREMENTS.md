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
| web/AGENTS.md | yes | The file exists before `web/` does; its rows are under "Not yet enforced" until P1 |
| crates/AGENTS.md | yes | |

## Commands

One row per tier in `.github/AGENTS.md`, "Tiers, jobs and gates". Write the exact command that a
person and CI both run. Fill in this order: the command; then three runs on the same host, recorded
as the baseline; then the limit, as the baseline plus a margin (start at 20%).

| Tier | Command | Runs (where, when) | Wall-time limit | Baseline (date, host, load) |
| --- | --- | --- | --- | --- |
| Pre-commit | | | | |
| Merge gate | `cargo clippy -- -Dwarnings`; `cargo fmt --all --check`; `cargo test --all-targets`; `python3 -I scripts/check-requirements.py`; `python3 -I scripts/test_check_requirements.py`; `./build-docker-image.sh` | GitHub Actions on every pull request and every push to main: `rust.yml` (jobs "Test and lint" and "Requirements check") and `image.yml` (job "Build image", which builds the same Dockerfile target as the script and, on main, also pushes it); the same commands run locally | 25 s for the cargo and Python commands, cold target directory; 230 s for the image build with no build cache; both on the desktop below; CI wall time not measured | cargo and Python: 21.1 s median of 3 cold runs (21.3, 20.8, 21.1 s), 2026-11-14, desktop `gentoo` (32 threads), load 9.4 to 14.1; tests alone 3.8, 3.8, 3.9 s warm. Image: 154.5 s median of 3 runs of `docker build --no-cache --target amd64` on rootless Docker (154.5, 151.5, 191.3 s; the limit is 20% over the slowest), 2026-11-14, desktop `gentoo` (32 threads), load 3.6 to 9.2 |
| Scheduled run on main | | | | |
| Release | `scripts/check-version-tag.sh "$TAG"`; `scripts/export-executables.sh dist` | `release.yml`, on a `v*` tag: checks the tag, creates the release from `CHANGELOG.md` and attaches the x86_64 musl executables; `image.yml` pushes the image for the same tag | 230 s, no build cache (the export is the merge-gate image build plus a copy) | the merge-gate image build: 154.5 s median of 3 runs (154.5, 151.5, 191.3 s), 2026-11-14, desktop `gentoo` (32 threads), load 3.6 to 9.2 |

## Numbers

Every limit has a measured baseline next to it. Record the date, the host and its load for each
baseline. Leave the limit empty until it has been measured, and list it under "Not yet enforced"
meanwhile.

| Item | Limit | Baseline (date, host, load) | Enforced by |
| --- | --- | --- | --- |
| Type scale (role: size / line height) | | | design system |
| Spacing scale (section, card, inside card, label to field) | | | design system |
| Page frame width; readable column width | | | e2e page-width check |
| Supported widths (phone, laptop, wide) | | | screenshots |
| Full end-to-end run, wall time | | | |
| Slowest test package | 2.5 s (`wastebin_core`) | 2.01 s, `wastebin_core`, identical in 3 warm `cargo test --all-targets` runs (2.01, 2.01, 2.01 s; next slowest `wastebin` 1.1 s), 2026-11-14, desktop `gentoo` (32 threads), load 7.1 to 9.9 | review of the `cargo test` output |
| Frontend entry chunk, gzip | | | |
| Frontend total JS + CSS, gzip | | | |
| Release server executable size | 16,300,000 bytes | 13,565,200 bytes (`wastebin`, release profile, `strip = "debuginfo"`, `lto = "fat"`); identical in 3 cold builds, 2026-11-14, desktop `gentoo` (32 threads), load 8.8 to 11.2 | review |
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
| web/AGENTS.md: Components and modularity | | | |
| web/AGENTS.md: Consistency | | | |
| web/AGENTS.md: Layout, type and loading | | | |
| web/AGENTS.md: i18n (if present) | | | |
| web/AGENTS.md: Usability | | | |
| web/AGENTS.md: Accessibility | | | |
| web/AGENTS.md: Performance | | | |
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
| Numbers: Type scale (role: size / line height) | not measured; `web/` does not exist until P1 | P1 sets the scale in the design-language document and measures it in the first screen shown to the owner (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Spacing scale (section, card, inside card, label to field) | not measured; `web/` does not exist until P1 | set with the type scale in P1 (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Page frame width; readable column width | not measured; `web/` does not exist until P1 | set with the design language in P1; checked by the end-to-end page-width check (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Supported widths (phone, laptop, wide) | not measured; `web/` does not exist until P1 | plan names 320 to 1280 px with no horizontal page scroll, 390 px for tables; confirm against the first screen (plan v11, sections 三 and 六) | 2027-01-31 |
| Numbers: Full end-to-end run, wall time | not measured; `web/` does not exist until P1 | measure after the first end-to-end suite lands in P1 (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Frontend entry chunk, gzip | not measured; `web/` does not exist until P1 | P1 acceptance requires cold-load transfer, first readable time and the scroll of a 1,600-line document inside a budget; the plan gives no figure, so measure the first reading page, then set the limit (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Frontend total JS + CSS, gzip | not measured; `web/` does not exist until P1 | set with the entry chunk; mermaid and mind map load on demand and stay outside it (P1b) (plan v11, section 六, P1) | 2027-01-31 |
| Numbers: Wait timeout before a stuck step is recorded and left | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Numbers: Context handoff limit (share of the model's window) | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Numbers: Status check interval while several jobs run | the owner has not set a value for this repository | the owner sets it; ask with the next batch of open questions | 2026-12-14 |
| Enforcement: AGENTS.md: Frontend–backend mapping (when both exist) | no gate yet; `web/` does not exist until P1 | fill when the first `/v1` endpoint and its frontend caller exist, with the contract test from plan section 四 (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Components and modularity | no gate yet; `web/` does not exist until P1 | add the frontend gates (lint, type check, build) and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Consistency | no gate yet; `web/` does not exist until P1 | add the frontend gates and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Layout, type and loading | no gate yet; `web/` does not exist until P1 | add the screenshot and page-width checks and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: i18n (if present) | no gate yet; `web/` does not exist until P1 | add the locale check for zh-CN, zh-TW and en (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Usability | no gate yet; `web/` does not exist until P1 | add the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Accessibility | no gate yet; `web/` does not exist until P1 | add the accessibility check and the review row (plan v11, section 六, P1) | 2027-01-31 |
| Enforcement: web/AGENTS.md: Performance | no gate yet; `web/` does not exist until P1 | add the route-size check from the Numbers rows above (plan v11, section 六, P1) | 2027-01-31 |
