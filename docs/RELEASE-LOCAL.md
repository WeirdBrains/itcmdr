# Local release builds

Used while GitHub Actions is unavailable (v0.15.0 and later). Nothing here uses Actions minutes. The tag is created
and pushed first by the normal release procedure (docs/HANDOVER.md); these steps build from that exact tag and never
move or recreate it.

## Linux step (dev box)

    git checkout vX.Y.Z            # or stay on main at the tagged commit
    NINJA_DIR=<dir with ninja> scripts/release-local.sh unix

Produces `dist/X.Y.Z/itcmdr-cli-X.Y.Z-linux-x86_64.tar.gz` and `itcmdr-app-X.Y.Z-linux-x86_64.tar.gz`. The Linux
app build needs clang, cmake, pkg-config, GTK 3 headers and ninja; a user-local ninja binary is enough.

## macOS step (Mac)

Clone the tag on the Mac and run `scripts/release-local.sh unix` with a Flutter that matches `app/pubspec.yaml`.
Produces `itcmdr-cli-X.Y.Z-macos-aarch64.tar.gz`. The macOS app archive is produced once the app has a `macos/`
desktop project (not yet; it is unsigned and not notarized until the Mac signing step). Copy the files into
`dist/X.Y.Z/` on the dev box.

## Windows VM step

1. `git archive --format=zip -o <share>/src-rX.zip vX.Y.Z` plus `third_party/` (as `sync.sh` does).
2. On the VM: expand it, `cargo build --release --locked -p itcmdr-server --bins`, `flutter build windows --release`,
   then `ISCC itcmdr-{server,agent,cli,app}.iss`.
3. Copy `installer\out\*.exe` back into `dist/X.Y.Z/`.
4. Scan the uncompressed payload (release exes, the Flutter runner folder, agent and installer files) for the
   private leak patterns; the installers themselves are compressed.

Signing: pass `/DSign` with signtool and the code-signing certificate when one exists (see release.yml). Until then
the installers are unsigned and the release notes say so.

## Check and publish (dev box)

    ITCMDR_LEAK_PATTERNS=<private extra patterns file> scripts/release-local.sh check
    scripts/release-local.sh publish

`check` unpacks every artifact, greps it for `config/banned.local.txt` plus the extra patterns (VM and machine
host names), refuses on any hit, then writes `SHA256SUMS`. `publish` needs the tag on origin and creates the GitHub
release with the CHANGELOG section as notes (`--verify-tag`), or uploads to an existing release.
