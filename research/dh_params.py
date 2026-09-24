import struct
from pathlib import Path

import pefile

path = Path(r"C:\Program Files\Guild Wars 2\Gw2-64.exe")
data = bytearray(path.read_bytes())

pe = pefile.PE(data=bytes(data))

rdata = next(
    section
    for section in pe.sections
    if section.Name.rstrip(b"\0") == b".rdata")

DH_PARAMS_SIZE = 136

v = 1
g = 4

dh_params = next(
    pos
    for pos in range(
        rdata.PointerToRawData,
        rdata.PointerToRawData + rdata.SizeOfRawData - DH_PARAMS_SIZE + 1,
    )
    if struct.unpack_from("<II", data, pos) == (v, g)
    and data[pos + 8 : pos + DH_PARAMS_SIZE].count(0) <= 3
)

Path("dh_params.bin").write_bytes(data[dh_params : dh_params + DH_PARAMS_SIZE])
