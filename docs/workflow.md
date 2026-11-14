# Zpaste workflow

This document gives this repository's values for the development workflow:
branch names, phases, review routing, the deployment runbook and when
documents change. The phases and the runbook are **target** steps from the
working plan, [plan.zh-TW.md](plan.zh-TW.md) (v12); P0 is in progress and
nothing later has started.

The rules behind it are not repeated here:

| Topic | Where |
| --- | --- |
| Rule priority, hard limits, change risk, commits, pull requests, writing | [AGENTS.md](../AGENTS.md) |
| CI tiers | [.github/AGENTS.md](../.github/AGENTS.md) |
| How to review and report | [REVIEW.md](../REVIEW.md) |
| Gate commands, numbers, exceptions | [REQUIREMENTS.md](../REQUIREMENTS.md) |

Items the plan does not decide are marked **open** and listed in
[Open items](#open-items).

## Branches

| Branch | Use | Who merges |
| --- | --- | --- |
| `main` | Accepts pull requests only. | The owner |
| `p<phase>/<topic>` | One logical change in one phase, opened from `main`, for example `p0/rebrand`, `p0/rules-kit`, `p0/ci`, `p0/docs`. | The owner, through a pull request |

- Each branch gets its own worktree; the main checkout never switches branches.
  Two agents never work on overlapping files at the same time.
- Pull requests so far were merged with merge commits; the repository also
  allows squash and rebase. Branches are deleted after merging.
- A phase starts only after the previous phase's branches are merged (plan
  section 六).
- No direct commit to `main`.

## Phases

The plan's phases, with the branches they use and what proves each done. The
branch prefix is the phase; topics are named when the phase starts. P1 and later
branch names follow the P0 pattern; confirming that pattern is open.

Every branch passes the merge gate in `REQUIREMENTS.md`, "Commands", before it
merges. The acceptance below is what a phase adds on top. Where a phase needs a
command that does not exist yet, the branch that builds the check adds it to
`REQUIREMENTS.md` in the same pull request.

| Phase | Branches |
| --- | --- |
| P0 Preparation | `p0/rebrand`, `p0/rules-kit` (merged); `p0/ci`, `p0/docs`, `p0/openapi` |
| P1 `/v1` reading endpoints and reading page | `p1/<topic>` |
| P1b mermaid and mind map | `p1b/<topic>` |
| P2 Edits and versions | `p2/<topic>` |
| P2b Workspaces | `p2b/<topic>` |
| P3 gzpaste in Rust | `p3/<topic>` |

Acceptance, from plan section 六:

- P0 Preparation:
  - The renamed executable, before any feature change, behaves like upstream against a copy of the live database.
  - The live configuration is inventoried, and the signing key and salt carry over.
  - The shared theme repository exists.
  - These three documents and the `REQUIREMENTS.md` commands and numbers exist.
  - The OpenAPI draft is reviewed.
  - Astryx 0.7.0 provides the reader components. Checked on 2026-11-14 by installing 0.7.0 in a scratch directory: Markdown, Outline, CodeBlock, Table and TabList are present; the gaps (syntax colouring languages, sticky table header, drawer) are in [design-language.md](design-language.md).
- P1 `/v1` reading endpoints and reading page:
  - Regenerating the contract shows no difference.
  - A burn paste is not consumed before the confirm press.
  - A wrong password returns 401 with the error JSON.
  - A Telegram preview shows title and summary.
  - With JavaScript off, plain text is readable.
  - At 390 px every table is a table; 320 to 1280 px have no horizontal page scroll.
  - Cold-load transfer, time to first readable text and scrolling a 1,600-line document are inside the `REQUIREMENTS.md` budgets.
  - Anonymous `curl` `POST /` and `/raw` still work.
  - Every endpoint row has a contract test.
  - The owner approves the first reading screen before the rest is built.
- P1b mermaid and mind map:
  - At 390 px the mind map is a vertical collapsible tree that needs no zoom.
  - A node jumps to its section.
  - Neither library downloads when unused.
  - The owner approves the side-by-side screen first.
- P2 Edits and versions:
  - Every branch of the edit rules (plan section 四) has a test: 401, 403 including a cookie without `Origin`, 409, 412, resending identical content, concurrent transactions, the history cap, history gone after delete, 429.
- P2b Workspaces:
  - An agent pushes the same entry three times: one link, three versions.
  - One `/w/<slug>` link shows everything.
  - Encrypted pastes show no title in the list.
  - Deleting a workspace keeps its pastes.
  - The owner approves the first workspace screen.
- P3 gzpaste in Rust:
  - An agent runs `put -w` three times: one link, three versions.
  - Every exit code in plan section 五 has a test.
  - Without a terminal, a 412 neither prompts nor overwrites.
  - Old flags and new subcommands give the same test results.

