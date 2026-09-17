use std::io::{Error, ErrorKind, Read as _, Result};
use std::marker::PhantomData;

use zerocopy::little_endian::{F32, U16, U32, U64};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, NativeEndian, Usize};

pub type Byte = u8;
pub type Byte3 = [u8; 3];
pub type Byte4 = [u8; 4];
pub type Word = U16;
pub type Word3 = [U16; 3];
pub type Dword = U32;
pub type Dword2 = [U32; 2];
pub type Dword4 = [U32; 4];
pub type Token = U64;
pub type Float = F32;
pub type Float2 = [F32; 2];
pub type Float3 = [F32; 3];
pub type Float4 = [F32; 4];
pub type FileName = FileRef;

pub struct Packfile(Vec<u8>);

impl Packfile {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let header = PackfileHeader::ref_from_prefix(&bytes).unwrap().0;
        if header.magic != *b"PF" {
            return Err(Error::new(ErrorKind::InvalidData, "invalid magic"));
        }

        Ok(Self(bytes).fixed()?)
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

impl Packfile {
    fn fixed(self) -> Result<Self> {
        let src_ptr_width = if self.header().flags.get() & 4 != 0 {
            8
        } else {
            4
        };
        let dst_ptr_width = size_of::<usize>();
        let ptr_width_delta = dst_ptr_width as isize - src_ptr_width as isize;

        let mut size = 0x10;
        for chunk in self.chunks() {
            let fixup_count = if chunk.header().data_size.get() == 0 {
                0
            } else {
                chunk.extra_bytes().read_le::<u32>()? as usize
            };

            size += chunk.header().header_size.get() as usize
                + (chunk.bytes().len() as isize + fixup_count as isize * ptr_width_delta) as usize;
        }

        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&self.0[..self.header().header_size.get() as usize]);
        bytes.resize(0x10, 0);
        PackfileHeader::mut_from_prefix(&mut bytes)
            .unwrap()
            .0
            .header_size
            .set(0x10);

        for chunk in self.chunks() {
            let chunk_bytes = chunk.bytes();
            let mut chunk_extra_bytes = chunk.extra_bytes();

            let fixup_count = if chunk.header().data_size.get() == 0 {
                0
            } else {
                chunk_extra_bytes.read_le::<u32>()? as usize
            };
            let fixups = <[Dword]>::ref_from_bytes(&chunk_extra_bytes[..fixup_count * 4]).unwrap();

            let chunk_start = bytes.len();
            bytes.extend_from_slice(&chunk.0[..chunk.header().header_size.get() as usize]);
            let chunk_bytes_start = bytes.len();

            let mut pos = 0;
            for &fixup in fixups {
                let fixup = fixup.get() as usize;
                bytes.extend_from_slice(&chunk_bytes[pos..fixup]);

                let offset = match src_ptr_width {
                    4 => (&chunk_bytes[fixup..]).read_le::<i32>()? as isize,
                    8 => (&chunk_bytes[fixup..]).read_le::<i64>()? as isize,
                    _ => unreachable!(),
                };

                let ptr = if offset == 0 {
                    std::ptr::null_mut::<u8>()
                } else {
                    let offset = fixup as isize + offset;
                    let mut start = 0;
                    let mut end = fixups.len();

                    while start < end {
                        let middle = start + (end - start) / 2;
                        let position = fixups[middle].get() as isize;

                        if position < offset {
                            start = middle + 1;
                        } else {
                            end = middle;
                        }
                    }

                    let offset = (offset + start as isize * ptr_width_delta) as usize;

                    unsafe { bytes.as_mut_ptr().add(chunk_bytes_start + offset) }
                };

                let offset_pos = bytes.len();
                bytes.resize(offset_pos + dst_ptr_width, 0);

                unsafe {
                    bytes
                        .as_mut_ptr()
                        .add(offset_pos)
                        .cast::<*mut u8>()
                        .write_unaligned(ptr);
                }

                pos = fixup + src_ptr_width;
            }

            bytes.extend_from_slice(&chunk_bytes[pos..]);

            let chunk_size = bytes.len() - chunk_start;
            let header = PackfileChunkHeader::mut_from_prefix(&mut bytes[chunk_start..])
                .unwrap()
                .0;

            header.data_size.set(0);
            header.size.set((chunk_size - 8) as _);
        }

        Ok(Self(bytes))
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
        let bytes = self.0.split_at(header.size.get() as usize + 8);
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
        let bytes = &self.0[self.header().header_size.get() as usize..];
        match self.header().data_size.get() {
            0 => bytes,
            n => &bytes[..n as usize],
        }
    }

    pub fn extra_bytes(&self) -> &'a [u8] {
        match self.header().data_size.get() {
            0 => &[],
            n => &self.0[self.header().header_size.get() as usize + n as usize..],
        }
    }
}

