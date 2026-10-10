#![feature(read_le)]

pub mod archive;
pub mod content;
pub mod inflate;
pub mod packfile;
pub mod texture;

struct BitReader<'a> {
    input:    &'a [u8],
    word:     u64,
    bits:     u32,
    position: usize,
}

impl<'a> BitReader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            word: 0,
            bits: 0,
            position: 0,
        }
    }

    #[inline(always)]
    fn peek(&mut self, count: u32) -> std::io::Result<u32> {
        while self.bits < count {
            let bytes = self
                .input
                .get(..4)
                .ok_or(std::io::ErrorKind::UnexpectedEof)?;
            self.input = &self.input[4..];
            self.word |= (u32::from_le_bytes(bytes.try_into().unwrap()) as u64) << (32 - self.bits);
            self.bits += 32;
        }

        Ok((self.word >> (64 - count)) as u32)
    }

    #[inline(always)]
    fn read(&mut self, count: u32) -> std::io::Result<u32> {
        let value = self.peek(count)?;

        self.word <<= count;
        self.bits -= count;
        self.position += count as usize;

        Ok(value)
    }

    #[inline(always)]
    fn position(&self) -> usize {
        self.position
    }
}
