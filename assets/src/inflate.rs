use std::{io, sync::LazyLock};

const SKIP_WORD_INTERVAL: usize = 16_384;

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u64,
    bits: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut this = Self {
            data,
            pos: 0,
            buffer: 0,
            bits: 0,
        };

        this.fill();
        this
    }

    #[inline(always)]
    fn pull_word(&mut self) -> Option<u32> {
        let physical_word = self.pos >> 2;

        if (physical_word + 1) % SKIP_WORD_INTERVAL == 0 {
            self.pos += 4;
        }

        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;

        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    #[inline(always)]
    fn fill(&mut self) {
        while self.bits <= 32 {
            let Some(word) = self.pull_word() else {
                break;
            };

            self.buffer |= (word as u64) << (32 - self.bits);
            self.bits += 32;
        }
    }

    #[inline(always)]
    fn peek(&self, count: u32) -> io::Result<u32> {
        if count > self.bits {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        if count == 0 {
            return Ok(0);
        }

        Ok((self.buffer >> (64 - count)) as u32)
    }

    #[inline(always)]
    fn drop(&mut self, count: u32) -> io::Result<()> {
        if count > self.bits {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        self.buffer <<= count;
        self.bits -= count;

        if self.bits <= 32 {
            self.fill();
        }

        Ok(())
    }

    #[inline(always)]
    fn read(&mut self, count: u32) -> io::Result<u32> {
        let value = self.peek(count)?;
        self.drop(count)?;
        Ok(value)
    }
}

const HASH_BITS: usize = 8;
const HASH_SIZE: usize = 1 << HASH_BITS;

#[derive(Clone, Copy, Default)]
struct LongEntry {
    comparison: u32,
    bits: u8,
    end: u16,
}

struct HuffmanTree {
    single: u16,
    has_single: bool,

    hash_symbol: [u16; HASH_SIZE],
    hash_bits: [u8; HASH_SIZE],

    long: [LongEntry; MAX_CODE_BITS - HASH_BITS],
    long_len: usize,

    symbols: [u16; MAX_SYMBOLS],
    symbol_len: usize,
}

impl HuffmanTree {
    #[inline(always)]
    fn read(&self, bits: &mut BitReader<'_>) -> io::Result<u16> {
        if self.has_single {
            return Ok(self.single);
        }

        let prefix = bits.peek(HASH_BITS as u32)? as usize;
        let width = self.hash_bits[prefix];

        if width != 0 {
            bits.drop(width as u32)?;
            return Ok(self.hash_symbol[prefix]);
        }

        let value = bits.peek(32)?;

        for entry in &self.long[..self.long_len] {
            if value < entry.comparison {
                continue;
            }

            let delta = ((value - entry.comparison) >> (32 - entry.bits as u32)) as usize;

            let end = entry.end as usize;

            if delta > end {
                return Err(io::ErrorKind::InvalidData.into());
            }

            bits.drop(entry.bits as u32)?;
            return Ok(self.symbols[end - delta]);
        }

        Err(io::ErrorKind::InvalidData.into())
    }
}

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

    #[inline(always)]
    fn add(&mut self, symbol: u16, bits: u8) -> io::Result<()> {
        let symbol_index = symbol as usize;
        let bits_index = bits as usize;

        if symbol_index >= MAX_SYMBOLS || bits_index >= MAX_CODE_BITS {
            return Err(io::ErrorKind::InvalidData.into());
        }

        self.next[symbol_index] = self.head[bits_index];
        self.head[bits_index] = symbol;

        Ok(())
    }

    fn build(self) -> io::Result<Option<HuffmanTree>> {
        let mut tree = HuffmanTree {
            single: 0,
            has_single: false,

            hash_symbol: [0; HASH_SIZE],
            hash_bits: [0; HASH_SIZE],

            long: [LongEntry::default(); MAX_CODE_BITS - HASH_BITS],
            long_len: 0,

            symbols: [0; MAX_SYMBOLS],
            symbol_len: 0,
        };
        let mut code = 0u32;
        let mut any = false;

        for width in 0..MAX_CODE_BITS {
            let mut symbol = self.head[width];

            if symbol != Self::NONE {
                any = true;
            }

            if width <= HASH_BITS {
                while symbol != Self::NONE {
                    if width == 0 {
                        return Err(io::ErrorKind::InvalidData.into());
                    }

                    let shift = HASH_BITS - width;
                    let start = (code as usize) << shift;
                    let end = (code.wrapping_add(1) as usize) << shift;

                    if end > HASH_SIZE {
                        return Err(io::ErrorKind::InvalidData.into());
                    }

                    for entry in start..end {
                        tree.hash_symbol[entry] = symbol;
                        tree.hash_bits[entry] = width as u8;
                    }

                    code = code.wrapping_sub(1);
                    symbol = self.next[symbol as usize];
                }
            } else {
                let symbol_start = tree.symbol_len;

                while symbol != Self::NONE {
                    if tree.symbol_len >= MAX_SYMBOLS {
                        return Err(io::ErrorKind::InvalidData.into());
                    }

                    tree.symbols[tree.symbol_len] = symbol;
                    tree.symbol_len += 1;

                    code = code.wrapping_sub(1);
                    symbol = self.next[symbol as usize];
                }

                if tree.symbol_len != symbol_start {
                    if tree.long_len >= tree.long.len() {
                        return Err(io::ErrorKind::InvalidData.into());
                    }

                    tree.long[tree.long_len] = LongEntry {
                        comparison: code.wrapping_add(1).wrapping_shl(32 - width as u32),
                        bits: width as u8,
                        end: (tree.symbol_len - 1) as u16,
                    };

                    tree.long_len += 1;
                }
            }

            code = code.wrapping_shl(1).wrapping_add(1);
        }

        Ok(any.then_some(tree))
    }
}

static DICTIONARY: LazyLock<HuffmanTree> = LazyLock::new(|| {
    let mut lengths = [16u8; 256];

    macro_rules! set {
        ($bits:expr => $($symbol:expr),* $(,)?) => {
            $(
                lengths[$symbol] = $bits;
            )*
        };
    }

    set!(3 =>
        0x08, 0x09, 0x0A
    );

    set!(4 =>
        0x00, 0x07, 0x0B, 0x0C
    );

    set!(5 =>
        0x06, 0x29, 0x2A, 0xE0
    );

    set!(6 =>
        0x04, 0x05, 0x20, 0x28,
        0x2B, 0x2C, 0x40, 0x4A
    );

    set!(7 =>
        0x03, 0x0D, 0x25, 0x26,
        0x27, 0x48, 0x49
    );

    set!(8 =>
        0x24, 0x47, 0x4B,
        0x4C, 0x69, 0x6A
    );

    set!(9 =>
        0x23, 0x46, 0x60, 0x63, 0x67,
        0x68, 0x88, 0x89, 0xA0, 0xE8
    );

    set!(10 =>
        0x01, 0x02, 0x2D, 0x43,
        0x44, 0x45, 0x65, 0x66,
        0x80, 0x87, 0x8A, 0xA8,
        0xA9, 0xC0, 0xC9, 0xE9
    );

    set!(11 =>
        0x0E, 0x4D, 0x64, 0x6B,
        0x6C, 0x84, 0x85, 0x8B,
        0xA4, 0xA5, 0xAA, 0xC8,
        0xE5
    );

    set!(12 =>
        0x83, 0x86, 0xA6,
        0xA7, 0xC7, 0xCA, 0xE7
    );

    set!(13 =>
        0x22, 0x2E, 0x8C,
        0xC4, 0xE4, 0xE6
    );

    set!(14 =>
        0x4E, 0x6D, 0xC6, 0xEC
    );

    set!(15 =>
        0x0F, 0x10, 0x11, 0x8D,
        0xAB, 0xAC, 0xCC, 0xEA
    );

    let mut builder = HuffmanTreeBuilder::new();

    for symbol in (0u16..=255).rev() {
        builder.add(symbol, lengths[symbol as usize]).unwrap();
    }

    builder.build().unwrap().unwrap()
});

#[inline]
fn read_tree(bits: &mut BitReader<'_>) -> io::Result<HuffmanTree> {
    let count = bits.read(16)? as u16;
    if count > MAX_SYMBOLS as u16 {
        return Err(io::ErrorKind::InvalidData.into());
    }

    let mut builder = HuffmanTreeBuilder::new();

    let mut remaining = count as i32 - 1;
    let single = remaining as u16;

    while remaining >= 0 {
        let code = DICTIONARY.read(bits)?;

        let width = (code & 0x1f) as u8;
        let mut n = ((code >> 5) + 1) as i32;

        if width == 0 {
            remaining -= n;
            continue;
        }

        while n > 0 && remaining >= 0 {
            builder.add(remaining as u16, width)?;

            remaining -= 1;
            n -= 1;
        }
    }

    Ok(builder.build()?.unwrap_or_else(|| HuffmanTree {
        single,
        has_single: true,

        hash_symbol: [0; HASH_SIZE],
        hash_bits: [0; HASH_SIZE],

        long: [LongEntry::default(); MAX_CODE_BITS - HASH_BITS],
        long_len: 0,

        symbols: [0; MAX_SYMBOLS],
        symbol_len: 0,
    }))
}

pub fn inflate(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut bits = BitReader::new(data);

    bits.drop(32)?;
    let output_size = bits.read(32)? as usize;

    bits.drop(4)?;
    let length_bias = bits.read(4)? + 1;

    let mut output = Vec::with_capacity(output_size);
    while output.len() < output_size {
        let literals = read_tree(&mut bits)?;
        let offsets = read_tree(&mut bits)?;

        let commands = (bits.read(4)? + 1) << 12;
        for _ in 0..commands {
            if output.len() == output_size {
                break;
            }

            let symbol = literals.read(&mut bits)?;
            if symbol < 0x100 {
                output.push(symbol as u8);
                continue;
            }

            let symbol = symbol - 0x100;
            let group = symbol / 4;

            let mut length = match group {
                0 => symbol as u32,
                1..=6 => (1u32 << (group - 1)) * (4 + (symbol % 4) as u32),
                _ if symbol == 28 => 0xff,
                _ => {
                    return Err(io::ErrorKind::InvalidData.into());
                }
            };

            if group > 1 && symbol != 28 {
                length |= bits.read((group - 1) as u32)?;
            }

            length += length_bias;

            let symbol = offsets.read(&mut bits)?;
            let group = symbol / 2;

            let mut offset = match group {
                0 => symbol as u32,
                1..=16 => (1u32 << (group - 1)) * (2 + (symbol % 2) as u32),
                _ => {
                    return Err(io::ErrorKind::InvalidData.into());
                }
            };

            if group > 1 {
                offset |= bits.read((group - 1) as u32)?;
            }

            let offset = offset as usize + 1;
            if offset > output.len() {
                return Err(io::ErrorKind::InvalidData.into());
            }

            let mut length = (length as usize).min(output_size - output.len());
            while length != 0 {
                let n = offset.min(length);
                let start = output.len() - offset;

                output.extend_from_within(start..start + n);

                length -= n;
            }
        }
    }

    Ok(output)
}
