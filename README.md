# itcmdr

A monorepo for itcmdr — an IT maintenance system with three parts:

- A: PowerShell agent (Windows) that runs nightly checks and optional repairs
- B: Rust gRPC server (`itcmdr-server`)
- C: Flutter desktop/web app (`itcmdr`)

This repository uses neutral placeholders and example ranges (RFC 5737) only. Site-specific values are loaded from a local, gitignored `config/site.json` or from `C:\\ProgramData\\itcmdr\\site.json` on Windows hosts.

See `docs/ARCHITECTURE.md` and `docs/INSTALL.md`.
