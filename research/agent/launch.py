import xml.etree.ElementTree as ET

import frida

PROGRAM = r"C:\Program Files\Guild Wars 2\Gw2-64.exe"
SCRIPT = "index.js"


def export_protocols(protocols):
    root = ET.Element("Protocols")

    protocols.sort(key=lambda protocol: protocol["id"])
    for protocol in protocols:
        protocol_elem = ET.SubElement(root, "Protocol")
        protocol_elem.set("Name", f"Unknown{protocol['id']}")

        protocol["msgs"].sort(key=lambda messages: messages["id"])
        for messages in protocol["msgs"]:
            messages_elem = ET.SubElement(protocol_elem, "Messages")
            messages_elem.set("Name", f"Unknown{messages['id']}")

            client_elem = ET.SubElement(messages_elem, "Client")
            server_elem = ET.SubElement(messages_elem, "Server")

            for message in messages["client"]:
                add_message(client_elem, message)

            for message in messages["server"]:
                add_message(server_elem, message)

    ET.indent(root, space="  ")

    ET.ElementTree(root).write(
        "protocols.xml",
        encoding="utf-8",
        xml_declaration=True,
    )


def add_message(parent, message):
    elem = ET.SubElement(parent, "Message")

    elem.set("Id", str(message["id"]))
    elem.set("Name", f"Unknown{message['id']}")

    if message["addr"] is not None:
        elem.set("Addr", message["addr"])

    for i, field in enumerate(message["elem"]):
        add_message_field(elem, field, i)


def add_message_field(parent, field, index):
    elem = ET.SubElement(parent, field["type"])
    elem.set("Name", f"unknown{index}")

    if field["size"] != 0:
        elem.set("Size", str(field["size"]))

    if (
        field["type"]
        in {
            "Optional",
            "ArrayFixed",
            "ArrayVarSmall",
            "ArrayVarLarge",
        }
        and len(field["elem"]) == 1
        and not field["elem"][0]["elem"]
        and field["elem"][0]["type"] not in {"String", "CString"}
    ):
        elem.set("TypeName", field["elem"][0]["type"])
        return

    for i, child in enumerate(field["elem"]):
        add_message_field(elem, child, i)


device = frida.get_local_device()
pid = device.spawn([PROGRAM])
session = device.attach(pid)

with open(SCRIPT, "r", encoding="utf-8") as f:
    source = f.read()

script = session.create_script(source)
script.load()

device.resume(pid)

input()

protocols = script.exports_sync.get_protocols()

export_protocols(protocols)

session.detach()
