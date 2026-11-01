# itcmdr Admin

The adminsvr app for owners, admins and SREs (Flutter). The Rust bridge in `rust/` wraps
`itcmdr-admin-cli`'s signed client (device key, pinned TLS, passkey step-up); regenerate the Dart
bindings with `flutter_rust_bridge_codegen generate` after changing `rust/src/api`.

    flutter test                      # widget tests (fake backend)
    (cd rust && cargo test)           # the bridge end to end against an in-process adminsvr
    flutter build linux|macos|windows|apk --release

See docs/ADMINSVR.md ("itcmdr Admin app", "Push notifications").
