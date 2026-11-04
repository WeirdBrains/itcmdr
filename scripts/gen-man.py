#!/usr/bin/env python3
"""Generate man/*.1 from the built binaries' --help (every subcommand, flag and env var)
plus docs/ENVIRONMENT.md (every ITCMDR_* variable).

    python3 scripts/gen-man.py [--bin-dir DIR] [--check]

DIR defaults to $CARGO_TARGET_DIR/debug or target/debug. --check exits 1 when man/ is stale.
Build first: cargo build --workspace --bins.
"""
import argparse, os, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAN = ROOT / "man"
# clap binaries: name -> one-line purpose
CLAP = {
    "itcmdr-cli": "site server administration on the server itself",
    "itcmdr-adminsvr": "offsite admin server (admin API, SiteLink, Let's Encrypt)",
    "itcmdr-admin": "command line client for adminsvr",
    "itcmdr-agent": "Rust agent that runs on every office machine",
    "itcmdr-agent-watchdog": "keeps itcmdr-agent running and rolls back bad upgrades",
    "itcmdr-server-watchdog": "keeps itcmdr-server running and rolls back bad upgrades",
}
# hand-written pages (no clap): name -> (purpose, synopsis lines, description)
MANUAL = {
    "itcmdr-server": ("the site server (gRPC for the agents, HTTPS API for the apps)",
        ["itcmdr-server [--console | --service]", "itcmdr-server init", "itcmdr-server recover-db-key",
         "itcmdr-server hash-token TOKEN"],
        "Runs the site server. With no argument or \\fB--console\\fR it runs in the foreground; "
        "\\fB--service\\fR runs as the Windows service. \\fBinit\\fR and \\fBrecover-db-key\\fR are the "
        "same as in \\fBitcmdr-cli\\fR(1). \\fBhash-token\\fR prints the stored hash of a legacy shared token. "
        "Settings come from server.json (see \\fBITCMDR_SERVER_CONFIG\\fR) and the database; see the settings guide."),
    "itcmdr-restore": ("restore helper started by the server for an app or CLI restore",
        ["itcmdr-restore (started by itcmdr-server; not run by hand)"],
        "Stops the service, restores the staged archive into the data directory, restarts and health-checks "
        "the server, and rolls back on failure. Use \\fBitcmdr-cli restore-stage\\fR to start it."),
    "itcmdr-plugin-ps": ("host for PowerShell check plugins",
        ["itcmdr-plugin-ps --script MAIN.ps1"],
        "Runs a PowerShell plugin's main script for the agent and speaks the plugin protocol on stdio. "
        "Started by \\fBitcmdr-agent\\fR(1); see the plugins guide."),
}

def esc(s):
    s = s.replace("\\", "\\\\").replace("-", "\\-")
    return ("\\&" + s) if s[:1] in ".'" else s

def helptext(bindir, name, path):
    r = subprocess.run([str(bindir / name), *path, "--help"], capture_output=True, text=True,
                       env={**os.environ, "NO_COLOR": "1", "COLUMNS": "100"}, timeout=30)
    return r.stdout

def commands(text):
    out, on = [], False
    for line in text.splitlines():
        if line.startswith("Commands:"):
            on = True; continue
        if on:
            if not line.startswith("  "):
                break
            m = re.match(r"^  (\S+)", line)
            if m and m.group(1) != "help" and not line.startswith("    "):
                out.append(m.group(1))
    return out

def walk(bindir, name, path=()):
    t = helptext(bindir, name, list(path))
    yield path, t
    for c in commands(t):
        yield from walk(bindir, name, path + (c,))

def env_section(names=None):
    """ENVIRONMENT from docs/ENVIRONMENT.md table rows: | `NAME` | used by | meaning |"""
    rows = []
    for line in (ROOT / "docs/ENVIRONMENT.md").read_text().splitlines():
        m = re.match(r"^\|\s*`([A-Z0-9_]+)`\s*\|\s*([^|]*)\|\s*(.*?)\s*\|\s*$", line)
        if m and (names is None or any(n in m.group(2) for n in names) or "all" in m.group(2)):
            rows.append(m.groups())
    out = [".SH ENVIRONMENT"]
    for n, used, mean in rows:
        out += [".TP", ".B " + n, esc(re.sub(r"`", "", mean)) + (" (" + esc(used.strip()) + ")" if names is None else "")]
    return out

def page(name, sect_purpose, body, envs):
    head = [f'.TH {name.upper().replace("-", "\\-")} 1 "" "itcmdr" "itcmdr manual"', ".SH NAME",
            f"{esc(name)} \\- {esc(sect_purpose)}"]
    tail = env_section(envs) + [".SH SEE ALSO",
        "\\fBitcmdr\\fR(7), docs/README.md (also the in-app help: \\fBitcmdr-cli help\\fR, \\fBitcmdr-admin help\\fR)"]
    return "\n".join(head + body + tail) + "\n"

def clap_page(bindir, name):
    body = []
    for path, text in walk(bindir, name):
        title = " ".join((name,) + path)
        body += [".SH " + (esc(title.upper()) if not path else "COMMAND: " + esc(title)), ".nf"]
        body += [esc(l) for l in text.rstrip().splitlines()]
        body += [".fi"]
    return page(name, CLAP[name], body, [name])

def manual_page(name):
    purpose, syn, desc = MANUAL[name]
    body = [".SH SYNOPSIS", ".nf"] + [esc(s) for s in syn] + [".fi", ".SH DESCRIPTION", desc]
    return page(name, purpose, body, [name])

def overview():
    lines = ['.TH ITCMDR 7 "" "itcmdr" "itcmdr manual"', ".SH NAME", "itcmdr \\- self-healing IT for small offices",
             ".SH DESCRIPTION",
             "itcmdr is a site server, a Rust agent on every machine, a site app, an optional offsite admin server "
             "(adminsvr) with an admin app and CLI. The guides are in docs/ and in the in-app help.",
             ".SH PROGRAMS"]
    for n, p in {**CLAP, **{k: v[0] for k, v in MANUAL.items()}}.items():
        lines += [".TP", f"\\fB{esc(n)}\\fR(1)", esc(p)]
    lines += env_section(None)
    return "\n".join(lines) + "\n"

def main():
    ap = argparse.ArgumentParser()
    td = os.environ.get("CARGO_TARGET_DIR")
    ap.add_argument("--bin-dir", default=str(Path(td) / "debug") if td else str(ROOT / "target/debug"))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    bindir = Path(a.bin_dir)
    pages = {f"{n}.1": clap_page(bindir, n) for n in CLAP}
    pages.update({f"{n}.1": manual_page(n) for n in MANUAL})
    pages["itcmdr.7"] = overview()
    stale = [f for f, s in pages.items() if not (MAN / f).exists() or (MAN / f).read_text() != s]
    if a.check:
        if stale:
            print("man pages out of date (run scripts/gen-man.py):", ", ".join(stale)); sys.exit(1)
        print(f"man: {len(pages)} pages current"); return
    MAN.mkdir(exist_ok=True)
    for f, s in pages.items():
        (MAN / f).write_text(s)
    print(f"man: wrote {len(pages)} pages ({len(stale)} changed)")

if __name__ == "__main__":
    main()
