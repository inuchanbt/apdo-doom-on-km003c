# Restore the original tester — independent firmware

**Recovery is not guaranteed.** One successful round trip does not cover every
unit, hardware revision or interrupted write. Connect only side HID and keep
both measurement connectors empty. Use this meter's own qualified stock/DFU
backup, original restore container and full original 4 MiB NOR backup.

Follow [INSTALL-PREBUILT.md, step 5](INSTALL-PREBUILT.md#5-return-to-stock-restore-nor-first-then-the-original-app)
or [FIRST-BACKUP.md, step 7](FIRST-BACKUP.md#7-restore-original-tester-functionality)
for the exact commands. **Restore NOR first, then the original application.**
A stock application file alone does not restore overwritten game-data sectors.

Stop the Web controller before using the USB CLI. Enter native DFU, then start
the independent service while holding physical OK to prevent engine
initialization. Verify that `apdo-host info` shows no rendered frames before
`restore-nor`; pausing an initialized engine does not enable storage writes.
After complete NOR readback succeeds, return to DFU, flash the qualified original
restore package, and start it only after application/loader verification succeeds.
Confirm the original UI and functions physically.

The service never programs the protected final 1 MiB. A tail mismatch, unrecognized
current prefix, wrong physical port or failed readback must stop the procedure.
Do not force a mismatch, retry an ambiguous programming failure, write a
loader/ICG or substitute another unit's backups.

If application USB is unavailable, unplug HID and reconnect on the same physical
PC port while holding Exit; release it at the native update/DFU screen. Preserve
read-only retained-RAM diagnostics with:

```powershell
independent/target/release/apdo-host.exe diagnostics STOCK_BACKUP NEW_OUTPUT_DIRECTORY
```

Keep originals and failure evidence offline before deciding how to proceed.
Do not start the stock app after a failed NOR restoration. If the independent
service cannot start, this document does not promise a repair path for that
failure; native DFU availability alone does not prove complete recoverability.

The owner reported the complete independent initial-install/stock-restore
workflow successful on the same HW2.3 unit on **2026-10-31**. See
[VALIDATION.md](VALIDATION.md) for scope and evidence attribution. Backups are
private, and GitHub does not supply replacement factory data.

The former `scripts/restore-stock.ps1`, embedded-baseline host and its commands
are preserved on [legacy](https://github.com/inuchanbt/apdo-doom-on-km003c/tree/legacy).
They are not the restoration procedure for the independent build. See
[branch migration](BRANCHES.md).
