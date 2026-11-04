# The website

The public site (itcmdr.weirdbrains.com) is static HTML on the `site` branch of this repository,
served by GitHub Pages (`CNAME`, `.nojekyll`). It has no build dependencies.

- Work in a separate worktree: `git worktree add ../itcmdr-site site`.
- Pages come from `build.py` (home, download, docs, security): edit it, then `python3 build.py`.
- Download links point at the release assets; update the version and links after each release.
- Before pushing, run the leak scans (`python3 scripts/scan_repo.py`) and check every link.
- Push the branch: `git push origin site`. Pages serves the branch; HTTPS is enforced.
- DNS: a CNAME for the domain pointing at the GitHub Pages host; the certificate is issued by GitHub.
- Drafts (posts) live in `drafts/`, untracked, and are never published without review.
