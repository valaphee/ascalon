import secrets
import struct
from pathlib import Path

import pefile
from Crypto.Util.number import getPrime

path = Path(r"C:\Program Files\Guild Wars 2\Gw2-64.exe")
data = bytearray(path.read_bytes())

pe = pefile.PE(data=bytes(data))
rdata = next(x for x in pe.sections if x.Name.rstrip(b"\0") == b".rdata")
start, end = (
    rdata.PointerToRawData,
    rdata.PointerToRawData + rdata.SizeOfRawData,
)

DH_SIZE = 136

o = next(
    i
    for i in range(start, end - DH_SIZE + 1)
    if struct.unpack_from("<II", data, i) == (1, 4)
    and data[i + 8 : i + DH_SIZE].count(0) <= 3
)

Path("dh.bin").write_bytes(data[o : o + DH_SIZE])
Path(str(path) + ".bak").write_bytes(data)

p = getPrime(512)
y = secrets.randbits(512)
x = pow(4, y, p)

dh = struct.pack("<II", 1, 4) + p.to_bytes(64, "little") + x.to_bytes(64, "little")

data[o : o + DH_SIZE] = dh

path.write_bytes(data)
Path("proxy/dh.bin").write_bytes(dh + y.to_bytes(64, "little"))
