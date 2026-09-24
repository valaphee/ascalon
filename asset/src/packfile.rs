use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
    ptr,
};

use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    little_endian::{F32 as Float, U16 as Word, U32 as Dword, U64 as Qword},
};

type Byte = u8;
type Byte3 = [u8; 3];
type Byte4 = [u8; 4];
type Word3 = [Word; 3];
type Dword2 = [Dword; 2];
type Dword4 = [Dword; 4];
type Float2 = [Float; 2];
type Float3 = [Float; 3];
type Float4 = [Float; 4];

pub struct Packfile(Vec<u8>);

impl Packfile {
    pub fn new(mut bytes: Vec<u8>) -> Result<Self> {
        let header = PackfileHeader::ref_from_prefix(&bytes).unwrap().0;
        if header.magic != *b"PF" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "packfile: invalid magic",
            ));
        }

        let mut offset = header.header_size.get() as usize;

        while offset < bytes.len() {
            let (length, header_size, data_end) = {
                let header = PackfileChunkHeader::ref_from_prefix(&bytes[offset..])
                    .unwrap()
                    .0;

                (
                    header.next_chunk_offset.get() as usize + 8,
                    header.header_size.get() as usize,
                    header._4.get() as usize,
                )
            };

            unsafe {
                let data = bytes.as_mut_ptr().add(offset + header_size);

                let mut fixup = data.add(data_end + 4).cast::<u32>();
                loop {
                    let reloc_offset = u32::from_le(fixup.read_unaligned()) as usize;
                    if reloc_offset == 0 {
                        break;
                    }

                    let value = data.add(reloc_offset).cast::<u64>();
                    let offset = u64::from_le(value.read_unaligned());
                    value.write_unaligned(value as u64 + offset);

                    fixup = fixup.add(1);
                }
            }

            offset += length;
        }

        Ok(Self(bytes))
    }

    fn header(&self) -> &PackfileHeader {
        PackfileHeader::ref_from_prefix(&self.0).unwrap().0
    }

    pub fn chunks(&self) -> PackfileChunks<'_> {
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

        let header = PackfileChunkHeader::ref_from_prefix(self.0).unwrap().0;
        let bytes = self.0.split_at(header.next_chunk_offset.get() as usize + 8);
        self.0 = bytes.1;

        Some(PackfileChunk(bytes.0))
    }
}

pub struct PackfileChunk<'a>(&'a [u8]);

impl<'a> PackfileChunk<'a> {
    fn header(&self) -> &'a PackfileChunkHeader {
        PackfileChunkHeader::ref_from_prefix(self.0).unwrap().0
    }

    pub fn bytes(&self) -> &'a [u8] {
        &self.0[self.header().header_size.get() as usize..][..self.header()._4.get() as usize]
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileHeader {
    magic: [u8; 2],
    _1: Word,
    _2: Word,
    header_size: Word,
    _4: [u8; 4],
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileChunkHeader {
    _0: [u8; 4],
    next_chunk_offset: Dword,
    _2: Word,
    header_size: Word,
    _4: Dword,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct Ptr<T> {
    offset: zerocopy::native_endian::U64,
    _marker: PhantomData<T>,
}

impl<T> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.offset.get() as usize as *const T
    }

    pub unsafe fn as_ref(&self) -> &T {
        unsafe { &*self.as_ptr() }
    }
}

impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.as_ref() }.fmt(f)
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct ArrayPtr<T> {
    length: Dword,
    offset: zerocopy::native_endian::U64,
    _phantom: PhantomData<T>,
}

impl<T> ArrayPtr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.offset.get() as usize as *const T
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.length.get() as usize) }
    }
}

impl<T: Debug> Debug for ArrayPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.as_slice() }.fmt(f)
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct CharPtr {
    offset: zerocopy::native_endian::U64,
}

impl CharPtr {
    pub fn as_ptr(&self) -> *const u8 {
        self.offset.get() as usize as *const u8
    }

    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.as_ptr();
        if ptr == ptr::null() {
            return 0;
        }

        unsafe {
            while ptr.read_unaligned() != 0 {
                ptr = ptr.add(1);
            }

            ptr.offset_from_unsigned(self.as_ptr())
        }
    }

    pub unsafe fn as_slice(&self) -> &[u8] {
        let ptr = self.as_ptr();
        if ptr == ptr::null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr, self.len()) }
    }

    pub unsafe fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(unsafe { self.as_slice() }).to_string()
    }
}

impl Debug for CharPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.to_string_lossy() }.fmt(f)
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct WcharPtr {
    offset: zerocopy::native_endian::U64,
}

impl WcharPtr {
    pub fn as_ptr(&self) -> *const u16 {
        self.offset.get() as usize as *const u16
    }

    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.as_ptr();
        if ptr == ptr::null() {
            return 0;
        }

        unsafe {
            while ptr.read_unaligned() != 0 {
                ptr = ptr.add(1);
            }

            ptr.offset_from_unsigned(self.as_ptr())
        }
    }

    pub unsafe fn as_slice(&self) -> &[u16] {
        let ptr = self.as_ptr();
        if ptr == ptr::null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr, self.len()) }
    }

    pub unsafe fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(unsafe { self.as_slice() })
    }
}

impl Debug for WcharPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.to_string_lossy() }.fmt(f)
    }
}

include!(concat!(env!("OUT_DIR"), "/packfiles.rs"));
