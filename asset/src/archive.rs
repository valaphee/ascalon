use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Read as _, Result, Seek as _, SeekFrom};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use zerocopy::little_endian::{U16, U32, U64};
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::BitReader;

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

        let mft = Mft::ref_from_bytes(&mft).unwrap();
        if mft.magic != *b"Mft\x1A" {
            return Err(Error::new(ErrorKind::InvalidData, "invalid MFT magic"));
        }
        if mft.entry_count.get() as usize - 1 != mft.entries.len() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "invalid MFT entry count",
            ));
        }

        let [_, mft_index_entry, _, ..] = mft.entries else {
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
                .filter(|entry| entry.file_id != 0)
                .map(|entry| {
                    (
                        entry.file_id.get(),
                        mft.entries[entry.mft_index.get() as usize - 1],
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
            while remaining > 0 {
                let len = remaining.min(block.len());
                file.read_exact(&mut block[..len])?;

                let data_len = len.saturating_sub(4);
                bytes.extend_from_slice(&block[..data_len]);

                remaining -= len;
            }
        }

        Ok(match mft_entry.extra_bytes.get() {
            0 => bytes,
            8 => {
                let output_len = match bytes[..4] {
                    [0x08, 0x00, 0x01, 0x80] => u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                    [0x80, 0x01, 0x00, 0x08] => u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
                    _ => return Err(ErrorKind::InvalidData.into()),
                };

                let mut output = vec![0; output_len as usize];
                decompress(&bytes[8..], &mut output)?;
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
struct Mft {
    magic:       [u8; 4],
    _04:         U32,
    _08:         U32,
    entry_count: U32,
    _10:         U32,
    _14:         U32,
    entries:     [MftEntry],
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

pub fn decompress(input: &[u8], output: &mut [u8]) -> Result<()> {
    let mut bits = BitReader::new(input);

    bits.read(4)?;
    let length_bias = bits.read(4)? + 1;

    let mut output_pos = 0;
    while output_pos < output.len() {
        let literals = read_tree(&mut bits)?;
        let offsets = read_tree(&mut bits)?;

        for _ in 0..(bits.read(4)? + 1) << 12 {
            if output_pos == output.len() {
                break;
            }

            let symbol = literals.read(&mut bits)?;
            if symbol < 0x100 {
                output[output_pos] = symbol as u8;
                output_pos += 1;
                continue;
            }

            let symbol = symbol - 0x100;
            let group = symbol / 4;
            let mut length = match group {
                0 => symbol as u32,
                1..=6 => (1 << (group - 1)) * (4 + (symbol % 4) as u32),
                _ if symbol == 28 => 0xFF,
                _ => return Err(ErrorKind::InvalidData.into()),
            };
            if group > 1 && symbol != 28 {
                length |= bits.read((group - 1) as u32)?;
            }
            length += length_bias;

            let symbol = offsets.read(&mut bits)?;
            let group = symbol / 2;
            let mut offset = match group {
                0 => symbol as u32,
                1..=16 => (1 << (group - 1)) * (2 + (symbol % 2) as u32),
                _ => return Err(ErrorKind::InvalidData.into()),
            };
            if group > 1 {
                offset |= bits.read((group - 1) as u32)?;
            }

            let offset = offset as usize + 1;
            if offset > output_pos {
                return Err(ErrorKind::InvalidData.into());
            }

            let output_end = output_pos + (length as usize).min(output.len() - output_pos);
            while output_pos < output_end {
                let count = offset.min(output_end - output_pos);
                output.copy_within(output_pos - offset..output_pos - offset + count, output_pos);
                output_pos += count;
            }
        }
    }

    Ok(())
}

const HASH_BITS: usize = 8;
const HASH_SIZE: usize = 1 << HASH_BITS;
const MAX_CODE_BITS: usize = 32;
const MAX_SYMBOLS: usize = 285;

struct HuffmanTreeBuilder {
    head: [u16; MAX_CODE_BITS],
    next: [u16; MAX_SYMBOLS],
}

impl HuffmanTreeBuilder {
    const NONE: u16 = u16::MAX;

    fn new() -> Self {
        Self {
            head: [Self::NONE; MAX_CODE_BITS],
            next: [Self::NONE; MAX_SYMBOLS],
        }
    }

    fn add(&mut self, symbol: u16, bits: u8) -> Result<()> {
        let symbol = symbol as usize;
        let bits = bits as usize;
        if symbol >= MAX_SYMBOLS || bits >= MAX_CODE_BITS {
            return Err(ErrorKind::InvalidData.into());
        }

        self.next[symbol] = self.head[bits];
        self.head[bits] = symbol as u16;

        Ok(())
    }

    fn build(self) -> Result<Option<HuffmanTree>> {
        let mut hash_bits = [0; HASH_SIZE];
        let mut hash_symbol = [0; HASH_SIZE];
        let mut long = [LongEntry::default(); MAX_CODE_BITS - HASH_BITS];
        let mut long_len = 0;
        let mut symbols = [0; MAX_SYMBOLS];

        let mut code = 0u32;
        let mut any = false;
        let mut symbol_len = 0;

        for width in 0..MAX_CODE_BITS {
            let mut symbol = self.head[width];
            if symbol != Self::NONE {
                any = true;
            }

            if width <= HASH_BITS {
                while symbol != Self::NONE {
                    if width == 0 {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    let shift = HASH_BITS - width;
                    let start = (code as usize) << shift;
                    let end = (code.wrapping_add(1) as usize) << shift;
                    if end > HASH_SIZE {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    for entry in start..end {
                        hash_bits[entry] = width as u8;
                        hash_symbol[entry] = symbol;
                    }

                    code = code.wrapping_sub(1);
                    symbol = self.next[symbol as usize];
                }
            } else {
                let start = symbol_len;

                while symbol != Self::NONE {
                    if symbol_len >= MAX_SYMBOLS {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    symbols[symbol_len] = symbol;
                    symbol_len += 1;

                    code = code.wrapping_sub(1);
                    symbol = self.next[symbol as usize];
                }

                if symbol_len != start {
                    if long_len >= long.len() {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    long[long_len] = LongEntry {
                        comparison: code.wrapping_add(1).wrapping_shl(32 - width as u32),
                        bits:       width as u8,
                        end:        (symbol_len - 1) as u16,
                    };
                    long_len += 1;
                }
            }

            code = code.wrapping_shl(1).wrapping_add(1);
        }

        Ok(any.then_some(HuffmanTree::Multi {
            hash_bits,
            hash_symbol,
            long,
            long_len,
            symbols,
        }))
    }
}

fn read_tree(bits: &mut BitReader<'_>) -> Result<HuffmanTree> {
    static DICTIONARY: LazyLock<HuffmanTree> = LazyLock::new(|| {
        let mut lengths = [16; 256];

        macro_rules! lengths {
            ($($bits:expr => $($symbol:expr),+ $(,)?);+ $(;)?) => {
                $(
                    $(lengths[$symbol] = $bits;)*
                )*
            };
        }

        lengths! {
            3 => 0x08, 0x09, 0x0A;
            4 => 0x00, 0x07, 0x0B, 0x0C;
            5 => 0x06, 0x29, 0x2A, 0xE0;
            6 => 0x04, 0x05, 0x20, 0x28, 0x2B, 0x2C, 0x40, 0x4A;
            7 => 0x03, 0x0D, 0x25, 0x26, 0x27, 0x48, 0x49;
            8 => 0x24, 0x47, 0x4B, 0x4C, 0x69, 0x6A;
            9 => 0x23, 0x46, 0x60, 0x63, 0x67, 0x68, 0x88, 0x89, 0xA0, 0xE8;
            10 => 0x01, 0x02, 0x2D, 0x43, 0x44, 0x45, 0x65, 0x66, 0x80, 0x87, 0x8A, 0xA8, 0xA9, 0xC0, 0xC9, 0xE9;
            11 => 0x0E, 0x4D, 0x64, 0x6B, 0x6C, 0x84, 0x85, 0x8B, 0xA4, 0xA5, 0xAA, 0xC8, 0xE5;
            12 => 0x83, 0x86, 0xA6, 0xA7, 0xC7, 0xCA, 0xE7;
            13 => 0x22, 0x2E, 0x8C, 0xC4, 0xE4, 0xE6;
            14 => 0x4E, 0x6D, 0xC6, 0xEC;
            15 => 0x0F, 0x10, 0x11, 0x8D, 0xAB, 0xAC, 0xCC, 0xEA;
        }

        let mut builder = HuffmanTreeBuilder::new();

        for symbol in (0..256).rev() {
            builder.add(symbol, lengths[symbol as usize]).unwrap();
        }

        builder.build().unwrap().unwrap()
    });

    let count = bits.read(16)? as u16;
    if count > MAX_SYMBOLS as u16 {
        return Err(ErrorKind::InvalidData.into());
    }

    let mut builder = HuffmanTreeBuilder::new();

    let mut remaining = count as i32 - 1;
    let single = remaining as u16;

    while remaining >= 0 {
        let code = DICTIONARY.read(bits)?;

        let width = (code & 0x1F) as u8;
        let mut count = ((code >> 5) + 1) as i32;

        if width == 0 {
            remaining -= count;
            continue;
        }

        while count > 0 && remaining >= 0 {
            builder.add(remaining as u16, width)?;
            remaining -= 1;
            count -= 1;
        }
    }

    Ok(builder.build()?.unwrap_or(HuffmanTree::Single(single)))
}

#[derive(Clone, Copy, Default)]
struct LongEntry {
    comparison: u32,
    bits:       u8,
    end:        u16,
}

enum HuffmanTree {
    Single(u16),
    Multi {
        hash_bits:   [u8; HASH_SIZE],
        hash_symbol: [u16; HASH_SIZE],

        long:     [LongEntry; MAX_CODE_BITS - HASH_BITS],
        long_len: usize,

        symbols: [u16; MAX_SYMBOLS],
    },
}

impl HuffmanTree {
    fn read(&self, bits: &mut BitReader<'_>) -> Result<u16> {
        match self {
            Self::Single(symbol) => Ok(*symbol),
            Self::Multi {
                hash_bits,
                hash_symbol,
                long,
                long_len,
                symbols,
            } => {
                let prefix = bits.peek(HASH_BITS as u32) as usize;
                let width = hash_bits[prefix];
                if width != 0 {
                    bits.read(width as u32)?;
                    return Ok(hash_symbol[prefix]);
                }

                let value = bits.peek(32);
                for entry in &long[..*long_len] {
                    if value < entry.comparison {
                        continue;
                    }

                    let delta = ((value - entry.comparison) >> (32 - entry.bits as u32)) as usize;
                    let end = entry.end as usize;
                    if delta > end {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    bits.read(entry.bits as u32)?;

                    return Ok(symbols[end - delta]);
                }

                Err(ErrorKind::InvalidData.into())
            }
        }
    }
}
