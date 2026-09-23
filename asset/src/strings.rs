use std::io::{Error, ErrorKind, Result};

use rc4::{KeyInit, Rc4, StreamCipher};
use zerocopy::{FromBytes, Immutable, KnownLayout, little_endian::U16};

pub fn parse(mut data: &[u8]) -> Result<Vec<Entry>> {
    if data[..4] != *b"strs" {
        return Err(Error::new(ErrorKind::InvalidData, "strs: invalid magic"));
    }

    data = &data[4..];

    let mut entries = Vec::new();
    while data.len() >= 6 {
        let size = u16::from_le_bytes(data[..2].try_into().unwrap());
        let _data = data.split_at(size as usize);
        data = _data.1;

        let entry = EntryRepr::ref_from_bytes(_data.0).unwrap();
        entries.push(Entry {
            offset: entry.offset.get(),
            bits: entry.bits.get(),
            data: entry.data.to_vec().into_boxed_slice(),
        });
    }

    Ok(entries)
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct EntryRepr {
    size: U16,
    offset: U16,
    bits: U16,
    data: [u8],
}

pub struct Entry {
    offset: u16,
    bits: u16,
    data: Box<[u8]>,
}

impl Entry {
    pub fn encrypted(&self) -> bool {
        self.offset != 0
    }

    pub fn decrypt(&mut self, key: u64) -> Result<()> {
        if !self.encrypted() {
            return Ok(());
        }

        const REPLACEMENT: [char; 31] = [
            '0', '1', '2', '3', '4', '5', '6', 's', 't', 'r', 'n', 'u', 'm', '(', ')', '[', ']',
            '<', '>', '%', '#', '/', ':', '-', '\'', '"', ' ', ',', '.', '!', '\n',
        ];

        Rc4::new_from_slice(&rc4_hash(&key.to_le_bytes()))
            .map_err(|_| ErrorKind::InvalidData)?
            .apply_keystream(&mut self.data);

        let bits = self.bits as u32;
        let mask = (1u32 << bits) - 1;

        let mut buffer = 0u32;
        let mut buffered_bits = 0u32;
        let mut output = Vec::new();

        'outer: for &byte in &self.data {
            buffer |= (byte as u32) << buffered_bits;
            buffered_bits += 8;

            while buffered_bits >= bits {
                let symbol = (buffer & mask) as u16;

                buffer >>= bits;
                buffered_bits -= bits;

                if symbol == 0 {
                    break 'outer;
                }

                let ch = if symbol < 0x20 {
                    REPLACEMENT
                        .get(symbol as usize - 1)
                        .copied()
                        .ok_or(ErrorKind::InvalidData)?
                } else {
                    char::from_u32(symbol as u32 + self.offset as u32 - 0x20)
                        .ok_or(ErrorKind::InvalidData)?
                };

                let mut words = [0; 2];
                for w in ch.encode_utf16(&mut words) {
                    output.extend_from_slice(&w.to_le_bytes());
                }
            }
        }

        self.offset = 0;
        self.bits = 16;
        self.data = output.into_boxed_slice();

        Ok(())
    }

    pub fn to_string(&self) -> Result<String> {
        if self.encrypted() {
            return Err(Error::new(ErrorKind::InvalidData, "encrypted"));
        }

        String::from_utf16le(&self.data)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid UTF-16"))
    }
}

fn rc4_hash(input: &[u8]) -> [u8; 20] {
    assert!(!input.is_empty());

    let mut bytes = [0u8; 20];
    if input.len() >= 20 {
        bytes.copy_from_slice(&input[..20]);
    } else {
        for i in 0..20 {
            bytes[i] = input[i % input.len()];
        }
    }

    for i in 20..input.len() {
        bytes[i % 20] ^= input[i];
    }

    let mut words = [
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
    ];

    let mut a = 0x6745_2301u32;
    let mut b = 0xEFCD_AB89u32;
    let mut c = 0x98BA_DCFEu32;
    let mut d = 0x1032_5476u32;
    let mut e = 0xC3D2_E1F0u32;

    e = e
        .wrapping_add(words[0])
        .wrapping_add(d ^ (b & (c ^ d)))
        .wrapping_add(a.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    b = b.rotate_left(30);

    d = d
        .wrapping_add(words[1])
        .wrapping_add(c ^ (a & (b ^ c)))
        .wrapping_add(e.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    a = a.rotate_left(30);

    c = c
        .wrapping_add(words[2])
        .wrapping_add(b ^ (e & (a ^ b)))
        .wrapping_add(d.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    e = e.rotate_left(30);

    b = b
        .wrapping_add(words[3])
        .wrapping_add(a ^ (d & (e ^ a)))
        .wrapping_add(c.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    d = d.rotate_left(30);

    a = a
        .wrapping_add(words[4])
        .wrapping_add(e ^ (c & (d ^ e)))
        .wrapping_add(b.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    c = c.rotate_left(30);

    words[0] = words[0].wrapping_add(a);
    words[1] = words[1].wrapping_add(b);
    words[2] = words[2].wrapping_add(c);
    words[3] = words[3].wrapping_add(d);
    words[4] = words[4].wrapping_add(e);

    for (dst, src) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        dst.copy_from_slice(&src.to_le_bytes());
    }

    bytes
}