The P0 acceptance command for "behaves like upstream against a copy of the live
database" is open.

## Review routing

| What | Reviewer | Notes |
| --- | --- | --- |
| Plans and API design (the OpenAPI draft, the edit rules, the workspace model) | GPT-6 astra | The owner's ruling of 2026-11-14. Before coding starts. |
| Pull request diffs | DeepSeek-flash, run through omp | The owner's ruling of 2026-11-14. Follows `REVIEW.md`. For a High-risk change the reviewer must also be a different model family from the writer (`AGENTS.md`, "Model usage"); when the writer is DeepSeek, the orchestrator picks another family. |
| English prose in documents and pull requests | DeepSeek-flash, run through omp | The same ruling. Against `AGENTS.md`, "Writing". |
| Screens | The owner | A visible UI change waits for screenshots or a preview (`AGENTS.md`, "Merging"). |
| Merge | The owner | Agents push branches and open pull requests; they never merge or push to `main`. |

## Deployment and rollback

The runbook for the P1 and P2 releases, from plan section 六. No step runs on
the server without the owner's explicit go (`AGENTS.md`, hard limits). The
exact commands are written into the release pull request and checked there.

Layout on the host, by FHS role:

| Path | Holds |
| --- | --- |
| `/opt/zpaste/compose.yml` | The compose file. The image is pinned by digest. |
| `/etc/zpaste/env` | `WASTEBIN_SIGNING_KEY` and `WASTEBIN_PASSWORD_SALT`, root 600, moved from the current setup with their values unchanged. |
| `/var/lib/zpaste/state.db` | The database, owned by uid 10001. Backups cover `/var/lib/zpaste`. |
| `/opt/wastebin` | The old service, left untouched until Zpaste is stable. |

Before the release:

1. The image for the release is built by CI and pushed to
   `ghcr.io/zakkaus/zpaste`; note its digest.
2. The migration and the full test suite have passed on a copy of the live
   database.
3. The signing key and salt are read from the current `.env` (root 600, with
   `sudo`) and written to `/etc/zpaste/env` unchanged. If no salt was set, the
   upstream default `somesalt` is what the old service used, and it stays.

Release steps:

1. Stop every write on the old service, including burn-after-reading reads,
   which delete data. Back up `/opt/wastebin/data/state.db`.
2. Copy the database to `/var/lib/zpaste/`, keeping uid 10001. Run the
   migration and the full test suite on a second copy.
3. Start the new image's digest from `/opt/zpaste/compose.yml`, with the same
   signing key, salt, Traefik labels and rate limit.
4. Spot-check: an old paste, an encrypted paste, deleting as owner, creating
   with gzpaste.
5. On failure, stop Zpaste. For P1, start the old service from the untouched
   `/opt/wastebin`. After P2, restore the backup, then switch back to the
   previous digest.
6. Delete `/opt/wastebin` only after a period of stable operation; the length of
   that period is open.

An older executable cannot open a migrated database (rusqlite_migration answers
`DatabaseTooFarAhead`), so a rollback after a migration always restores the
backup first. Every post, edit, delete and burn between the backup and the
rollback is lost; say so when announcing a rollback.

Host rules: firewall changes on the host edit the table and never run
`nft flush ruleset`, which also clears Docker's NAT. Desktop development and
tests use rootless Docker only.

## Documents change with the phase

A pull request that changes behaviour updates the documents that describe it
(`AGENTS.md`, "Documentation"). A phase's last pull request also moves every
section it built from "Target" to "Current".

| Document | Path | Authority |
| --- | --- | --- |
| Working plan | [docs/plan.zh-TW.md](plan.zh-TW.md) | Decisions, phases and acceptance |
| Architecture | [docs/architecture.md](architecture.md) | Structure and invariants, derived from the plan |
| Design language | [docs/design-language.md](design-language.md) | Interface values and visual rules, derived from the plan |
| Workflow | this file | This repository's workflow values |
| README | `README.md`, `README.zh-TW.md`, `README.zh-CN.md` | English is authoritative; the translations follow it |

No check yet confirms that the files these documents name exist or that the
"Current" labels are true; which gate does that is open.

## Open items

| # | Item | Blocks |
| --- | --- | --- |
| W1 | Confirm the `p<phase>/<topic>` branch pattern for P1 and later; the plan says each phase works on its own branch, and P0 used one branch per task. | P1 |
| W2 | The command that proves P0's "behaves like upstream against a copy of the live database". | P0 |
| W3 | Where the open-questions list and the deferred-options list from `AGENTS.md` live in this repository. | None; asked with the next batch |
| W4 | How long Zpaste must run stable before `/opt/wastebin` is deleted. This is the only place the question is tracked. | P1 release |
| W5 | A gate that checks these documents: named files exist, "Current" sections are true. | P1 |