#[derive(IntoBytes, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileHeader {
    magic:       [u8; 2],
    flags:       Word,
    _04:         Word,
    header_size: Word,
    r#type:      [u8; 4],
}

#[derive(IntoBytes, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct PackfileChunkHeader {
    name:        [u8; 4],
    size:        Dword,
    version:     Word,
    header_size: Word,
    data_size:   Dword,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Guid(Dword, Word, Word, [u8; 8]);

impl Guid {
    pub const fn new(a: u32, b: u16, c: u16, d: [u8; 8]) -> Self {
        Self(Dword::new(a), Word::new(b), Word::new(c), d)
    }
}

impl std::fmt::Debug for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0,
            self.1,
            self.2,
            self.3[0],
            self.3[1],
            self.3[2],
            self.3[3],
            self.3[4],
            self.3[5],
            self.3[6],
            self.3[7],
        )
    }
}

#[repr(C)]
pub struct Ptr<T> {
    ptr:      Usize<NativeEndian>,
    _phantom: PhantomData<T>,
}

impl<T> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.ptr.get() as *const T
    }

    pub unsafe fn as_ref(&self) -> Option<&T> {
        unsafe { self.as_ptr().as_ref() }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.as_ref() }.fmt(f)
    }
}

#[repr(C)]
pub struct ArrayPtr<T> {
    len:      Dword,
    ptr:      Usize<NativeEndian>,
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

        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len.get() as usize) }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ArrayPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.as_slice() }.fmt(f)
    }
}

#[repr(transparent)]
pub struct CharPtr(Usize<NativeEndian>);

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
}

impl std::fmt::Debug for CharPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from_utf8_lossy(unsafe { self.as_slice() }).fmt(f)
    }
}

#[repr(transparent)]
pub struct WcharPtr(Usize<NativeEndian>);

impl WcharPtr {
    pub fn as_ptr(&self) -> *const U16 {
        self.0.get() as *const U16
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

    pub unsafe fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        if ptr.is_null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr.cast(), self.len() * 2) }
    }

    pub unsafe fn as_slice(&self) -> &[U16] {
        let ptr = self.as_ptr();
        if ptr.is_null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr, self.len()) }
    }
}

impl std::fmt::Debug for WcharPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::from_utf16le_lossy(unsafe { self.as_bytes() }).fmt(f)
    }
}

#[repr(transparent)]
pub struct FileRef(pub WcharPtr);

impl FileRef {
    pub unsafe fn file_id(&self) -> Option<u32> {
        let [a, b, ..] = (unsafe { self.0.as_slice() }) else {
            return None;
        };

        let (a, b) = (a.get(), b.get());
        if a <= 0xFF || b <= 0xFF {
            return None;
        }

        Some((u32::from(a) - 0xFF) + (u32::from(b) - 0x100) * 0xFF00)
    }
}

impl std::fmt::Debug for FileRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FileRef")
            .field(&unsafe { self.file_id() })
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Token32(U32);

impl Token32 {
    const ALPHABET: &[u8] = b"abcdefghiklmnopvrstuwxy";

    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut i = bytes.len();

        let mut value = 0u32;
        while i > 0 {
            i -= 1;

            let mut digit = 0;
            while digit < Self::ALPHABET.len() && Self::ALPHABET[digit] != bytes[i] {
                digit += 1;
            }

            value = value * Self::ALPHABET.len() as u32 + digit as u32;
        }

        Self(U32::new(value + 0x3000_0000))
    }
}

impl std::fmt::Debug for Token32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        let mut value = self.0.get() - 0x3000_0000;
        while value != 0 {
            out.push(Self::ALPHABET[(value % Self::ALPHABET.len() as u32) as usize] as char);
            value /= Self::ALPHABET.len() as u32;
        }

        out.fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Token64(U64);

impl Token64 {
    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut i = 0;

        let mut value = 0u64;
        while i < bytes.len() && i < 12 {
            value |= match bytes[i] {
                b'a'..=b'z' => (bytes[i] - b'`') as u64,
                _ => 0,
            } << (i * 5);
            i += 1;
        }

        let mut suffix = 0u64;
        while i < bytes.len() {
            suffix = suffix * 10 + (bytes[i] - b'0') as u64;
            i += 1;
        }

        Self(U64::new(value | (suffix << 60)))
    }
}

impl std::fmt::Debug for Token64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        let mut value = self.0.get() & 0x0FFF_FFFF_FFFF_FFFF;
        while value != 0 {
            let v = (value & 0x1F) as u8;
            out.push(if v == 0 { ' ' } else { (v + b'`') as char });
            value >>= 5;
        }

        let suffix = (self.0.get() >> 60) as u8;
        if suffix != 0 {
            out.push((b'0' + suffix / 10) as char);
            out.push((b'0' + suffix % 10) as char);
        }

        out.fmt(f)
    }
}

include!(concat!(env!("OUT_DIR"), "/packfiles.rs"));
