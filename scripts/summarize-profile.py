"""Summarize game-profile JSON without device identifiers or USB access."""
import argparse
import json
from pathlib import Path
from statistics import median

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('profiles', type=Path, nargs='+')
parser.add_argument('--discard', type=int, default=5)
args = parser.parse_args()
if args.discard < 0:
    parser.error('--discard must be nonnegative')
results = {}
for path in args.profiles:
    samples = json.loads(path.read_text(encoding='utf-8-sig'))[args.discard:]
    if not samples:
        parser.error(f'{path}: no samples remain')
    clocks = {sample['hclk_hz'] for sample in samples}
    if len(clocks) != 1 or min(clocks) <= 0:
        parser.error(f'{path}: invalid or mixed clock rates')
    hz = clocks.pop()
    result = {'samples': len(samples), 'discarded': args.discard, 'hclk_hz': hz,
              'arena_address': hex(samples[0]['arena_address']),
              'view_width': samples[0]['view_width']}
    for field in ['nor_read_hz', 'verified_read_bytes']:
        if field in samples[0]:
            result[field] = samples[0][field]
    if 'interval_us' in samples[0]:
        periods = sorted(sample['interval_us']/1000 for sample in samples)
        result['pacing_enabled'] = samples[0]['pacing_enabled']
        result['frame_period_ms'] = round(median(periods), 3)
        result['cadence_fps'] = round(1000/median(periods), 3)
        result['frame_period_min_ms'] = round(periods[0], 3)
        result['frame_period_p90_ms'] = round(periods[int(len(periods)*0.9)], 3)
        result['frame_period_max_ms'] = round(periods[-1], 3)
    for field in ['total', 'nor', 'lcd', 'capture']:
        result[field+'_ms'] = round(median(s[field+'_cycles'] for s in samples)*1000/hz, 3)
    result['other_ms'] = round(median(s['total_cycles']-s['nor_cycles']-s['lcd_cycles']-s['capture_cycles'] for s in samples)*1000/hz, 3)
    result['render_fps'] = round(hz/median(s['total_cycles'] for s in samples), 3)
    for field in ['nor_bytes', 'nor_calls', 'lcd_pixels']:
        result[field] = median(s[field] for s in samples)
    results[path.stem] = result
print(json.dumps(results, indent=2))
