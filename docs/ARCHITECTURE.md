## itcmdr architecture (overview)

Components:
- Server (`itcmdr-server`, Rust + tonic): gRPC/JSON ingest, SQLite, metrics
- Agent (PowerShell): nightly checks/repairs, JSONL logs, idempotent
- App (Flutter): desktop/web, connects to server via gRPC/gRPC-Web

Site-specific values are loaded from `config/site.json` or `C:\ProgramData\itcmdr\site.json`. Placeholders in this repo use RFC 5737 doc ranges (192.0.2.x, 198.51.100.x, 203.0.113.x) and neutral hostnames like `SERVER01`, `WS-01`, `RCU-01`, `PRN-01`.

On-demand runs: agents run a lightweight poller scheduled task that fetches pending requests from the server or a shared location; requests are audit-logged and shown in the app with status (queued, running, done). This avoids dependency on remote PowerShell (WinRM).


## Regenerating Dart gRPC code

`protoc -I proto --dart_out=grpc:app/lib/proto proto/itcmdr.proto` using protoc_plugin 25.1.0 (`dart pub global activate protoc_plugin 25.1.0`), matching protobuf ^6.1 / grpc ^5.2 in app/pubspec.lock.
