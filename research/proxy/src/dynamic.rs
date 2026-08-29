use std::ops::{Index, IndexMut};
use std::{fmt, net::SocketAddr};

use ascalon_protocol::{Decode, Encode};
use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

pub struct Message {
    pub id: u16,
    pub name: String,
    pub elem: Vec<MessageField>,
}

pub struct MessageField {
    pub name: String,
    pub value: MessageFieldValue,
}

pub enum MessageFieldValue {
    Byte(u8),
    Word(u16),
    Dword(u32),
    Qword(u64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float4([f32; 4]),
    Point3([f32; 3], u32),
    Guid(Uuid),
    Address(SocketAddr),
    String(Vec<u16>),
    CString(String),
    Optional(Option<Box<MessageFieldValue>>),
    ArrayFixed(Vec<MessageFieldValue>),
    ArrayVarSmall(Vec<MessageFieldValue>),
    ArrayVarLarge(Vec<MessageFieldValue>),
    BufferFixed(Vec<u8>),
    BufferVarSmall(Vec<u8>),
    BufferVarLarge(Vec<u8>),
    Struct(Vec<MessageField>),
}

impl Index<&str> for Message {
    type Output = MessageFieldValue;

    fn index(&self, name: &str) -> &Self::Output {
        self.elem
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
            .unwrap()
    }
}

impl IndexMut<&str> for Message {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        self.elem
            .iter_mut()
            .find(|field| field.name == name)
            .map(|field| &mut field.value)
            .unwrap()
    }
}

impl Index<&str> for MessageFieldValue {
    type Output = MessageFieldValue;

    fn index(&self, name: &str) -> &Self::Output {
        match self {
            Self::Struct(v) => &v.iter().find(|f| f.name == name).unwrap().value,
            _ => panic!(),
        }
    }
}

impl IndexMut<&str> for MessageFieldValue {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        match self {
            Self::Struct(v) => &mut v.iter_mut().find(|f| f.name == name).unwrap().value,
            _ => panic!(),
        }
    }
}

macro_rules! impl_accessors {
    ($(
        $variant:ident => $ty:ty, $get:ident, $get_mut:ident
    );* $(;)?) => {
        impl MessageFieldValue {
            $(
                pub fn $get(&self) -> &$ty {
                    match self {
                        Self::$variant(value) => value,
                        _ => panic!(),
                    }
                }

                pub fn $get_mut(&mut self) -> &mut $ty {
                    match self {
                        Self::$variant(value) => value,
                        _ => panic!(),
                    }
                }
            )*
        }
    };
}

impl_accessors! {
    Byte    => u8,         as_u8,      as_u8_mut;
    Word    => u16,        as_u16,     as_u16_mut;
    Dword   => u32,        as_u32,     as_u32_mut;
    Qword   => u64,        as_u64,     as_u64_mut;
    Float   => f32,        as_f32,     as_f32_mut;
    Float2  => [f32; 2],   as_float2,  as_float2_mut;
    Float3  => [f32; 3],   as_float3,  as_float3_mut;
    Float4  => [f32; 4],   as_float4,  as_float4_mut;
    Guid    => Uuid,       as_guid,    as_guid_mut;
    Address => SocketAddr, as_address, as_address_mut;
    CString => String,     as_cstring, as_cstring_mut;
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r#struct = f.debug_struct(&self.name);
        for field in &self.elem {
            r#struct.field(&field.name, &field.value);
        }
        r#struct.finish()
    }
}

impl fmt::Debug for MessageFieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Byte(v) => v.fmt(f),
            Self::Word(v) => v.fmt(f),
            Self::Dword(v) => v.fmt(f),
            Self::Qword(v) => v.fmt(f),
            Self::Float(v) => v.fmt(f),
            Self::Float2(v) => v.fmt(f),
            Self::Float3(v) => v.fmt(f),
            Self::Float4(v) => v.fmt(f),
            Self::Point3(pos, unk) => f.debug_tuple("Point3").field(pos).field(unk).finish(),
            Self::Guid(v) => v.fmt(f),
            Self::Address(v) => v.fmt(f),
            Self::String(v) => String::from_utf16_lossy(v).fmt(f),
            Self::CString(v) => v.fmt(f),
            Self::Optional(v) => v.fmt(f),
            Self::ArrayFixed(v) | Self::ArrayVarSmall(v) | Self::ArrayVarLarge(v) => v.fmt(f),
            Self::BufferFixed(v) | Self::BufferVarSmall(v) | Self::BufferVarLarge(v) => {
                for b in v {
                    write!(f, "{b:02x}")?;
                }

                Ok(())
            }
            Self::Struct(v) => {
                let mut r#struct = f.debug_struct("");
                for field in v {
                    r#struct.field(&field.name, &field.value);
                }
                r#struct.finish()
            }
        }
    }
}

