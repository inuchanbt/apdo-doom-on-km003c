# Remove leftover DOOM data after restoring stock firmware

Restoring the stock **application alone leaves DOOM.DAT in external NOR**. The
MCU application and game storage are separate. A complete rollback restores the
original NOR bytes and then the stock application; it does not just erase the
flash to blank values.

For **HW2.3**, extract the v0.1.2 Windows installer ZIP. If the original stock
application is already running:

1. Close MTOOLS and the Web controller. Connect only the side HID connector on
   the same PC USB port used for the original installation; leave both measurement
   connectors empty.
2. Double-click **`Remove-Leftover-DOOM-Data.cmd`**, or choose **5** in
   `Install-APDO-DOOM.cmd`. Enter the original installation backup folder (the
   session folder containing `stock` and `nor`, not either subfolder).
3. Read the prompt and type `REMOVE-DATA`. Follow the DFU and **hold OK** prompts.
   Wait for the completion message, then check the stock screen and measurements.

**This writes firmware temporarily and carries the same bricking/data-loss risk
as installation.** Before the first write it verifies the qualified stock and
full original NOR backup, exact game pack, selected USB port, native loader and
the installed stock application. It temporarily installs the included DOOM
storage service, restores the original NOR, verifies all 4 MiB, returns to DFU
and restores/verifies the original application. Game initialization must stay
disabled by holding OK during the service startup.

The tool stops on missing/mismatched backups or unknown current NOR contents.
It preserves the final 1 MiB and does not continue to stock-app flashing after a
failed NOR restore. Keep the error and the new `run-*` evidence folder if it
stops. Recovery is not guaranteed. You need your own **original, verified 4 MiB
NOR backup**, qualified stock backup and `restore.mencrypt`; GitHub does not
provide replacement factory data. Do not acquire a new "original" NOR backup
after game data was written, substitute another unit's dump or blanket-erase NOR.

If DOOM is still installed, choose **3 — Restore original NOR and stock
firmware** instead. That completes the ordinary rollback in the correct order.
This new stock-first cleanup workflow has software orchestration checks; it
has **not** been exercised on hardware as part of this release.

## 日本語

純正ファームだけに戻しても、外部NORに `DOOM.DAT` が残ります。元に戻すには、
**導入前のNORデータを復元してから、純正アプリへ戻す**必要があります。

HW2.3用のv0.1.2 Windows ZIPを展開し、既に純正アプリへ戻した場合は
**`Remove-Leftover-DOOM-Data.cmd`** をダブルクリックするか、通常メニューの **5** を選びます。
`stock` と `nor` を含む導入時のバックアップフォルダーを指定し、説明を読んで
`REMOVE-DATA` と入力します。その後は表示に従ってDFU操作とOK長押しを行います。
測定側は空にし、元と同じPC側USBポートに側面HIDだけをつないでください。

一時的にDOOMの保存領域サービスを書き込み、元のNORを全4 MiB照合付きで復元してから、
純正アプリを復元します。**再度ファームを書き換えるため文鎮化・データ消失リスクがあります。**
元の検証済みバックアップ一式が必要です。途中で止まった場合はエラーと記録を保管してください。
この追加操作の実機検証は未実施です。DOOMがまだ入っている場合は通常メニューの **3** を使います。
