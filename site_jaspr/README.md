# itcmdr website (Jaspr)

Static marketing site for [itcmdr.weirdbrains.com](https://itcmdr.weirdbrains.com),
built with [Jaspr](https://jaspr.site) in **static** mode.

## Layout

- **Source (this folder):** checked into `main` as `site_jaspr/`.
- **Published output:** committed to the `site` branch at the repo root
  (flat `*.html` URLs, `assets/`, `demos/`, `CNAME`, `.nojekyll`) for GitHub Pages.

Build on a scratch directory under `/mnt/ext1` (never inside `~/Projects`).

```bash
export PATH="$HOME/.pub-cache/bin:$HOME/flutter/bin:$PATH"
./tool/sync_web.sh
dart pub get
jaspr build --sitemap-domain https://itcmdr.weirdbrains.com
./tool/flatten_and_stage.sh
# then copy build/site-out/* into the site-branch worktree and push
```

Clear `build/` and `.dart_tool/` when finished.
