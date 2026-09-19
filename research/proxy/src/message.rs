use std::{
    fmt, io,
    net::SocketAddr,
    ops::{Index, IndexMut},
    str,
};

use ascalon_protocol::{Decode, Encode, Point3};
use ascalon_protocol_schema::Field;
use bytes::{BufMut as _, BytesMut};
use indexmap::IndexMap;
use uuid::Uuid;

use crate::STRINGS;

pub struct Message {
    pub id: u16,
    pub name: String,
    fields: IndexMap<String, Value>,
}

pub enum Value {
    Byte(u8),
    Word(u16),
    Dword(u32),
    Qword(u64),
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float4([f32; 4]),
    Point3(Point3),
    Guid(Uuid),
    Address(SocketAddr),
    String(Vec<u16>),
    CString(String),
    Optional(Option<Box<Value>>),
    ArrayFixed(Vec<Value>),
    ArrayVarSmall(Vec<Value>),
    ArrayVarLarge(Vec<Value>),
    BufferFixed(Vec<u8>),
    BufferVarSmall(Vec<u8>),
    BufferVarLarge(Vec<u8>),
    Struct(IndexMap<String, Value>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct(&self.name);
        for (name, value) in &self.fields {
            debug.field(name, value);
        }
        debug.finish()
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Byte(v) => v.fmt(f),
            Value::Word(v) => v.fmt(f),
            Value::Dword(v) => v.fmt(f),
            Value::Qword(v) => v.fmt(f),
            Value::Float(v) => v.fmt(f),
            Value::Float2(v) => v.fmt(f),
            Value::Float3(v) => v.fmt(f),
            Value::Float4(v) => v.fmt(f),
            Value::Point3(v) => v.fmt(f),
            Value::Guid(v) => v.fmt(f),
            Value::Address(v) => v.fmt(f),
            Value::String(v) => decode_coded(&mut &v[..])
                .unwrap_or_else(|_| String::from_utf16_lossy(&v[..v.len() - 1]))
                .fmt(f),
            Value::CString(v) => v.fmt(f),
            Value::Optional(v) => v.fmt(f),
            Value::ArrayFixed(v) | Value::ArrayVarSmall(v) | Value::ArrayVarLarge(v) => v.fmt(f),
            Value::BufferFixed(v) | Value::BufferVarSmall(v) | Value::BufferVarLarge(v) => {
                write!(f, "0x")?;
                for b in v {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
            Value::Struct(v) => {
                let mut debug = f.debug_struct("");
                for (name, value) in v {
                    debug.field(name, value);
                }
                debug.finish()
            }
        }
    }
}

macro_rules! impl_accessors {
    ($(
        $variant:ident => $ty:ty, $get:ident, $get_mut:ident
    );* $(;)?) => {
        impl Value {
            $(
                pub fn $get(&self) -> &$ty {
                    match self {
                        Self::$variant(v) => v,
                        _ => panic!(),
                    }
                }

                pub fn $get_mut(&mut self) -> &mut $ty {
                    match self {
                        Self::$variant(v) => v,
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
    Point3  => Point3,     as_point3,  as_point3_mut;
    Guid    => Uuid,       as_guid,    as_guid_mut;
    Address => SocketAddr, as_address, as_address_mut;
    String  => Vec<u16>,   as_string,  as_string_mut;
    CString => String,     as_cstring, as_cstring_mut;
}

impl Value {
    pub fn as_slice(&self) -> &[Value] {
        match self {
            Self::ArrayFixed(v) => v.as_slice(),
            Self::ArrayVarSmall(v) => v.as_slice(),
            Self::ArrayVarLarge(v) => v.as_slice(),
            _ => panic!(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [Value] {
        match self {
            Self::ArrayFixed(v) => v.as_mut_slice(),
            Self::ArrayVarSmall(v) => v.as_mut_slice(),
            Self::ArrayVarLarge(v) => v.as_mut_slice(),
            _ => panic!(),
        }
    }
}

impl Index<&str> for Message {
    type Output = Value;

    fn index(&self, name: &str) -> &Self::Output {
        self.fields.get(name).unwrap()
    }
}

impl IndexMut<&str> for Message {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        self.fields.get_mut(name).unwrap()
    }
}

impl Index<&str> for Value {
    type Output = Value;

    fn index(&self, name: &str) -> &Self::Output {
        match self {
            Self::Struct(v) => v.get(name).unwrap(),
            _ => panic!(),
        }
    }
}

impl IndexMut<&str> for Value {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        match self {
            Self::Struct(v) => v.get_mut(name).unwrap(),
            _ => panic!(),
        }
    }
}

impl Encode for Message {
    fn encode(&self, buf: &mut BytesMut) -> io::Result<()> {
        self.id.encode(buf)?;
        self.fields.values().try_for_each(|f| f.encode(buf))
    }
}

impl Encode for Value {
    fn encode(&self, buf: &mut BytesMut) -> io::Result<()> {
        match self {
            Value::Byte(v) => v.encode(buf),
            Value::Word(v) => v.encode(buf),
            Value::Dword(v) => v.encode(buf),
            Value::Qword(v) => v.encode(buf),
            Value::Float(v) => v.encode(buf),
            Value::Float2(v) => v.encode(buf),
            Value::Float3(v) => v.encode(buf),
            Value::Float4(v) => v.encode(buf),
            Value::Guid(v) => v.encode(buf),
            Value::Point3(v) => v.encode(buf),
            Value::Address(v) => v.encode(buf),
            Value::String(v) => v.iter().try_for_each(|&v| v.encode(buf)),
            Value::CString(v) => {
                buf.put_slice(v.as_bytes());
                0u8.encode(buf)
            }
            Value::Optional(v) => match v {
                None => 0u8.encode(buf),
                Some(value) => {
                    1u8.encode(buf)?;
                    value.encode(buf)
                }
            },
            Value::ArrayFixed(v) => v.iter().try_for_each(|v| v.encode(buf)),
            Value::ArrayVarSmall(v) => {
                (v.len() as u8).encode(buf)?;
                v.iter().try_for_each(|v| v.encode(buf))
            }
            Value::ArrayVarLarge(v) => {
                (v.len() as u16).encode(buf)?;
                v.iter().try_for_each(|v| v.encode(buf))
            }
            Value::BufferFixed(v) => {
                buf.put_slice(v);
                Ok(())
            }
            Value::BufferVarSmall(v) => {
                (v.len() as u8).encode(buf)?;
                buf.put_slice(v);
                Ok(())
            }
            Value::BufferVarLarge(v) => {
                (v.len() as u16).encode(buf)?;
                buf.put_slice(v);
                Ok(())
            }
            Value::Struct(v) => v.values().try_for_each(|v| v.encode(buf)),
        }
    }
}

pub fn decode(
    protocols: &[ascalon_protocol_schema::Protocol],
    protocol: &str,
    server: bool,
    mut buf: &mut &[u8],
) -> io::Result<Message> {
    let id = u16::decode(buf)?;

    let protocol = protocols
        .iter()
        .find(|p| p.name == protocol)
        .ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
    let (message_prefix, message) = protocol
        .msgs
        .iter()
        .find_map(|msgs| {
            if server { &msgs.server } else { &msgs.client }
                .iter()
                .find(|m| m.id == id)
                .map(|m| (&msgs.name, m))
        })
        .ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
    let fields = decode_fields(&message.fields, &mut buf)?;

    Ok(Message {
        id,
        name: format!(
            "{}::{}::{}::{}",
            protocol.name,
            if server { "Server" } else { "Client" },
            message_prefix,
            message.name.clone()
        ),
        fields,
    })
}

fn decode_fields(fields: &[Field], buf: &mut &[u8]) -> io::Result<IndexMap<String, Value>> {
    fields
        .iter()
        .map(|f| Ok((f.name.clone(), decode_value(&f.r#type, Some(f), buf)?)))
        .collect()
}

fn decode_value(r#type: &str, field: Option<&Field>, buf: &mut &[u8]) -> io::Result<Value> {
    Ok(match r#type {
        "Byte" => Value::Byte(Decode::decode(buf)?),
        "Word" => Value::Word(Decode::decode(buf)?),
        "Dword" => Value::Dword(Decode::decode(buf)?),
        "Qword" => Value::Qword(Decode::decode(buf)?),
        "Float" => Value::Float(Decode::decode(buf)?),
        "Float2" => Value::Float2(Decode::decode(buf)?),
        "Float3" => Value::Float3(Decode::decode(buf)?),
        "Float4" => Value::Float4(Decode::decode(buf)?),
        "Guid" => Value::Guid(Decode::decode(buf)?),
        "Point3" => Value::Point3(Decode::decode(buf)?),
        "Address" => Value::Address(Decode::decode(buf)?),
        "String" => {
            let field = field.unwrap();

            let mut value = Vec::new();
            for _ in 0..field.size {
                let word = u16::decode(buf)?;
                value.push(word);

                if word == 0 {
                    return Ok(Value::String(value));
                }
            }

            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        "CString" => {
            let field = field.unwrap();

            let mut value = Vec::new();
            for _ in 0..field.size {
                let byte = u8::decode(buf)?;

                if byte == 0 {
                    return Ok(Value::CString(unsafe {
                        String::from_utf8_unchecked(value)
                    }));
                }

                if !byte.is_ascii() {
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }

                value.push(byte);
            }

            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        "Optional" => {
            let field = field.unwrap();

            Value::Optional(match u8::decode(buf)? {
                0 => None,
                1 => Some(Box::new(decode_value_inner(field, buf)?)),
                _ => return Err(io::Error::from(io::ErrorKind::InvalidData)),
            })
        }
        "ArrayFixed" => {
            let field = field.unwrap();

            let len = field.size;

            Value::ArrayFixed(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "ArrayVarSmall" => {
            let field = field.unwrap();

            let len = u8::decode(buf)? as usize;
            if len > field.size {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }

            Value::ArrayVarSmall(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "ArrayVarLarge" => {
            let field = field.unwrap();

            let len = u16::decode(buf)? as usize;
            if len > field.size {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }

            Value::ArrayVarLarge(
                (0..len)
                    .map(|_| decode_value_inner(field, buf))
                    .collect::<Result<_, _>>()?,
            )
        }
        "BufferFixed" => {
            let field = field.unwrap();

            let len = field.size;

            let value = Vec::<u8>::from(
                buf.get(..len)
                    .ok_or(io::Error::from(io::ErrorKind::UnexpectedEof))?,
            );
            *buf = &buf[len..];

            Value::BufferFixed(value)
        }
        "BufferVarSmall" => {
            let field = field.unwrap();

            let len = u8::decode(buf)? as usize;
            if len > field.size {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }

            let value = Vec::<u8>::from(
                buf.get(..len)
                    .ok_or(io::Error::from(io::ErrorKind::UnexpectedEof))?,
            );
            *buf = &buf[len..];

            Value::BufferVarSmall(value)
        }
        "BufferVarLarge" => {
            let field = field.unwrap();

            let len = u16::decode(buf)? as usize;
            if len > field.size {
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }

            let value = Vec::<u8>::from(
                buf.get(..len)
                    .ok_or(io::Error::from(io::ErrorKind::UnexpectedEof))?,
            );
            *buf = &buf[len..];

            Value::BufferVarLarge(value)
        }
        _ => panic!(),
    })
}

fn decode_value_inner(field: &Field, buf: &mut &[u8]) -> io::Result<Value> {
    if !field.fields.is_empty() {
        return Ok(Value::Struct(decode_fields(&field.fields, buf)?));
    }

    decode_value(
        field
            .type_name
            .as_deref()
            .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?,
        None,
        buf,
    )
}

fn decode_coded(words: &mut &[u16]) -> io::Result<String> {
    let mut output = {
        let mut strings = STRINGS.write().unwrap();
        let string = strings
            .get_mut(decode_coded_numeric(words)? as usize)
            .ok_or(io::ErrorKind::InvalidData)?;
        if words[0] & 0x8000 != 0 {
            string.decrypt(decode_coded_numeric(words)?)?;
        }
        string.text()?
    };

    loop {
        let (word, _words) = words.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        *words = _words;

        match word {
            0x0000 | 0x0001 => return Ok(output),
            0x0002 => {
                output.push_str(&decode_coded(words)?);
                return Ok(output);
            }
            0x0003 => {
                output.push_str(&decode_coded_literal(words)?);
                return Ok(output);
            }
            0x0101..=0x0106 => {
                output = output.replace(
                    &format!("%num{}%", (word - 0x0101) + 1),
                    &decode_coded_numeric(words)?.to_string(),
                );
            }
            0x0107..=0x010c => {
                output = output.replace(
                    &format!("%str{}%", (word - 0x0107) + 1),
                    &decode_coded_literal(words)?,
                );
            }
            0x010d..=0x0112 => {
                output = output.replace(
                    &format!("%str{}%", (word - 0x010d) + 1),
                    &decode_coded(words)?,
                )
            }
            _ => {}
        }
    }
}

fn decode_coded_numeric(words: &mut &[u16]) -> io::Result<u64> {
    let mut value = 0u64;

    loop {
        let (word, _words) = words.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        *words = _words;

        let part = word & 0x7fff;
        if part < 0x0100 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        value += (part - 0x0100) as u64;

        if word & 0x8000 == 0 {
            return Ok(value);
        }

        value *= 0x7f00;
    }
}

fn decode_coded_literal(words: &mut &[u16]) -> io::Result<String> {
    let mut value = Vec::new();

    loop {
        let (word, _words) = words.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        *words = _words;

        if *word == 0x0001 {
            return String::from_utf16(&value).map_err(|_| io::ErrorKind::InvalidData.into());
        }

        value.push(*word);
    }
}
