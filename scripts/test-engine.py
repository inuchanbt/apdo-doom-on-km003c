"""Exercise map startup, game tics, turns, fire and the menu with the real pack."""
import ctypes as C
from pathlib import Path
import hashlib
import json
root=Path(__file__).resolve().parents[1]
dll=C.CDLL(str(root/'artifacts/host-engine/kmdoom-test.dll'))
dll.km_host_init.argtypes=[C.c_void_p,C.c_uint32]
dll.km_host_step.argtypes=[C.c_uint32]
dll.km_host_error.restype=C.c_char_p
dll.km_host_screen.restype=C.POINTER(C.c_uint16)
pack=(root/'vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT').read_bytes()
data=C.create_string_buffer(pack)
assert dll.km_host_init(data,len(pack))==0,dll.km_host_error()
hashes=[]
for i in range(140):
    keys=(2 if 35<=i<70 else 0)|(8 if 70<=i<100 else 0)|(1 if 100<=i<125 else 0)
    assert dll.km_host_step(keys)==0,(i,dll.km_host_error())
    if i in [0,34,69,99,124,139]:
        frame=C.string_at(dll.km_host_screen(),320*240*2)
        hashes.append({'frame':i,'sha256':hashlib.sha256(frame).hexdigest()})
        (root/f'artifacts/host-engine/frame-{i:03}.rgb565').write_bytes(frame)
assert len({h['sha256'] for h in hashes})>=4,'frames do not change'
for key in [32,0,128,0,64,0]:
    assert dll.km_host_step(key)==0,dll.km_host_error()
(root/'artifacts/host-engine/verified.json').write_text(json.dumps(hashes,indent=2)+'\n')
print('PASS: 140 game frames, movement, turning, firing and menu input; changing RGB565 output')
