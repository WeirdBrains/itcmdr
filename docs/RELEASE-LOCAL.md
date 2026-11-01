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

Clone the tag on the Mac and run `scripts/release-local.sh unix`, `android` and `ios-check` with a Flutter that
matches `app/pubspec.yaml`. Produces the CLI, admin CLI and adminsvr tarballs and the app and admin app zips
(unsigned, not notarized until the Mac signing step), plus the admin app APK. Copy the files into `dist/X.Y.Z/`
on the dev box.

No CocoaPods (v0.20.0): iOS and macOS use Flutter's Swift Package Manager integration and the Rust bridges are
built by flutter_rust_bridge native assets (`hook/build.dart`, toolchain pinned in `rust/rust-toolchain.toml`,
installed by rustup). Once per user: `flutter config --enable-swift-package-manager` (the script turns it on if
off). The script refuses to build when a Podfile or Pods folder exists. All plugins in both apps support this
(the only native code is the bridge; `path_provider` and `objective_c` need no pods).

## Windows VM step

1. `git archive --format=zip -o <share>/src-rX.zip vX.Y.Z` plus `third_party/` (as `sync.sh` does).
2. On the VM: expand it, `cargo build --release --locked -p itcmdr-server -p itcmdr-agent --bins`, `flutter build windows --release`,
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
