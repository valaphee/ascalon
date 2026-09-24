import struct
import sys
import xml.etree.ElementTree as ET

import pefile


TYPES = {
    1: "Array",
    2: "ArrayPtr",
    3: "ArrayPtr",
    5: "Byte",
    6: "Byte4",
    10: "Dword",
    11: "WcharPtr",
    12: "Float",
    13: "Float2",
    14: "Float3",
    15: "Float4",
    16: "Ptr",
    17: "Qword",
    18: "WcharPtr",
    19: "CharPtr",
    20: "Struct",
    21: "Word",
    22: "Guid",
    23: "Byte3",
    24: "Dword2",
    25: "Dword4",
    26: "Word3",
    27: "WcharPtr",
    28: "Union",
    29: "Struct",
    36: "Dword",
    37: "Qword",
}


data = open(sys.argv[1], "rb").read()
size = len(data)

pe = pefile.PE(data=data)
base = pe.OPTIONAL_HEADER.ImageBase


def offset(address):
    try:
        pos = pe.get_offset_from_rva(address - base)
        return pos if 0 <= pos < size else None
    except Exception:
        return None


def cstring(address):
    if (pos := offset(address)) is None:
        return None

    end = data.find(b"\0", pos)
    if end < 0:
        return None

    try:
        return data[pos:end].decode("ascii")
    except UnicodeDecodeError:
        return None


def add_struct(parent, address):
    if (pos := offset(address)) is None:
        return False

    while pos + 32 <= size:
        type, name, reference_type, array_count = struct.unpack_from(
            "<H6xQQQ",
            data,
            pos,
        )

        if (_name := cstring(name)) is None:
            return False

        if type == 0:
            parent.set("TypeName", _name)
            return True

        if (type_name := TYPES.get(type)) is None:
            return False

        elem = ET.SubElement(
            parent,
            type_name,
            Name=_name,
            **({"Size": str(array_count)} if array_count else {}),
        )

        if reference_type:
            if type == 28:
                if (array_pos := offset(reference_type)) is None:
                    return False

                array_end = array_pos + array_count * 8
                if array_end > size:
                    return False

                for (variant,) in struct.iter_unpack(
                    "<Q",
                    data[array_pos:array_end],
                ):
                    if variant and not add_struct(
                        ET.SubElement(elem, "Struct"),
                        variant,
                    ):
                        return False
            else:
                if not add_struct(elem, reference_type):
                    return False

                if (
                    len(elem) == 1
                    and (item := elem[0]).get("Name") == ""
                    and not len(item)
                ):
                    elem.set("TypeName", item.tag)
                    elem.remove(item)

        pos += 32

    return False


root = ET.Element("Chunks")

rdata = next(
    section
    for section in pe.sections
    if section.Name.rstrip(b"\0") == b".rdata"
)

seen_chunks = set()

for chunk_pos in range(
    rdata.PointerToRawData,
    rdata.PointerToRawData + rdata.SizeOfRawData - 16,
    4,
):
    name, version_count, versions = struct.unpack_from(
        "<4sIQ",
        data,
        chunk_pos,
    )

    name = name.rstrip(b"\0")
    if not name or not name.isascii():
        continue

    if not version_count:
        continue

    versions_pos = offset(versions)
    if versions_pos is None or versions_pos + 8 > size:
        continue

    if versions in seen_chunks:
        continue

    chunk_elem = ET.SubElement(
        root,
        "Chunk",
        Name=name.decode(),
    )

    for version_index in range(version_count):
        version_pos = versions_pos + version_index * 24
        if version_pos + 8 > size:
            break

        (version,) = struct.unpack_from("<Q", data, version_pos)
        if not version:
            continue

        version_elem = ET.SubElement(
            chunk_elem,
            "Version",
            Id=str(version_index),
        )

        if not add_struct(version_elem, version):
            chunk_elem.remove(version_elem)
            break

    seen_chunks.add(versions)

    if not len(chunk_elem):
        root.remove(chunk_elem)


ET.indent(root, space="  ")

ET.ElementTree(root).write(
    sys.argv[2] if len(sys.argv) > 2 else "packfiles.xml",
    encoding="utf-8",
    xml_declaration=True,
)
