# Man pages

`man/` holds one page per program (`itcmdr-cli.1`, `itcmdr-admin.1`, `itcmdr-adminsvr.1`,
`itcmdr-agent.1`, both watchdogs, `itcmdr-server.1`, `itcmdr-restore.1`, `itcmdr-plugin-ps.1`) and
`itcmdr.7` with every environment variable. They list every command, subcommand and flag.

They are generated, never edited by hand:

    cargo build --workspace --bins
    python3 scripts/gen-man.py            # writes man/
    python3 scripts/gen-man.py --check    # exit 1 when man/ is out of date (the test suite runs it)

The flags come from each program's `--help`; the ENVIRONMENT sections come from
[environment](ENVIRONMENT.md). Read one without installing: `man ./man/itcmdr-cli.1`.
