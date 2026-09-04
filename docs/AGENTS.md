# AGENTS.md

Read this file in full at the start of every session. It is the standalone
orientation and the only agent router for this repository. The conversation is
not durable state; the tree, the record and the running system are.

## 1. Where you are

`bit-ids` builds an automatically maintained catalogue of identities observed
from exact stable BitTorrent client and library builds. The product is the
measurement and its raw evidence, not an identity registry copied from source.

The current state and work order live in
[`../TODO/PROGRESS.md`](../TODO/PROGRESS.md). Every entry and status lives in
[`../TODO/INDEX.md`](../TODO/INDEX.md). The invariant rules live in
[`../TODO/RULES.md`](../TODO/RULES.md).

## 2. Absolutes

1. A published value is observed from a running client. Source code, client-ID
   tables, release notes, UI labels, self-reports and public swarm statistics
   cannot populate it.
2. Every profile uses the Rust active observer plus at least one independent
   connector. Their overlapping observations agree or the profile stays
   unpublished with the disagreement recorded.
3. Raw evidence and capture conditions ship with every profile. A parsed value
   with no recoverable bytes is not a measurement.
4. Two distinct acquisition routes must resolve the same stable version for
   the same target and host. Version equality is checked after installation,
   not trusted from filenames or package metadata.
5. Only the newest stable release is targeted. Do not backfill old versions.
   When stable moves, append a new record and retain the old one.
6. Published records and the `data` branch are append-only. Never force-push,
   edit or delete published data. Correct by supersession.
7. Work on `main`, never a `claude/*` branch. If a clone is shallow, run
   `git fetch --unshallow`, then check `git rev-list --count HEAD..origin/main`
   before work. Reconcile a non-zero answer before editing.
8. Prefer authenticated `gh` for GitHub reads. For a read-only GitHub REST
   path not handled by `gh`, use
   `https://api.gh.pkgforge.dev/<GH_API_PATH>`. GraphQL and authenticated-only
   routes are the stated exceptions.
9. Fetch an ordinary web source directly first. If it returns 401/403 or is
   otherwise unreachable, use
   `https://api.rv.pkgforge.dev/<ORIGINAL_URL>` and record which route answered.
10. This repository's own remote is the only remote that may be written. Every
    other repository is read-only. Never open an issue, pull request, comment,
    discussion, fork, review or star elsewhere.
11. Commit and push each coherent, green unit as work proceeds so completed
    work does not live only on one machine. Never credit an agent, model or
    tool as author, co-author or generator. The configured operator identity is
    the only commit identity.
12. A secret never enters the tree, logs, artifacts, profiles, records or
    commit messages. Never ask the operator to paste a secret value.
13. Read exit codes from the process that produced them, unpiped. A pipeline's
    status is not the check's status.
14. Update the entry, index, progress record and affected docs in the same
    change as the work.
15. Do not run the session-end protocol until the operator says to wrap up or
    the current work order is genuinely exhausted. Genuinely exhausted means
    at least five L-sized entries, or equivalent effort, have been completed or
    driven to measured external blockers. Never end early merely because work
    is difficult, the response is long, or budget remains available to return.

## 3. Start of session

Run these in order.

1. Read `TODO/PROGRESS.md`, then `TODO/RULES.md` and `TODO/INDEX.md`.
2. Raise every pending operator decision at the start, with a recommendation.
   If the operator does not answer, work unattended on non-blocked items. Do
   not interrupt the middle of a session with a question that can wait; keep it
   in `PROGRESS.md` and ask again only when the operator initiates or ends a
   session.
3. Inspect `git status`, the current branch, remote, authentication, shallow
   state and `HEAD..origin/main`. Never assume the identity or clone state from
   a prior session.
4. Run the doctor and re-measure the local gate.
5. Rewrite `docs/history/RESUME.md` before editing and refresh it whenever the
   in-flight item changes.
6. Follow the routing table below and restate the immediate plan.

## 4. Routing table

| task | read in full |
| --- | --- |
| any open item | its category file from `TODO/INDEX.md`, `docs/methodology/work-todo.md`, `docs/methodology/gate.md` |
| data model or validation | `docs/architecture.md`, `docs/capture-methodology.md`, `TODO/schema.md`, `TODO/corpus.md` |
| observer or connector | `docs/architecture.md`, `docs/capture-methodology.md`, `TODO/observer.md`, the bit-cli reference sweep |
| client acquisition/capture | `docs/client-matrix.md`, `docs/capture-host.md`, `TODO/acquisition.md`, `TODO/clients.md`, `SECURITY.md` |
| library-backed targets | `docs/client-matrix.md`, `TODO/engines.md`, `TODO/acquisition.md` |
| publishing or CI | `docs/publishing.md`, `TODO/publishing.md`, `TODO/ci.md`, `docs/security/remote-ops.md` |
| reference research | `docs/methodology/references.md`, `docs/methodology/experiments.md`, existing `docs/reference-sweeps/` files |
| documentation | `docs/conventions/docs.md`, `docs/conventions/prose.md`, `TODO/documentation.md` |
| shell or cross-platform work | `docs/conventions/shell.md`, `scripts/README.md`, `docs/agent-tooling.md` |
| security or remote action | `SECURITY.md`, `docs/security/secrets.md`, `docs/security/remote-ops.md` |
| session wrap/resume | `docs/methodology/sessions.md`, `docs/methodology/reviews.md`, `docs/methodology/history.md` |

Required reading is read end to end. Before acting, report each required file's
line count and last heading as the receipt described in
[`methodology/sessions.md`](methodology/sessions.md).

## 5. Working unattended

- Take the work order in `TODO/PROGRESS.md` in sequence. Foundations precede
  client breadth and polish.
- A blocker closes a route, not the question. Consider and record at least
  three safe routes before calling an item blocked, then pivot to the next
  unblocked item.
- A capture or install that could alter a non-disposable host is refused. The
  two independent disposable-runner guards in `ACQ-04` must exist first.
- Do not redistribute a client binary. Keep URLs, signatures, package
  metadata and digests only.
- Shell is the default orchestration language. Core parsers, validators,
  capture tooling, publishing logic and the consumer library are Rust. Python
  is used only where a documented constraint makes both unsuitable.
- Main docs describe current truth. Amend stale text in place. Put superseded
  reasoning, corrections, reviews and narrative history under `docs/history/`.
  Never append diary or changelog material to reference pages.

## 6. The gate

A unit closes only after all three parts pass:

1. the automated local/CI suite;
2. a driven run of the real path the entry changes;
3. at least three deep reviews with different lenses.

Use the exact acceptance command in the entry. Mutation-prove critical guards
by planting the defect in disposable state and confirming the check refuses
it, then restoring and confirming the clean case.

## 7. Session end protocol

Run this only under rule 15 in section 2.

1. Finish the last task in flight or checkpoint it coherently. Save current
   progress and leave no unexplained half-edit.
2. Update every stale document, entry, index count, progress field and route.
   Amend current truth in place; move narrative history to `docs/history/`.
3. Run the full gate and drive the last changed path.
4. Perform at least three deep reviews: a dependency/door sweep, a guard
   mutation pass, and a claim audit. Fix findings or file them with acceptance
   commands.
5. Commit and push the coherent state using the operator's identity. Confirm
   the remote branch and checks, and leave the local tree clean.
6. Print the evidence-backed summary table in chat.
7. Print the next kickoff prompt in a fenced code block. The prompt points to
   this router and the record; it does not duplicate the work order.

Never claim a check, push, release or capture happened until its durable state
has been read back.
