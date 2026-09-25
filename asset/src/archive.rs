use std::{
    collections::HashMap,
    fs::File,
    io::{Error, ErrorKind, Read, Result, Seek, SeekFrom},
    path::Path,
    sync::Mutex,
};

use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    little_endian::{U16, U32, U64},
};

use crate::inflate::inflate;

pub struct Archive {
    file: Mutex<File>,
    index: HashMap<u32, MftEntry>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut an_header = vec![0; size_of::<AnHeader>()];
        file.read_exact(&mut an_header)?;

        let an_header = AnHeader::ref_from_bytes(&an_header).unwrap();
        if an_header.magic != *b"AN\x1A" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "archive: invalid AN magic",
            ));
        }

        let mut mft = vec![0; an_header.mft_size.get() as usize];
        file.seek(SeekFrom::Start(an_header.mft_offset.get()))?;
        file.read_exact(&mut mft)?;

        let (mft_header, mft_entries) = MftHeader::ref_from_prefix(&mft).unwrap();
        if mft_header.magic != *b"Mft\x1A" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "archive: invalid MFT magic",
            ));
        }

        let mft_entry_count = mft_header.entry_count.get() as usize;
        if mft_entry_count < 2 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "archive: invalid MFT entry count",
            ));
        }

        let mft_entries = <[MftEntry]>::ref_from_bytes_with_elems(mft_entries, mft_entry_count - 1)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "archive: invalid MFT entry count"))?;

        let mft_index_entry = &mft_entries[1];

        let mut index = vec![0; mft_index_entry.size.get() as usize];
        file.seek(SeekFrom::Start(mft_index_entry.offset.get()))?;
        file.read_exact(&mut index)?;

        let index = <[IndexEntry]>::ref_from_bytes(&index).unwrap();

        Ok(Self {
            file: Mutex::new(file),
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
        let mft_entry = self.index.get(&file_id).ok_or(ErrorKind::NotFound)?;

        let mut bytes = Vec::new();

        {
            const BLOCK_SIZE: usize = 0x10000;

            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(mft_entry.offset.get()))?;

            let mut remaining = mft_entry.size.get() as usize;
            let mut block = [0; BLOCK_SIZE];

            while remaining > BLOCK_SIZE - 4 {
                file.read_exact(&mut block)?;
                bytes.extend_from_slice(&block[..BLOCK_SIZE - 4]);
                remaining -= BLOCK_SIZE;
            }

            let len = bytes.len();
            bytes.resize(len + remaining, 0);
            file.read_exact(&mut bytes[len..])?;
        }

        match mft_entry._0c.get() {
            0 => Ok(bytes),
            8 => {
                let mut output =
                    vec![0; u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize];
                inflate(&bytes[8..], &mut output)?;
                Ok(output)
            }
            _ => Err(ErrorKind::InvalidData.into()),
        }
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct AnHeader {
    version: u8,
    magic: [u8; 3],
    _04: U32,
    _08: U32,
    _0c: U32,
    _10: U32,
    _14: U32,
    mft_offset: U64,
    mft_size: U32,
    _24: U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftHeader {
    magic: [u8; 4],
    _04: U32,
    _08: U32,
    entry_count: U32,
    _10: U32,
    _14: U32,
}

#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftEntry {
    offset: U64,
    size: U32,
    _0c: U16,
    _0e: u8,
    _0f: u8,
    _10: U32,
    _14: U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct IndexEntry {
    file_id: U32,
    mft_index: U32,
}
