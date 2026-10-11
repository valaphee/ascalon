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
    fn peek(&mut self, count: u32) -> u32 {
        while self.bits < count {
            let word = if let Some((bytes, input)) = self.input.split_at_checked(4) {
                self.input = input;
                u32::from_le_bytes(bytes.try_into().unwrap()) as u64
            } else {
                0
            };

            self.word |= word << (32 - self.bits);
            self.bits += 32;
        }

        (self.word >> (64 - count)) as u32
    }

    #[inline(always)]
    fn read(&mut self, count: u32) -> std::io::Result<u32> {
        if self.bits as usize + self.input.len() * 8 < count as usize {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        let value = self.peek(count);

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
