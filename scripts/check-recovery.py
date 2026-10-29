"""Verify saved recovery artifacts without accessing USB or modifying the device."""
from pathlib import Path
import hashlib
import json
root=Path(__file__).resolve().parents[1]
config=json.loads((root/'local/device.json').read_text(encoding='utf-8-sig'))
stock=root/config['stock_backup']
external=root/config['external_backup']
checks=[]
for sidecar in stock.glob('*.bin.sha256'):
    expected,name=sidecar.read_text().strip().split('  ',1)
    data=(stock/name).read_bytes()
    assert hashlib.sha256(data).hexdigest()==expected,name
    verified=json.loads((stock/(name+'.verified.json')).read_text())
    assert verified['matching_reads']==2,name
    checks.append(name)
data=(external/'external-4MiB.bin').read_bytes()
meta=json.loads((external/'verified.json').read_text())
assert len(data)==0x400000 and hashlib.sha256(data).hexdigest()==meta['sha256']
assert meta['matching_reads']==2 and meta['stock_prefix_matches']
assert data[:0x300000]==(stock/'external-first-3MiB.bin').read_bytes()
assert (stock/'loader-00000004-00003fff.bin').read_bytes()==(stock/'dfu-recovery-check/loader.bin').read_bytes()
restore=root/'artifacts/stock-restore-check'
assert (restore/'application-readback.bin').read_bytes()==(stock/'stock-app.bin').read_bytes()
assert (restore/'loader-before.bin').read_bytes()==(restore/'loader-after.bin').read_bytes()
assert json.loads((restore/'verified.json').read_text())['loader_unchanged']
roundtrips=sorted((root/'artifacts').glob('restore-nor-*/verified.json'))
assert roundtrips, 'complete restoration evidence missing'
last=roundtrips[-1]
full=json.loads(last.read_text())
assert full['full_external_restore_verified'] and full['protected_tail_unchanged']
assert full['sha256']==meta['sha256']
app=last.parent.parent/last.parent.name.replace('restore-nor-', 'restore-stock-')
assert (app/'application-readback.bin').read_bytes()==(stock/'stock-app.bin').read_bytes()
assert (app/'loader-before.bin').read_bytes()==(app/'loader-after.bin').read_bytes()
assert json.loads((app/'verified.json').read_text())['loader_unchanged']
print('PASS: stock app, loader, calibration, internal tail, full 4 MiB NOR, and complete hardware restoration readback')
