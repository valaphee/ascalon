import secrets
import struct
from pathlib import Path

import pefile
from Crypto.Util.number import getPrime

path = Path(r"C:\Program Files\Guild Wars 2\Gw2-64.exe")
data = bytearray(path.read_bytes())

pe = pefile.PE(data=bytes(data))
rdata = next(x for x in pe.sections if x.Name.rstrip(b"\0") == b".rdata")

DH_PARAMS_SIZE = 136

v = 1
g = 4

o = next(
    i
    for i in range(
        rdata.PointerToRawData,
        rdata.PointerToRawData + rdata.SizeOfRawData - DH_PARAMS_SIZE + 1,
    )
    if struct.unpack_from("<II", data, i) == (v, g)
    and data[i + 8 : i + DH_PARAMS_SIZE].count(0) <= 3
)

Path("dh_params.bin").write_bytes(data[o : o + DH_PARAMS_SIZE])
Path(str(path) + ".bak").write_bytes(data)

p = getPrime(512)
y = secrets.randbits(512)
x = pow(g, y, p)

dh_params = (
    struct.pack("<II", v, g) + p.to_bytes(64, "little") + x.to_bytes(64, "little")
)

data[o : o + DH_PARAMS_SIZE] = dh_params

path.write_bytes(data)
Path("proxy/dh_params.bin").write_bytes(dh_params + y.to_bytes(64, "little"))