impl Encode for Message {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        buf.put_u16_le(self.id);
        self.elem.iter().try_for_each(|f| f.value.encode(buf))
    }
}

impl Encode for MessageFieldValue {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        match self {
            MessageFieldValue::Byte(v) => v.encode(buf),
            MessageFieldValue::Word(v) => v.encode(buf),
            MessageFieldValue::Dword(v) => v.encode(buf),
            MessageFieldValue::Qword(v) => v.encode(buf),
            MessageFieldValue::Float(v) => v.encode(buf),
            MessageFieldValue::Float2(v) => v.encode(buf),
            MessageFieldValue::Float3(v) => v.encode(buf),
            MessageFieldValue::Float4(v) => v.encode(buf),
            MessageFieldValue::Guid(v) => v.encode(buf),
            MessageFieldValue::Point3(pos, unk) => {
                pos.encode(buf)?;
                unk.encode(buf)
            }
            MessageFieldValue::Address(v) => v.encode(buf),
            MessageFieldValue::String(v) => {
                v.iter().for_each(|&v| buf.put_u16_le(v));
                0u16.encode(buf)
            }
            MessageFieldValue::CString(v) => {
                buf.put_slice(v.as_bytes());
                0u8.encode(buf)
            }
            MessageFieldValue::Optional(v) => match v {
                None => 0u8.encode(buf),
                Some(value) => {
                    1u8.encode(buf)?;
                    value.encode(buf)
                }
            },
            MessageFieldValue::ArrayFixed(v) => v.iter().try_for_each(|v| v.encode(buf)),
            MessageFieldValue::ArrayVarSmall(v) => {
                buf.put_u8(u8::try_from(v.len()).map_err(|_| ())?);
                v.iter().try_for_each(|v| v.encode(buf))
            }
            MessageFieldValue::ArrayVarLarge(v) => {
                buf.put_u16_le(u16::try_from(v.len()).map_err(|_| ())?);
                v.iter().try_for_each(|v| v.encode(buf))
            }

            MessageFieldValue::BufferFixed(v) => {
                buf.put_slice(v);
                Ok(())
            }
            MessageFieldValue::BufferVarSmall(v) => {
                u8::try_from(v.len()).map_err(|_| ())?.encode(buf)?;
                buf.put_slice(v);
                Ok(())
            }
            MessageFieldValue::BufferVarLarge(v) => {
                u16::try_from(v.len()).map_err(|_| ())?.encode(buf)?;
                buf.put_slice(v);
                Ok(())
            }
            MessageFieldValue::Struct(v) => v.iter().try_for_each(|v| v.value.encode(buf)),
        }
    }
}

pub fn decode(
    protocols: &[ascalon_protocol_schema::Protocol],
    protocol: &str,
    server: bool,
    mut data: &mut &[u8],
) -> Result<Message, ()> {
    let id = data.try_get_u16_le().map_err(|_| ())?;

    let protocol = protocols.iter().find(|p| p.name == protocol).ok_or(())?;
    let (group, message) = protocol
        .msgs
        .iter()
        .find_map(|group| {
            let messages = if server { &group.client } else { &group.server };
            messages.iter().find(|m| m.id == id).map(|m| (group, m))
        })
        .ok_or(())?;
    let fields = decode_fields(&message.elem, &mut data)?;

    Ok(Message {
        id,
        name: format!("{}::{}", group.name, message.name),
        elem: fields,
    })
}

