use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Read as _, Result, Seek as _, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use zerocopy::little_endian::{U16, U32, U64};
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::inflate::inflate;

pub struct Archive {
    file:  Mutex<File>,
    index: HashMap<u32, MftEntry>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut an_header = vec![0; size_of::<AnHeader>()];
        file.read_exact(&mut an_header)?;

        let an_header = AnHeader::ref_from_bytes(&an_header).unwrap();
        if an_header.version != 151 || an_header.magic != *b"AN\x1A" {
            return Err(Error::new(ErrorKind::InvalidData, "invalid AN magic"));
        }

        let mut mft = vec![0; an_header.mft_size.get() as usize];
        file.seek(SeekFrom::Start(an_header.mft_offset.get()))?;
        file.read_exact(&mut mft)?;

        let (mft_header, mft_entries) = MftHeader::ref_from_prefix(&mft).unwrap();
        if mft_header.magic != *b"Mft\x1A" {
            return Err(Error::new(ErrorKind::InvalidData, "invalid MFT magic"));
        }

        let mft_entries = <[MftEntry]>::ref_from_bytes_with_elems(
            mft_entries,
            mft_header.entry_count.get() as usize - 1,
        )
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid MFT entry count"))?;
        let [_, mft_index_entry, _, ..] = mft_entries else {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid MFT entry count",
            ));
        };

        let mut index = vec![0; mft_index_entry.size.get() as usize];
        file.seek(SeekFrom::Start(mft_index_entry.offset.get()))?;
        file.read_exact(&mut index)?;

        let index = <[IndexEntry]>::ref_from_bytes(&index).unwrap();

        Ok(Self {
            file:  Mutex::new(file),
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
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(mft_entry.offset.get()))?;

            let mut block = [0; 0x10000];

            let mut remaining = mft_entry.size.get() as usize;
            while remaining > block.len() - 4 {
                file.read_exact(&mut block)?;
                bytes.extend_from_slice(&block[..block.len() - 4]);
                remaining -= block.len();
            }

            let len = bytes.len();
            bytes.resize(len + remaining, 0);
            file.read_exact(&mut bytes[len..])?;
        }

        Ok(match mft_entry.extra_bytes.get() {
            0 => bytes,
            8 => {
                let output_len = match bytes[0..4] {
                    [0x08, 0x00, 0x01, 0x80] => u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                    [0x80, 0x01, 0x00, 0x08] => u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
                    _ => return Err(ErrorKind::InvalidData.into()),
                };

                let mut output = vec![0; output_len as usize];
                inflate(&bytes[8..], &mut output)?;
                output
            }
            _ => return Err(ErrorKind::InvalidData.into()),
        })
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct AnHeader {
    version:    u8,
    magic:      [u8; 3],
    _04:        U32,
    _08:        U32,
    _0c:        U32,
    _10:        U32,
    _14:        U32,
    mft_offset: U64,
    mft_size:   U32,
    _24:        U32,
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftHeader {
    magic:       [u8; 4],
    _04:         U32,
    _08:         U32,
    entry_count: U32,
    _10:         U32,
    _14:         U32,
}

#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct MftEntry {
    offset:      U64,
    size:        U32,
    extra_bytes: U16,
    flags:       u8,
    stream:      u8,
    next_stream: U32,
    crc:         U32,
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MftEntryFlags: u8 {
        const ENTRY_USED   = 1 << 0;
        const FIRST_STREAM = 1 << 1;
    }
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct IndexEntry {
    file_id:   U32,
    mft_index: U32,
}
