"""Report byte differences from the public HW2.1 loader without changing it."""
from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parents[1]
config = json.loads((root/'local/device.json').read_text(encoding='utf-8-sig'))
backup = root / config['stock_backup']
reference = (root / 'vendor/km003c-firmware/fw/bootloader_hw2.1_00000004-00003fff.bin').read_bytes()
actual = (backup / 'loader-00000004-00003fff.bin').read_bytes()
assert len(reference) == len(actual) == 0x3ffc
changed = [i for i, (a,b) in enumerate(zip(reference, actual)) if a != b]
groups = []
for i in changed:
    if groups and i == groups[-1][-1] + 1:
        groups[-1].append(i)
    else:
        groups.append([i])
result = {
    'reference_sha256': hashlib.sha256(reference).hexdigest(),
    'actual_sha256': hashlib.sha256(actual).hexdigest(),
    'changed_bytes': len(changed),
    'differences': [{'start': hex(g[0]+4), 'end_exclusive': hex(g[-1]+5),
        'reference': reference[g[0]:g[-1]+1].hex(), 'actual': actual[g[0]:g[-1]+1].hex()}
        for g in groups],
}
print(json.dumps({k:v for k,v in result.items() if k!='differences'}, indent=2))
(backup / 'loader-comparison.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
