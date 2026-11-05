# Local release builds

Used while GitHub Actions is unavailable (v0.15.0 and later). Nothing here uses Actions minutes. The tag is created
and pushed first by the normal release procedure (docs/HANDOVER.md); these steps build from that exact tag and never
move or recreate it.

## Release guard: everything from the tag (v1.3.1)

Every release asset must be built from the exact tagged commit, so the built-in help always matches the docs of
that release.

- Each binary carries a build stamp `ITCMDR-BUILD v=<version> commit=<git commit> help=<hash> END` (from
  `help/build.rs`): the server, `itcmdr-cli`, `itcmdr-admin`, adminsvr, agent and watchdogs, and the Rust bridge
  of both apps. The help hash is FNV-1a 64 over the embedded docs; the apps' `help_topics.g.dart` carries the
  same `helpHash`. The commit comes from `ITCMDR_BUILD_COMMIT`, or else `git rev-parse HEAD`.
- `release-local.sh unix` and `android` refuse to build unless HEAD is the tag `vX.Y.Z`
  (`ITCMDR_ALLOW_UNTAGGED=1` for a trial build, which `check` then refuses), and set `ITCMDR_BUILD_COMMIT`.
- The Windows build (`scripts/release-windows.ps1 -Zip src.zip -Ver X.Y.Z -Commit <tag commit>`) checks every
  binary's stamp before Inno Setup and writes `windows-stamps.txt`; copy it to `dist/X.Y.Z.windows-stamps.txt`
  (the installers are compressed, so `check` uses those lines for them).
- Since 1.3.2 no Windows install needs the Visual C++ Redistributable: the Rust binaries link the C runtime
  statically (`.cargo/config.toml`, `+crt-static`), and the Windows build copies the runtime DLLs from the
  Build Tools' `VC\Redist\MSVC\<ver>\x64\Microsoft.VC143.CRT` next to both Flutter apps. It refuses a Rust
  `.exe` whose import table (`dumpbin /dependents`) still lists `vcruntime*`/`msvcp*` (`GUARD: ... imports the
  VC++ runtime`); a plain text search is not enough, since the CLIs embed these docs.
- `release-local.sh check` (and again `publish`) runs `scripts/check_stamps.py`: every asset needs a stamp, and
  every stamp must name the tag's commit and the help hash of the tag's docs, else `GUARD: ...` and no release.
  Tests: `python3 scripts/test_check_stamps.py` (also run by `cargo test -p itcmdr-help`).

So the order is: commit, annotated tag (locally), build every platform from the tag, `check`, push with
`--follow-tags`, `publish`.

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

1. `git archive --format=zip -o <share>/src-vX.Y.Z.zip vX.Y.Z` plus `third_party/`.
   Also `git bundle create <share>/itcmdr-vX.Y.Z.bundle vX.Y.Z`: the script then builds from a git checkout of
   the tag (with `core.autocrlf=false`, so the embedded help keeps the docs' line endings) because the Flutter
   native-asset hooks do not pass `ITCMDR_BUILD_COMMIT` on; without a checkout the app bridges get `commit=unknown`
   and the guard stops the build (found while building 1.3.1).
2. On the VM: `scripts/release-windows.ps1 -Zip src-vX.Y.Z.zip -Ver X.Y.Z -Commit $(git rev-parse vX.Y.Z^{commit})`
   (cargo, both Flutter apps, stamp check, the 7 Inno Setup installers, then the build caches are removed).
3. Copy the `*.exe` into `dist/X.Y.Z/` and `windows-stamps.txt` to `dist/X.Y.Z.windows-stamps.txt`.
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

## Admin app desktop builds (v0.22.0)

- macOS: `release-local.sh unix` on the Mac builds `itcmdr-admin-app-<v>-macos-aarch64.zip` with
  Swift Package Manager (`flutter config --enable-swift-package-manager`; there is no Podfile and
  the script refuses to call `pod`). Unsigned and not notarized.
- Windows: built on the Windows VM (`flutter build windows` in `admin-app`, then Inno Setup) as
  `itcmdr-admin-app-Setup-<v>.exe`. Unsigned.
- Linux tarball and Android APK (debug-signed) come from the Linux and Mac runs.

## Build caches (v1.2.1)
Every build step (`unix`, `android`, `ios-check`) removes this checkout's build caches and targets when it ends, also on failure: the Rust `target` dirs (and `CARGO_TARGET_DIR` when its name starts with `itcmdr`), Flutter `build/` and native-asset builds (`.dart_tool/hooks_runner`), Gradle output and, on macOS, the Xcode DerivedData of this checkout. `dist/`, sources and logs stay. `ITCMDR_KEEP_BUILD=1` skips it to chain steps; then run `scripts/release-local.sh clean` last. The Windows VM build removes its `target` and Flutter build dirs after copying the installers.
