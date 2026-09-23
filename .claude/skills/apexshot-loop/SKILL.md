---
name: apexshot-loop
description: "ApexShot working agreement. Use on EVERY task in this repo: read AGENTS.md and CONTRIBUTING.md first, state the loop state (investigate / branch / verify / publish), and follow its verify, commit, and no-CI-watching rules."
---

# ApexShot loop

This skill exists because sessions have drifted: behaviour-changing edits
landed directly on `main`, and sessions parked ~30 min running full suites
then watching slow CI. The fix is to load the agreement every time.

## Every conversation, before anything else

1. Read `AGENTS.md` and `CONTRIBUTING.md` in the repo root.
2. Say you did, with a link: `Per [AGENTS.md](AGENTS.md) — ...`.
3. State which loop state you are in: **step 1 investigate** / **step 2 branch** /
   **step 3 verify** / **step 4 publish** / **step 5 after merge**.
4. Do not skip step 1. Diagnosis and fix are separate steps; the maintainer
   decides whether a fix proceeds.

## The rules that keep getting broken

- **No code edits during step 1.** Restate the suspected issue, reproduce or
  trace to `path/file.rs:123` with command + output, end with a verdict
  (**confirmed** / **partly confirmed** / **not reproducible** /
  **already fixed / by design**). Only a confirmed behaviour-changing issue
  earns a branch.
- **Step 2 path:** docs/typo/metadata-only small edits with no behaviour risk
  go straight to `main`; behaviour changes, multi-file fixes, features,
  refactors, packaging, and CI changes go on a branch + PR from `origin/main`.
- **Step 3 verify:** test narrow by default
  (`cargo test --lib <module>`, `./scripts/test.sh --test NAME`). Full
  `cargo test --jobs 2 -- --test-threads=1` only for shared/test infra or a
  merge-ready PR — CI already runs the full suite.
- **Step 4 publish:** after pushing a branch, report the PR URL and which CI
  jobs will run, then STOP. Never `gh pr checks --watch` or poll CI in a loop.
  Red CI becomes a follow-up task, not a wait.
- **Never launch, drive, or script the app.** Manual checks are the
  maintainer's; list them under "Not verified".
- Never merge your own PR. No `reset --hard`, `clean`, force-push, or branch
  deletion unless the maintainer asks.

## Traps (see AGENTS.md for the full list)

- `flameshot/`, `obs-studio/`, `spectacle/` in the repo root are read-only
  reference checkouts, not app components.
- GTK tests use `crate::test_support::with_gtk`, one init per process;
  CI runs `--test-threads=1`.
- Every `t(...)` string must exist in all eight `po/*.po` catalogs.
- Do not create a worktree unless two checkouts must be live at once; prefer
  `git switch -c` in the current checkout to reuse the warm `target/`.
