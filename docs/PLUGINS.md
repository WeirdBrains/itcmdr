# Plugins

Checks and repairs come as signed plugin packages. The built-in checks are plugins too (pinned in
`plugins.pin.json`, fetched by `scripts/fetch-plugins.sh`). A plugin is PowerShell (run by
`itcmdr-plugin-ps`) or a Rust binary (the `itcmdr-plugin` crate), and speaks the plugin protocol on
stdio with a per-launch secret ([log standard](LOG-STANDARD.md) has the protocol, the log format and
the error contract).

## On the site server

    itcmdr-cli plugins install --path disk-watch-1.2.0.tar.gz   # or --url; --disabled to install off
    itcmdr-cli plugins list
    itcmdr-cli plugins enable disk-watch[@1.2.0] | disable disk-watch | remove disk-watch[@1.2.0]
    itcmdr-cli plugins run disk-watch                           # Verify, then Execute
    itcmdr-cli plugins runs
    itcmdr-cli plugins quarantine                               # refused packages
    itcmdr-cli plugins pack --dir ./disk-watch --key owner.key --out disk-watch-1.2.0.tar.gz

- Only owner-signed packages install; a bad signature or checksum is quarantined
  (`quarantine/index.jsonl`) and the install is refused.
- `plugins.enable_on_install` (default true) enables a valid package on install.

## On the agents

The server pushes the plugin selection `agent.plugins` (endorsed by the root key) with file checksums;
the agent checks them and refuses tampered copies (alert, then resolved when fixed).

    itcmdr-cli agent run WS-01 <plugin>     # report-only, now
    itcmdr-cli agent runs                   # reports, with parity against the nightly
    itcmdr-cli agent plugins                # what each agent applied

The site app's Plugins page and, from adminsvr, `itcmdr-admin plugin ...` (with the organization's
approval rules) do the same.

Plugin environment (set by the agent or server for each run): `ITCMDR_PLUGIN_SECRET`,
`ITCMDR_LOG_PATH`, `ITCMDR_RUN_ID`, `ITCMDR_MACHINE`, `ITCMDR_REPORT_ONLY`, `ITCMDR_ARTIFACT_DIR`,
`ITCMDR_PS_MODULE` ([environment](ENVIRONMENT.md)). A plugin has `ITCMDR_PLUGIN_START_SECS`
(default 60) to answer the handshake.
