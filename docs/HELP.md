# In-app help

These guides are the help: the same files are built into the programs, so the docs and the
help never differ.

- **Site app and admin app:** the Help page (site app: the Help entry in the menu; admin app:
  Settings > Help). Pick a topic or search titles and text.
- **itcmdr-cli and itcmdr-admin:** `help` lists topics; `help <topic>` prints one, for example
  `itcmdr-cli help backup` or `itcmdr-admin help certificates`. Topic names are the file names in
  lower case without `.md`. Command help is still `--help`.

The apps embed a generated copy (`lib/help_topics.g.dart`, from `scripts/gen-help.py`); a test fails
when it differs from `docs/`. Archived pages in `docs/historical/` are not help topics.
