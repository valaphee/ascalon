use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
};

use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    little_endian::{U16, U32, U64},
};

pub mod txtm;

pub struct Packfile<'a>(&'a [u8]);

impl<'a> Packfile<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        if Self(bytes).header().magic != *b"PF" {
            return Err(Error::from(ErrorKind::InvalidData));
        }

        Ok(Self(bytes))
    }

    fn header(&self) -> &'a PackfileHeader {
        PackfileHeader::ref_from_prefix(self.0).unwrap().0
    }

    pub fn chunks(&self) -> PackfileChunks<'a> {
        PackfileChunks(&self.0[self.header().header_size.get() as usize..])
    }
}

pub struct PackfileChunks<'a>(&'a [u8]);

impl<'a> Iterator for PackfileChunks<'a> {
    type Item = PackfileChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let (header, _) = PackfileChunkHeader::ref_from_prefix(self.0).unwrap();
        let (bytes, remaining) = self.0.split_at(header.next_chunk_offset.get() as usize + 8);
        self.0 = remaining;

        Some(PackfileChunk(bytes))
    }
}

pub struct PackfileChunk<'a>(&'a [u8]);

impl<'a> PackfileChunk<'a> {
    fn header(&self) -> &'a PackfileChunkHeader {
        PackfileChunkHeader::ref_from_prefix(self.0).unwrap().0
    }

    pub fn data(&self) -> &'a [u8] {
        &self.0[self.header().header_size.get() as usize..][..self.header()._4.get() as usize]
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileHeader {
    magic: [u8; 2],
    _1: U16,
    _2: U16,
    header_size: U16,
    _4: [u8; 4],
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileChunkHeader {
    _0: [u8; 4],
    next_chunk_offset: U32,
    _2: U16,
    header_size: U16,
    _4: U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct ArrayPtr<T> {
    length: U32,
    offset: U64,
    _phantom: PhantomData<T>,
}

impl<T> ArrayPtr<T> {
    pub fn as_ptr(&self) -> *const T {
        unsafe {
            (std::ptr::addr_of!(self.offset) as *const u8)
                .add(self.offset.get() as usize)
                .cast()
        }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.length.get() as usize) }
    }
}

impl<T: Debug> Debug for ArrayPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct WcharPtr {
    offset: U64,
    _marker: PhantomData<*const u16>,
}

impl WcharPtr {
    pub fn as_ptr(&self) -> *const u16 {
        unsafe {
            (std::ptr::addr_of!(self.offset) as *const u8)
                .add(self.offset.get() as usize)
                .cast()
        }
    }

    pub fn len(&self) -> usize {
        let mut ptr = self.as_ptr();

        unsafe {
            while ptr.read_unaligned() != 0 {
                ptr = ptr.add(1);
            }

            ptr.offset_from_unsigned(self.as_ptr())
        }
    }

    pub fn as_slice(&self) -> &[u16] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(self.as_slice())
    }

    pub fn file_id(&self) -> u32 {
        let value = self.as_slice();
        return (value[0] as u32 - 0xff) + (value[1] as u32 - 0x100) * 0xff00;
    }
}

impl Debug for WcharPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string_lossy().fmt(f)
    }
}
