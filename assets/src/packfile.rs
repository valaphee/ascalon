use std::marker::PhantomData;

use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    little_endian::{U16, U32, U64},
};

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct PackfileHeader {
    pub magic: [u8; 2],
    _1: U16,
    _2: U16,
    pub header_size: U16,
    pub file_type: [u8; 4],
}

pub struct Packfile<'a>(&'a [u8]);

impl<'a> Packfile<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    pub fn header(&self) -> &'a PackfileHeader {
        PackfileHeader::ref_from_prefix(self.0).unwrap().0
    }

    pub fn chunks(&self) -> PackfileChunks<'a> {
        PackfileChunks(&self.0[self.header().header_size.get() as usize..])
    }
}

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct PackfileChunkHeader {
    pub magic: [u8; 4],
    next_chunk_offset: U32,
    pub version: U16,
    pub header_size: U16,
    relocation_offset: U32,
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
    pub fn header(&self) -> &'a PackfileChunkHeader {
        PackfileChunkHeader::ref_from_prefix(self.0).unwrap().0
    }

    pub fn data(&self) -> &'a [u8] {
        &self.0[self.header().header_size.get() as usize..]
            [..self.header().relocation_offset.get() as usize]
    }
}

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
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

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct Ptr<T> {
    offset: U64,
    _phantom: PhantomData<T>,
}

impl<T> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        unsafe {
            (std::ptr::addr_of!(self.offset) as *const u8)
                .add(self.offset.get() as usize)
                .cast()
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.as_ptr() }
    }
}

#[repr(C)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct WString {
    offset: U64,
    _marker: PhantomData<*const u16>,
}

impl WString {
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
        let words = self.as_slice();
        let lo_word = words[0] as u32;
        let hi_word = words[1] as u32;
        if hi_word >= 0x100 && lo_word >= 0x100 {
            (hi_word - 0x100) * 0xFF00 + (lo_word - 0xFF)
        } else {
            0
        }
    }
}
