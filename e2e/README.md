# Windows VM end-to-end checks

Scripts in this folder run on a disposable Windows test bed only. They make real
breaks (kill services, bad enroll tokens) and check self-heal. Do not run them on
a production office server.

| Script | Purpose |
|--------|---------|
| `v137.ps1` | Upgrade smoke: FromVer → ToVer, build stamps, `failureflag` 1 after upgrade, real self-heal (server-watchdog-only kill must be Running within 90 s by default), already-enrolled `/EnrollToken=` and a bad-token exit 9. |

Paths, versions, the tag commit and the help hash are parameters. Hostnames, share
names and other machine details stay in `HANDOVER.local.md` (gitignored), not here.

See also docs/UPGRADES.md (watchdogs and this smoke) and
`scripts/Invoke-ItcmdrFaultSuite.ps1` (generic fault injection).
