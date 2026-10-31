# History rewrite of 2026-10-31

The plugin sources (`plugins/`) moved to github.com/WeirdBrains/itcmdr-plugins in v0.14.1. With the owner's approval, the
itcmdr history was then rewritten once (`git filter-repo --invert-paths --path plugins/`) so that no reachable commit
contains them. This was a one-time exception to the no-force-push and no-tag-move rules.

- Commits before v0.11.0 did not contain `plugins/` and kept their hashes.
- Every annotated tag was recreated on its rewritten commit with the original message, tagger and date.
- Commit messages that mention old hashes were rewritten by filter-repo. Docs in the tree were updated by a follow-up commit.
- GitHub may keep the old commits reachable by SHA (and through the read-only `refs/pull/1/head`) until it
  garbage-collects them. The old history only differs by the plugin files, which now live in the itcmdr-plugins repo.
- Clones made before the rewrite: `git fetch --force --tags origin && git reset --hard origin/main`. Don't merge old history back in.

## Tags (old -> new commit)

| Tag | Old | New |
|---|---|---|
| v0.1.0 | `c90fa4b` | `c90fa4b` |
| v0.1.1 | `adab590` | `adab590` |
| v0.2.0 | `6f3b76b` | `6f3b76b` |
| v0.3.0 | `db03433` | `db03433` |
| v0.4.0 | `9f9c33f` | `9f9c33f` |
| v0.5.0 | `ed548c4` | `ed548c4` |
| v0.5.1 | `0402dd2` | `0402dd2` |
| v0.5.2 | `b098ff6` | `b098ff6` |
| v0.5.3 | `568b954` | `568b954` |
| v0.6.0 | `0ce70c0` | `0ce70c0` |
| v0.6.1 | `a4b0eb6` | `a4b0eb6` |
| v0.6.2 | `19c297c` | `19c297c` |
| v0.7.0 | `4afe6c7` | `4afe6c7` |
| v0.8.0 | `25e232f` | `25e232f` |
| v0.8.1 | `c1b4388` | `c1b4388` |
| v0.9.0 | `073671d` | `073671d` |
| v0.9.1 | `4e66a71` | `4e66a71` |
| v0.10.0 | `faa2ae6` | `faa2ae6` |
| v0.10.1 | `2366819` | `2366819` |
| v0.11.0 | `b249612` | `1827b06` |
| v0.12.0 | `fb007b2` | `d79a031` |
| v0.13.0 | `fba79ce` | `38f6f7e` |
| v0.14.0 | `aa55782` | `fd28fa1` |
| v0.14.1 | `8b311e7` | `41cf69f` |

## Rewritten commits (old -> new)

| Old | New | Date and subject |
|---|---|---|
| `46bb345` | `0631198` | 2026-10-31 wip: v0.12.0 progress (handover) |
| `8b311e7` | `41cf69f` | 2026-10-31 release: v0.14.1 |
| `9484fb5` | `5881a22` | 2026-10-31 v0.12.0 follow-up: Dart e2e runs on Windows (ITCMDR_E2E_BIN_DIR, .exe), VM results (kept-both path, app smoke), docs |
| `aa55782` | `fd28fa1` | 2026-10-31 release: v0.14.0 |
| `b249612` | `1827b06` | 2026-10-30 v0.11.0: built-in checks as plugins (parallel report-only, parity), default nightly/backup schedules, gated cutover, rollback, local fallback |
| `fb007b2` | `d79a031` | 2026-10-31 v0.12.0: the app signs requests and ships keys, schedules, backup, and settings screens. |
| `fba79ce` | `38f6f7e` | 2026-10-31 release: v0.13.0 |
