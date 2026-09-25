use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Read, Result},
    marker::PhantomData,
};

use zerocopy::{
    F32, FromBytes, Immutable, IntoBytes, KnownLayout, LittleEndian, NativeEndian, U16, U32, U64,
    Usize,
};

type Byte = u8;
type Byte3 = [u8; 3];
type Byte4 = [u8; 4];
type Word = U16<LittleEndian>;
type Word3 = [U16<LittleEndian>; 3];
type Dword = U32<LittleEndian>;
type Dword2 = [U32<LittleEndian>; 2];
type Dword4 = [U32<LittleEndian>; 4];
type Qword = U64<LittleEndian>;
type Float = F32<LittleEndian>;
type Float2 = [F32<LittleEndian>; 2];
type Float3 = [F32<LittleEndian>; 3];
type Float4 = [F32<LittleEndian>; 4];

pub struct Packfile(Vec<u8>);

impl Packfile {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let header = PackfileHeader::ref_from_prefix(&bytes).unwrap().0;
        if header.magic != *b"PF" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "packfile: invalid magic",
            ));
        }

        let header_size = header.header_size.get() as usize;

        let src_ptr_width = if header.flags.get() & 4 != 0 { 8 } else { 4 };
        let dst_ptr_width = size_of::<usize>();
        let ptr_width_delta = dst_ptr_width as isize - src_ptr_width as isize;

        let mut dst = bytes[..header_size].to_vec();

        let mut src = &bytes[header_size..];
        while !src.is_empty() {
            let header = PackfileChunkHeader::ref_from_prefix(src).unwrap().0;
            let header_size = header.header_size.get() as usize;

            let bytes = src.split_at(header.next_chunk_offset.get() as usize + 8);
            src = bytes.1;

            let dst_chunk = dst.len();
            dst.extend_from_slice(&bytes.0[..header_size]);

            let bytes = &bytes.0[header_size..];
            let (data, mut fixups) = bytes.split_at(header.fixups_offset.get() as usize);

            let mut pos = 0;

            let fixup_count = fixups.read_le::<u32>()? as usize;
            for _ in 0..fixup_count {
                let fixup = fixups.read_le::<u32>()? as usize;

                dst.extend_from_slice(&data[pos..fixup]);
                dst.resize(dst.len() + dst_ptr_width, 0);

                pos = fixup + src_ptr_width;
            }

            dst.extend_from_slice(&data[pos..]);

            let header = PackfileChunkHeader::mut_from_prefix(&mut dst[dst_chunk..])
                .unwrap()
                .0;
            header
                .fixups_offset
                .set((data.len() as isize + fixup_count as isize * ptr_width_delta) as _);
            header
                .next_chunk_offset
                .set((header.fixups_offset.get() as usize + header_size - 8) as _);
        }

        let dst_ptr = dst.as_ptr() as usize;
        let mut dst_pos = header_size;

        let mut pos = header_size;
        while pos < bytes.len() {
            let header = PackfileChunkHeader::ref_from_prefix(&bytes[pos..])
                .unwrap()
                .0;
            let header_size = header.header_size.get() as usize;
            let length = header.next_chunk_offset.get() as usize + 8;

            let dst_header = PackfileChunkHeader::ref_from_prefix(&dst[dst_pos..])
                .unwrap()
                .0;
            let dst_length = dst_header.next_chunk_offset.get() as usize + 8;

            let bytes = &bytes[pos + header_size..pos + length];
            let (data, mut _fixups) = bytes.split_at(header.fixups_offset.get() as usize);

            let fixup_count = _fixups.read_le::<u32>()? as usize;
            let mut fixups = Vec::with_capacity(fixup_count);
            for _ in 0..fixup_count {
                fixups.push(_fixups.read_le::<u32>()? as usize);
            }

            let dst_data = dst_pos + header_size;

            for (i, &fixup) in fixups.iter().enumerate() {
                let offset = match src_ptr_width {
                    4 => (&data[fixup..]).read_le::<i32>()? as isize,
                    8 => (&data[fixup..]).read_le::<i64>()? as isize,
                    _ => unreachable!(),
                };

                let target = fixup as isize + offset;
                let dst_offset = if offset > 0 {
                    let end = fixups.partition_point(|&f| (f as isize) < target);
                    offset + (end - i) as isize * ptr_width_delta
                } else if offset < 0 {
                    let start = fixups.partition_point(|&f| (f as isize) < target);
                    offset - (i - start) as isize * ptr_width_delta
                } else {
                    0
                };
                let dst_fixup = dst_data + (fixup as isize + i as isize * ptr_width_delta) as usize;

                let ptr = if offset == 0 {
                    0
                } else {
                    (dst_ptr as isize + dst_fixup as isize + dst_offset) as usize
                };

                match dst_ptr_width {
                    4 => dst[dst_fixup..dst_fixup + 4].copy_from_slice(&(ptr as u32).to_ne_bytes()),
                    8 => dst[dst_fixup..dst_fixup + 8].copy_from_slice(&(ptr as u64).to_ne_bytes()),
                    _ => unreachable!(),
                }
            }

            pos += length;
            dst_pos += dst_length;
        }

        Ok(Self(dst))
    }

    fn header(&self) -> &PackfileHeader {
        PackfileHeader::ref_from_prefix(&self.0).unwrap().0
    }

    pub fn r#type(&self) -> [u8; 4] {
        self.header().r#type
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

    pub fn name(&self) -> [u8; 4] {
        self.header().name
    }

    pub fn version(&self) -> u16 {
        self.header().version.get()
    }

    pub fn bytes(&self) -> &'a [u8] {
        &self.0[self.header().header_size.get() as usize..]
            [..self.header().fixups_offset.get() as usize]
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileHeader {
    magic: [u8; 2],
    flags: Word,
    _04: Word,
    header_size: Word,
    r#type: [u8; 4],
}

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileChunkHeader {
    name: [u8; 4],
    next_chunk_offset: Dword,
    version: Word,
    header_size: Word,
    fixups_offset: Dword,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct Ptr<T> {
    ptr: zerocopy::Usize<NativeEndian>,
    _marker: PhantomData<T>,
}

impl<T> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.ptr.get() as *const T
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
    ptr: Usize<NativeEndian>,
    _phantom: PhantomData<T>,
}

impl<T> ArrayPtr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.ptr.get() as *const T
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        if self.as_ptr().is_null() {
            return &[];
        }

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
pub struct CharPtr(zerocopy::Usize<NativeEndian>);

impl CharPtr {
    pub fn as_ptr(&self) -> *const u8 {
        self.0.get() as *const u8
    }

    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.as_ptr();
        if ptr.is_null() {
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
        if ptr.is_null() {
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
pub struct WcharPtr(zerocopy::Usize<NativeEndian>);

impl WcharPtr {
    pub fn as_ptr(&self) -> *const u16 {
        self.0.get() as *const u16
    }

    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.as_ptr();
        if ptr.is_null() {
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
        if ptr.is_null() {
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
