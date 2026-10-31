# Branches and tool migration

| Location | Purpose |
| --- | --- |
| `main` | Independent DOOM-only firmware, host, backup utility and current instructions |
| [legacy](https://github.com/inuchanbt/apdo-doom-on-km003c/tree/legacy) | Previous v8 code, overlays, board patch, host tools and historical documentation |
| `codex/independent-firmware` | Original independent-development history, merged into main |
| `v0.1.0-independent` | Fixed release snapshot corresponding to the published binary/source assets |

The legacy branch preserves the former main at commit
`9a64d764483dcb51c87c07fb7b5cb945f213073c`. No legacy v8 binary is published.
The release tag and downloads have not been replaced by the main-branch migration.

## Existing workspaces

Keep private backups, evidence and original files, with a second offline copy.
Switching Git branches does not restore the meter or create recovery material.
The independent tools read qualified backups at runtime; the old board host
embedded a private baseline. Rebuild the tools for the chosen implementation.

| Previous v8 entry point | Independent entry point |
| --- | --- |
| `vendor/km003c-firmware/target/release/km003c-host-cli.exe` | `independent/target/release/apdo-host.exe` |
| `local.example.json` / `scripts/local-config.ps1` | Explicit backup/image/evidence arguments in [INSTALL-PREBUILT.md](INSTALL-PREBUILT.md) |
| `scripts/restore-stock.ps1` | Ordered `restore-nor`, `loader`, `flash`, `start` in [RECOVERY.md](RECOVERY.md) |
| Old board overlays and `patches/km003c-firmware.patch` | `independent/firmware`, `independent/hardware`, `independent/protocol` |
| Old performance/validation records | Corresponding documents on the `legacy` branch |

Use [BUILD.md](BUILD.md) for main and [FIRST-BACKUP.md](FIRST-BACKUP.md) for
untouched stock acquisition. Old host commands and evidence formats must not be
assumed interchangeable. Do not rename a historical directory or invent a
qualification record to satisfy the new tools; stop if validation fails.

The Web controller still starts with `scripts/play.ps1`, but now defaults to the
independent host executable. Stop an older server/helper before switching tools.
Ignored old `vendor/`, `tools/`, `local/`, `artifacts/` and backup files are not
removed by this migration. Never upload private recovery files.