fn decode_fields(
    fields: &[ascalon_protocol_schema::MessageField],
    buf: &mut &[u8],
) -> Result<Vec<MessageField>, ()> {
    fields
        .iter()
        .map(|field| {
            Ok(MessageField {
                name: field.name.clone(),
                value: decode_value(&field.r#type, Some(field), buf)?,
            })
        })
        .collect()
}

fn decode_value(
    ty: &str,
    field: Option<&ascalon_protocol_schema::MessageField>,
    buf: &mut &[u8],
) -> Result<MessageFieldValue, ()> {
    Ok(match ty {
        "Byte" => MessageFieldValue::Byte(Decode::decode(buf)?),
        "Word" => MessageFieldValue::Word(Decode::decode(buf)?),
        "Dword" => MessageFieldValue::Dword(Decode::decode(buf)?),
        "Qword" => MessageFieldValue::Qword(Decode::decode(buf)?),
        "Float" => MessageFieldValue::Float(Decode::decode(buf)?),
        "Float2" => MessageFieldValue::Float2(Decode::decode(buf)?),
        "Float3" => MessageFieldValue::Float3(Decode::decode(buf)?),
        "Float4" => MessageFieldValue::Float4(Decode::decode(buf)?),
        "Guid" => MessageFieldValue::Guid(Decode::decode(buf)?),
        "Point3" => MessageFieldValue::Point3(Decode::decode(buf)?, Decode::decode(buf)?),
        "Address" => MessageFieldValue::Address(Decode::decode(buf)?),
        "String" => {
            let field = field.ok_or(())?;

            let mut value = Vec::with_capacity(field.size);

            loop {
                let word = buf.try_get_u16_le().map_err(|_| ())?;
                if word == 0 {
                    break MessageFieldValue::String(value);
                }

                if value.len() == field.size {
                    return Err(());
                }

                value.push(word);
            }
        }
        "CString" => {
            let field = field.ok_or(())?;

            let end = buf
                .iter()
                .take(field.size)
                .position(|&byte| byte == 0)
                .ok_or(())?;

            let value = std::str::from_utf8(&buf[..end]).map_err(|_| ())?.to_owned();

            buf.advance(end + 1);

            MessageFieldValue::CString(value)
        }
        "Optional" => {
            let field = field.ok_or(())?;

            MessageFieldValue::Optional(match buf.try_get_u8().map_err(|_| ())? {
                0 => None,
                1 => Some(Box::new(decode_value_inner(field, buf)?)),
                _ => return Err(()),
            })
        }
        "ArrayFixed" => {
            let field = field.ok_or(())?;

            let len = field.size;

            MessageFieldValue::ArrayFixed(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "ArrayVarSmall" => {
            let field = field.ok_or(())?;

            let len = u8::decode(buf)? as usize;
            if len > field.size {
                return Err(());
            }

            MessageFieldValue::ArrayVarSmall(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "ArrayVarLarge" => {
            let field = field.ok_or(())?;

            let len = u16::decode(buf)? as usize;
            if len > field.size {
                return Err(());
            }

            MessageFieldValue::ArrayVarLarge(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "BufferFixed" => {
            let field = field.ok_or(())?;

            let len = field.size;

            let value = Vec::<u8>::from(buf.get(..len).ok_or(())?);
            *buf = &buf[len..];

            MessageFieldValue::BufferFixed(value)
        }
        "BufferVarSmall" => {
            let field = field.ok_or(())?;

            let len = u8::decode(buf)? as usize;
            if len > field.size {
                return Err(());
            }

            let value = Vec::<u8>::from(buf.get(..len).ok_or(())?);
            *buf = &buf[len..];

            MessageFieldValue::BufferVarSmall(value)
        }
        "BufferVarLarge" => {
            let field = field.ok_or(())?;

            let len = u16::decode(buf)? as usize;
            if len > field.size {
                return Err(());
            }

            let value = Vec::<u8>::from(buf.get(..len).ok_or(())?);
            *buf = &buf[len..];

            MessageFieldValue::BufferVarLarge(value)
        }
        _ => return Err(()),
    })
}

fn decode_value_inner(
    field: &ascalon_protocol_schema::MessageField,
    buf: &mut &[u8],
) -> Result<MessageFieldValue, ()> {
    if !field.elem.is_empty() {
        return Ok(MessageFieldValue::Struct(decode_fields(&field.elem, buf)?));
    }

    decode_value(field.type_name.as_deref().ok_or(())?, None, buf)
}
