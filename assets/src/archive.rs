use std::{
    collections::HashMap,
    fs::File,
    io::{ErrorKind, Result},
    os::windows::fs::FileExt,
    path::Path,
};

use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    little_endian::{U16, U32, U64},
};

use crate::inflate::inflate;

pub struct Archive {
    file: File,
    index: HashMap<u32, MftEntry>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;

        let mut an_header = vec![0; size_of::<AnHeader>()];
        file.seek_read(&mut an_header, 0)?;

        let an_header = AnHeader::ref_from_bytes(&an_header).map_err(|_| ErrorKind::InvalidData)?;
        if an_header.magic != *b"AN\x1A" {
            return Err(ErrorKind::InvalidData.into());
        }

        let mut mft = vec![0; an_header.mft_size.get() as usize];
        file.seek_read(&mut mft, an_header.mft_offset.get())?;

        let (mft_header, mft_entries) =
            MftHeader::ref_from_prefix(&mft).map_err(|_| ErrorKind::InvalidData)?;
        if mft_header.magic != *b"Mft\x1A" {
            return Err(ErrorKind::InvalidData.into());
        }

        let mft_entries = <[MftEntry]>::ref_from_bytes_with_elems(
            mft_entries,
            mft_header.entry_count.get() as usize - 1,
        )
        .map_err(|_| ErrorKind::InvalidData)?;

        let mft_index_entry = &mft_entries[1];

        let mut index = vec![0; mft_index_entry.size.get() as usize];
        file.seek_read(&mut index, mft_index_entry.offset.get())?;

        let index = <[IndexEntry]>::ref_from_bytes(&index).map_err(|_| ErrorKind::InvalidData)?;

        Ok(Self {
            file,
            index: index
                .iter()
                .filter(|entry| entry.file_id != 0 && entry.mft_index != 0)
                .map(|entry| {
                    (
                        entry.file_id.get(),
                        mft_entries[entry.mft_index.get() as usize - 1],
                    )
                })
                .collect(),
        })
    }

    pub fn read(&self, file_id: u32) -> Result<Vec<u8>> {
        let mft_entry = self.index.get(&file_id).ok_or(ErrorKind::InvalidInput)?;

        let mut data = vec![0; mft_entry.size.get() as usize];
        self.file.seek_read(&mut data, mft_entry.offset.get())?;

        match mft_entry._2.get() {
            0 => Ok(data),
            8 => inflate(&data),
            _ => Err(ErrorKind::InvalidData.into()),
        }
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct AnHeader {
    version: u8,
    magic: [u8; 3],
    _2: U32,
    _3: U32,
    _4: U32,
    _5: U32,
    _6: U32,
    mft_offset: U64,
    mft_size: U32,
    _9: U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftHeader {
    magic: [u8; 4],
    _1: U32,
    _2: U32,
    entry_count: U32,
    _4: U32,
    _5: U32,
}

#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftEntry {
    offset: U64,
    size: U32,
    _2: U16,
    _3: u8,
    _4: u8,
    _5: U32,
    _6: U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct IndexEntry {
    file_id: U32,
    mft_index: U32,
}
