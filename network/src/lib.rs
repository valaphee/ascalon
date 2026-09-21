use std::io::{Error, ErrorKind, Result};

use bytes::{Bytes, BytesMut};
use lz4_flex::block::{
    CompressTable, compress_into_with_table, decompress_into, get_maximum_output_size,
};
use rc4::{KeyInit, Rc4, StreamCipher};
use tokio_util::codec::{Decoder, Encoder};

pub use tokio_util::codec::Framed;

pub struct ClientCodec {
    encryptor: Rc4,
    decryptor: Rc4,
    decrypted: usize,

    decompressed: BytesMut,
}

impl ClientCodec {
    pub fn from_key(key: &[u8]) -> Self {
        let key = rc4_hash(key);

        Self {
            encryptor: Rc4::new_from_slice(&key).unwrap(),
            decryptor: Rc4::new_from_slice(&key).unwrap(),
            decrypted: 0,

            decompressed: BytesMut::new(),
        }
    }
}

pub struct ServerCodec {
    encryptor: Rc4,
    decryptor: Rc4,

    compress_table: CompressTable,
}

impl ServerCodec {
    pub fn from_key(key: &[u8]) -> Self {
        let key = rc4_hash(key);

        Self {
            encryptor: Rc4::new_from_slice(&key).unwrap(),
            decryptor: Rc4::new_from_slice(&key).unwrap(),

            compress_table: CompressTable::default(),
        }
    }
}

impl Encoder<Bytes> for ClientCodec {
    type Error = Error;

    fn encode(&mut self, src: Bytes, dst: &mut BytesMut) -> Result<()> {
        let start = dst.len();

        dst.resize(start + src.len(), 0);
        self.encryptor.apply_keystream_b2b(&src, &mut dst[start..]);

        Ok(())
    }
}

impl Decoder for ServerCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Bytes>> {
        if src.is_empty() {
            return Ok(None);
        }

        self.decryptor.apply_keystream(src);

        Ok(Some(src.split().freeze()))
    }
}

impl Encoder<Bytes> for ServerCodec {
    type Error = Error;

    fn encode(&mut self, src: Bytes, dst: &mut BytesMut) -> Result<()> {
        let start = dst.len();

        dst.resize(start + 4 + get_maximum_output_size(src.len()), 0);
        let compressed_len =
            compress_into_with_table(&src, &mut dst[start + 4..], &mut self.compress_table)
                .map_err(|_| Error::from(ErrorKind::InvalidData))?;
        dst.truncate(start + 4 + compressed_len);

        dst[start..start + 2].copy_from_slice(&(compressed_len as u16).to_le_bytes());
        dst[start + 2..start + 4].copy_from_slice(&(src.len() as u16).to_le_bytes());

        self.encryptor.apply_keystream(&mut dst[start..]);

        Ok(())
    }
}

impl Decoder for ClientCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Bytes>> {
        self.decryptor.apply_keystream(&mut src[self.decrypted..]);
        self.decrypted = src.len();

        if src.len() < 4 {
            return Ok(None);
        }

        let compressed_len = u16::from_le_bytes([src[0], src[1]]) as usize;
        let raw_len = u16::from_le_bytes([src[2], src[3]]) as usize;

        let frame_len = 4 + compressed_len;
        if src.len() < frame_len {
            return Ok(None);
        }

        let frame = src.split_to(frame_len);
        self.decrypted -= frame_len;

        self.decompressed.resize(raw_len, 0);
        let written = decompress_into(&frame[4..], &mut self.decompressed)
            .map_err(|_| Error::from(ErrorKind::InvalidData))?;
        if written != raw_len {
            return Err(Error::from(ErrorKind::InvalidData));
        }

        Ok(Some(self.decompressed.split().freeze()))
    }
}

pub fn rc4_hash(input: &[u8]) -> [u8; 20] {
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

    for (dst, src) in bytes.chunks_exact_mut(4).zip(words) {
        dst.copy_from_slice(&src.to_le_bytes());
    }

    bytes
}
