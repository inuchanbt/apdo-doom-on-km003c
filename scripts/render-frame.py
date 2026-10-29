from pathlib import Path
from PIL import Image
import struct
root=Path(__file__).resolve().parents[1]
source=root/'artifacts/host-engine/frame-099.rgb565'
rgb=bytearray()
for (c,) in struct.iter_unpack('<H',source.read_bytes()):
    rgb.extend((((c>>11)&31)*255//31,((c>>5)&63)*255//63,(c&31)*255//31))
image=Image.frombytes('RGB',(320,240),bytes(rgb))
image.save(root/'artifacts/host-engine/preview.png')
