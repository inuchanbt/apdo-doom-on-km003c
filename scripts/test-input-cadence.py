"""Check that a 100 ms draw cannot queue a frame of stale local input."""
import argparse
import ctypes as C
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--expect-legacy-lag', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
dll = C.CDLL(str(root/'artifacts/host-engine/kmdoom-test.dll'))
dll.km_host_init.argtypes = [C.c_void_p, C.c_uint32]
dll.km_host_error.restype = C.c_char_p
dll.km_host_state.argtypes = [C.POINTER(C.c_uint32)]
pack = (root/'vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT').read_bytes()
data = C.create_string_buffer(pack)
assert dll.km_host_init(data, len(pack)) == 0, dll.km_host_error()
dll.km_host_render_delay(100)
state = (C.c_uint32*9)()
def step(keys):
    assert dll.km_host_step(keys) == 0, dll.km_host_error()
    dll.km_host_state(state)
    return state[3]
for _ in range(12):
    before = step(0)
pressed = step(8)
for _ in range(6):
    held = step(8)
released = step(0)
lag = pressed == before or released != held
if args.expect_legacy_lag:
    assert lag, 'legacy queue no longer reproduces the stale-input delay'
    print('PASS: legacy queued-input delay reproduced with a 100 ms draw')
else:
    assert pressed != before, 'right turn waited behind stale commands'
    assert released == held, 'turn release waited behind stale commands'
    print('PASS: turn press and release apply in the first simulated frame after a slow draw')
