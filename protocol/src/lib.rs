#![feature(array_try_from_fn)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]

use std::fmt::Debug;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

pub trait Encode {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()>;
}

pub trait Decode: Sized {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()>;
}

impl Encode for u8 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        buf.put_u8(*self);

        Ok(())
    }
}

impl Decode for u8 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        buf.try_get_u8().map_err(|_| ())
    }
}

impl Encode for u16 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        buf.put_u16_le(*self);

        Ok(())
    }
}

impl Decode for u16 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        buf.try_get_u16_le().map_err(|_| ())
    }
}

impl Encode for u32 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        let mut value = *self;
        while value >= 0x80 {
            buf.put_u8((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        buf.put_u8(value as u8);

        Ok(())
    }
}

impl Decode for u32 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let byte = u8::decode(buf)?;
            value |= ((byte & 0x7f) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }

        Err(())
    }
}

impl Encode for u64 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        buf.put_u64_le(*self);

        Ok(())
    }
}

impl Decode for u64 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        buf.try_get_u64_le().map_err(|_| ())
    }
}

impl Encode for f32 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        buf.put_f32_le(*self);

        Ok(())
    }
}

impl Decode for f32 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        buf.try_get_f32_le().map_err(|_| ())
    }
}

#[derive(Debug)]
pub struct Point3([f32; 3], u32);

impl Encode for Point3 {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        self.0.encode(buf)?;
        self.1.encode(buf)
    }
}

impl Decode for Point3 {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        Ok(Point3(Decode::decode(buf)?, Decode::decode(buf)?))
    }
}

impl Encode for Uuid {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        let (d1, d2, d3, d4) = self.as_fields();
        buf.put_u32_le(d1);
        buf.put_u16_le(d2);
        buf.put_u16_le(d3);
        buf.put_slice(d4);

        Ok(())
    }
}

impl Decode for Uuid {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        if buf.len() < 16 {
            return Err(());
        }

        Ok(Uuid::from_fields(
            buf.get_u32_le(),
            buf.get_u16_le(),
            buf.get_u16_le(),
            &buf.get_u64().to_be_bytes(),
        ))
    }
}

impl Encode for SocketAddr {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        match self {
            SocketAddr::V4(addr) => {
                buf.put_u16_le(2);
                buf.put_u16_le(addr.port());
                buf.put_slice(&addr.ip().octets());
                buf.put_bytes(0, 20);
            }
            SocketAddr::V6(addr) => {
                buf.put_u16_le(23);
                buf.put_u16_le(addr.port());
                buf.put_u32_le(addr.flowinfo());
                buf.put_slice(&addr.ip().octets());
                buf.put_u32_le(addr.scope_id());
            }
        }

        Ok(())
    }
}

impl Decode for SocketAddr {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        if buf.len() < 28 {
            return Err(());
        }

        match buf.get_u16_le() {
            2 => {
                let port = buf.get_u16_le();
                let ip = Ipv4Addr::from(buf.get_u32().to_be_bytes());

                buf.advance(20);

                Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
            }
            23 => {
                let port = buf.get_u16_le();
                let flowinfo = buf.get_u32_le();
                let ip = Ipv6Addr::from(buf.get_u128().to_be_bytes());
                let scope_id = buf.get_u32_le();

                Ok(SocketAddr::V6(SocketAddrV6::new(
                    ip, port, flowinfo, scope_id,
                )))
            }
            _ => Err(()),
        }
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        match self {
            None => 0u8.encode(buf),
            Some(value) => {
                1u8.encode(buf)?;
                value.encode(buf)
            }
        }
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        match u8::decode(buf)? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(buf)?)),
            _ => Err(()),
        }
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, buf: &mut BytesMut) -> Result<(), ()> {
        for value in self {
            value.encode(buf)?;
        }

        Ok(())
    }
}

impl<T: Decode, const N: usize> Decode for [T; N] {
    fn decode(buf: &mut &[u8]) -> Result<Self, ()> {
        std::array::try_from_fn(|_| T::decode(buf))
    }
}

include!(concat!(env!("OUT_DIR"), "/protocols.rs"));
