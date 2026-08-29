use std::io::{self, ErrorKind};

use bytes::{Bytes, BytesMut};
use lz4_flex::block::{
    CompressTable, compress_into_with_table, decompress_into, get_maximum_output_size,
};
use rc4::{KeyInit, Rc4, StreamCipher};
use tokio_util::codec::{Decoder, Encoder};

use crate::rc4_hash;

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
    type Error = io::Error;

    fn encode(&mut self, data: Bytes, dst: &mut BytesMut) -> io::Result<()> {
        let start = dst.len();
        dst.resize(start + data.len(), 0);

        self.encryptor.apply_keystream_b2b(&data, &mut dst[start..]);

        Ok(())
    }
}

impl Decoder for ServerCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Bytes>> {
        if src.is_empty() {
            return Ok(None);
        }

        self.decryptor.apply_keystream(src);

        Ok(Some(src.split().freeze()))
    }
}

impl Encoder<Bytes> for ServerCodec {
    type Error = io::Error;

    fn encode(&mut self, data: Bytes, dst: &mut BytesMut) -> io::Result<()> {
        let max_len = get_maximum_output_size(data.len());
        let start = dst.len();

        dst.resize(start + 4 + max_len, 0);
        let compressed_len =
            compress_into_with_table(&data, &mut dst[start + 4..], &mut self.compress_table)
                .map_err(|_| io::Error::from(ErrorKind::InvalidData))?;
        dst.truncate(start + 4 + compressed_len);

        dst[start..start + 2].copy_from_slice(&(compressed_len as u16).to_le_bytes());
        dst[start + 2..start + 4].copy_from_slice(&(data.len() as u16).to_le_bytes());

        self.encryptor.apply_keystream(&mut dst[start..]);

        Ok(())
    }
}

impl Decoder for ClientCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Bytes>> {
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
            .map_err(|_| io::Error::from(ErrorKind::InvalidData))?;
        if written != raw_len {
            return Err(ErrorKind::InvalidData.into());
        }

        Ok(Some(self.decompressed.split().freeze()))
    }
}
