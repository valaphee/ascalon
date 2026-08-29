use std::{collections::HashMap, fs::OpenOptions, ops::Range, path::Path};

use memmap2::Mmap;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::inflate::inflate;

const FLAG_ENTRY_USED: u8 = 1 << 0;
const FLAG_FIRST_STREAM: u8 = 1 << 1;

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, KnownLayout, Immutable)]
struct AnHeader {
    version: u8,
    magic: [u8; 3],
    _2: u32,
    _3: u32,
    _4: u32,
    _5: u32,
    _6: u32,
    mft_offset: u64,
    mft_size: u32,
    _9: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, KnownLayout, Immutable)]
struct MftHeader {
    magic: [u8; 4],
    _1: u32,
    _2: u32,
    _3: u32,
    _4: u32,
    _5: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, KnownLayout, Immutable)]
struct MftEntry {
    offset: u64,
    size: u32,
    extra_bytes: u16,
    alloc_flags: u8,
    _4: u8,
    _5: u32,
    _6: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, KnownLayout, Immutable)]
struct IndexEntry {
    file_id: u32,
    mft_index: u32,
}

pub struct Archive {
    mmap: Mmap,
    mft: Range<usize>,
    index: HashMap<u32, u32>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Self {
        let file = OpenOptions::new().read(true).open(path).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        let an_header = AnHeader::ref_from_prefix(&mmap[..]).unwrap().0;
        assert_eq!(an_header.version, 151);
        assert_eq!(&an_header.magic, b"AN\x1A");

        let mft = an_header.mft_offset as usize
            ..an_header.mft_offset as usize + an_header.mft_size as usize;
        let (mft_header, mft_entries) = MftHeader::ref_from_prefix(&mmap[mft.clone()]).unwrap();
        assert_eq!(&mft_header.magic, b"Mft\x1A");

        let mft_entries = <[MftEntry]>::ref_from_bytes(mft_entries).unwrap();
        let [an_entry, index_entry, mft_entry, ..] = mft_entries else {
            panic!();
        };

        assert_eq!(an_entry.offset, 0);
        assert_eq!(an_entry.size, 40);
        assert_eq!(an_entry.extra_bytes, 0);
        assert_eq!(an_entry.alloc_flags, FLAG_ENTRY_USED | FLAG_FIRST_STREAM);

        assert_eq!(index_entry.extra_bytes as u16, 0);
        assert_eq!(index_entry.alloc_flags, FLAG_ENTRY_USED | FLAG_FIRST_STREAM);

        assert_eq!(an_header.mft_offset as u64, mft_entry.offset as u64);
        assert_eq!(an_header.mft_size as u32, mft_entry.size as u32);
        assert_eq!(mft_entry.extra_bytes, 0);
        assert_eq!(mft_entry.alloc_flags, FLAG_ENTRY_USED | FLAG_FIRST_STREAM);

        let index = <[IndexEntry]>::ref_from_bytes(
            &mmap[index_entry.offset as usize..][..index_entry.size as usize],
        )
        .unwrap();

        let index = index
            .iter()
            .filter(|entry| entry.file_id != 0 && entry.mft_index != 0)
            .map(|entry| (entry.file_id, entry.mft_index as u32 - 1))
            .collect();

        Self { mmap, mft, index }
    }

    pub fn get(&self, file_id: u32) -> Option<Vec<u8>> {
        let mft_index = *self.index.get(&file_id)?;

        let mft_entries =
            <[MftEntry]>::ref_from_bytes(&self.mmap[self.mft.clone()][size_of::<MftHeader>()..])
                .unwrap();
        let mft_entry = &mft_entries[mft_index as usize];
        if mft_entry.extra_bytes != 8 {
            return None;
        }

        Some(inflate(&self.mmap[mft_entry.offset as usize..][..mft_entry.size as usize]).unwrap())
    }
}
