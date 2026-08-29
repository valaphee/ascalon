interface Protocol {
  id: number;
  msgs: Messages[];
}

interface Messages {
  id: number;
  client: Message[];
  server: Message[];
}

interface Message {
  id: number;
  addr: string | null;
  elem: MessageField[];
}

interface MessageField {
  type: string;
  size: number;
  elem: MessageField[];
}

const protocols: Protocol[] = [];

rpc.exports = {
  getProtocols: () => protocols,
};

Interceptor.attach(Process.mainModule.base.add(0xfea110), {
  onEnter(args) {
    const mc = args[0];
    const rawData = args[2];

    const protocolId = mc.add(0x10).readU32();
    const protocol = protocols.find((protocol) => protocol.id == protocolId);
    if (protocol === undefined) return;

    const messageId = rawData.readU16();
    for (const msgs of protocol.msgs) {
      const message = msgs.client.find((message) => message.id === messageId);

      if (message !== undefined) {
        message.addr = this.returnAddress
          .sub(Process.mainModule.base)
          .add(0x140000000)
          .toString();
        return;
      }
    }
  },
});

Interceptor.attach(Process.mainModule.base.add(0xfed670), {
  onEnter(args) {
    registerMessages(
      args[0].toUInt32(),
      args[6].toUInt32(),
      args[2].toUInt32(),
      args[3],
      args[4].toUInt32(),
      args[5],
    );
  },
});

Interceptor.attach(Process.mainModule.base.add(0xfed730), {
  onEnter(args) {
    registerMessages(
      args[0].toUInt32(),
      args[6].toUInt32(),
      args[2].toUInt32(),
      args[3],
      args[4].toUInt32(),
      args[5],
    );
  },
});

Interceptor.attach(Process.mainModule.base.add(0xfed7f0), {
  onEnter(args) {
    registerMessages(
      args[0].toUInt32(),
      args[4].toUInt32(),
      0,
      null,
      args[2].toUInt32(),
      args[3],
    );
  },
});

Interceptor.attach(Process.mainModule.base.add(0xfed880), {
  onEnter(args) {
    registerMessages(
      args[0].toUInt32(),
      args[4].toUInt32(),
      args[2].toUInt32(),
      args[3],
      0,
      null,
    );
  },
});

function registerMessages(
  protocolId: number,
  messagesId: number,
  sendMsgCount: number,
  sendMsgArray: NativePointer | null,
  recvMsgCount: number,
  recvMsgArray: NativePointer | null,
): void {
  let protocol = protocols.find((protocol) => protocol.id === protocolId);
  if (protocol === undefined) {
    protocol = {
      id: protocolId,
      msgs: [],
    };
    protocols.push(protocol);
  }

  let messages = protocol.msgs.find((messages) => messages.id == messagesId);
  if (messages === undefined) {
    messages = {
      id: messagesId,
      client: [],
      server: [],
    };
    protocol.msgs.push(messages);
  }

  if (sendMsgArray != null) {
    for (let i = 0; i < sendMsgCount; i++) {
      const ptr = sendMsgArray.add(i * Process.pointerSize);
      const defArray = ptr.readPointer();

      const messageId = defArray.add(0x10).readU16();
      messages.client.push({
        id: messageId,
        elem: parseFields(defArray.add(0x28)),
        addr: null,
      });
    }
  }

  if (recvMsgArray != null) {
    for (let i = 0; i < recvMsgCount; i++) {
      const ptr = recvMsgArray.add(i * Process.pointerSize * 2);
      const defArray = ptr.readPointer();
      const dispatch = ptr.add(Process.pointerSize).readPointer();

      const messageId = defArray.add(0x10).readU16();
      messages.server.push({
        id: messageId,
        elem: parseFields(defArray.add(0x28)),
        addr: dispatch.sub(Process.mainModule.base).add(0x140000000).toString(),
      });
    }
  }
}

function parseFields(defArray: NativePointer): MessageField[] {
  const fields: MessageField[] = [];

  for (let def = defArray; def.readU32() != 0x00; def = def.add(0x28)) {
    fields.push(parseField(def));
  }

  return fields;
}

function parseField(def: NativePointer): MessageField {
  const fieldType = def.readU32();
  const param = def.add(0x10).readU32();
  const refTypeDef = def.add(0x18).readPointer();

  const types: Record<number, string> = {
    0x02: "Byte",
    0x03: "Word",
    0x04: "Dword",
    0x05: "Qword",
    0x06: "Float",
    0x07: "Float2",
    0x08: "Float3",
    0x09: "Float4",
    0x0a: "Point3",
    0x0b: "Guid",
    0x0c: "Address",
    0x0d: "String",
    0x0e: "CString",
    0x0f: "Optional",
    0x10: "ArrayFixed",
    0x11: "ArrayVarSmall",
    0x12: "ArrayVarLarge",
    0x13: "BufferFixed",
    0x14: "BufferVarSmall",
    0x15: "BufferVarLarge",
    0x16: "SrvAlign",
    0x17: "SrvDword",
  };

  return {
    type: types[fieldType] ?? "Unknown",
    size: param,
    elem: refTypeDef.isNull() ? [] : parseFields(refTypeDef),
  };
}
