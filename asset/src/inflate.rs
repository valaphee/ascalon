use std::{
    io::{ErrorKind, Result},
    sync::LazyLock,
};

pub fn inflate(input: &[u8]) -> Result<Vec<u8>> {
    let mut bits = BitReader::new(input);

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
                1..=16 => (1u32 << (group - 1)) * (2 + (symbol % 2) as u32),
                _ => return Err(ErrorKind::InvalidData.into()),
            };
            if group > 1 {
                offset |= bits.read((group - 1) as u32)?;
            }

            let offset = offset as usize + 1;
            if offset > output.len() {
                return Err(ErrorKind::InvalidData.into());
            }

            let length = (length as usize).min(output_size - output.len());
            if length != 0 {
                let start = output.len() - offset;
                let end = output.len() + length;
                while output.len() < end {
                    let m = output.len() - start;
                    let n = m.min(end - output.len());
                    output.extend_from_within(start..start + n);
                }
            }
        }
    }

    Ok(output)
}

const STATE_ALPHA: u8 = 1;
const STATE_COLOR: u8 = 2;

const DXT1: u32 = 0x3154_5844;
const DXT2: u32 = 0x3254_5844;
const DXT3: u32 = 0x3354_5844;
const DXT4: u32 = 0x3454_5844;
const DXT5: u32 = 0x3554_5844;
const DXTA: u32 = 0x4154_5844;
const DXTL: u32 = 0x4c54_5844;
const DXTN: u32 = 0x4e54_5844;
const BC7X: u32 = 0x5837_4342;
const DC3X: u32 = 0x5843_4433;

pub fn inflate_texture(input: &[u8], format: u32, width: usize, height: usize) -> Result<Vec<u8>> {
    if !matches!(
        format,
        DXT1 | DXT2 | DXT3 | DXT4 | DXT5 | DXTA | DXTL | DXTN | BC7X | DC3X
    ) {
        return Err(ErrorKind::InvalidData.into());
    }

    let has_alpha = !matches!(format, DXT1 | DXTL);
    let has_color = format != DXTA;
    let component_bytes = if format == DXTL { 16 } else { 8 };

    let block_bytes = component_bytes * (usize::from(has_alpha) + usize::from(has_color));

    let color_offset = component_bytes * usize::from(has_alpha);

    let blocks = width
        .checked_add(3)
        .and_then(|width| {
            height
                .checked_add(3)
                .and_then(|height| (width / 4).checked_mul(height / 4))
        })
        .ok_or(ErrorKind::InvalidData)?;

    let output_size = blocks
        .checked_mul(block_bytes)
        .ok_or(ErrorKind::InvalidData)?;

    let mut output = vec![0; output_size];
    let mut state = vec![0; blocks];

    let mut bits = BitReader::new(input);
    let compression = bits.read(32)?;

    if compression & 1 != 0 {
        decode_white(&mut bits, block_bytes, &mut output, &mut state)?;
    }

    if compression & 2 != 0 {
        let alpha = bits.read(4)? as u8;
        let alpha = alpha | (alpha << 4);

        decode_alpha(
            &mut bits,
            block_bytes,
            component_bytes,
            &mut output,
            &mut state,
            [alpha; 8],
        )?;
    }

    if compression & 4 != 0 {
        let alpha = bits.read(8)? as u8;

        decode_alpha(
            &mut bits,
            block_bytes,
            component_bytes,
            &mut output,
            &mut state,
            [alpha, alpha, 0, 0, 0, 0, 0, 0],
        )?;
    }

    if compression & 8 != 0 {
        let blue = bits.read(8)?;
        let green = bits.read(8)?;
        let red = bits.read(8)?;

        decode_color(
            &mut bits,
            block_bytes,
            component_bytes,
            color_offset,
            &mut output,
            &mut state,
            encode_color(red, green, blue, format == DXT1),
        )?;
    }

    let mut pos = bits.pos()?;

    if has_alpha {
        copy_raw(
            input,
            &mut pos,
            &mut output,
            &state,
            block_bytes,
            component_bytes,
            0,
            STATE_ALPHA,
        )?;
    }

    if has_color {
        copy_raw(
            input,
            &mut pos,
            &mut output,
            &state,
            block_bytes,
            component_bytes,
            color_offset,
            STATE_COLOR,
        )?;
    }

    Ok(output)
}

fn decode_white(
    bits: &mut BitReader<'_>,
    block_bytes: usize,
    output: &mut [u8],
    state: &mut [u8],
) -> Result<()> {
    const WHITE: [u8; 8] = 0xffff_ffff_ffff_fffeu64.to_le_bytes();

    let tree = texture_tree();
    let mut block = 0;

    while let Some(offset) = state[block..]
        .iter()
        .position(|flags| flags & STATE_COLOR == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            let offset = state[block..]
                .iter()
                .position(|flags| flags & STATE_COLOR == 0)
                .ok_or(ErrorKind::InvalidData)?;

            block += offset;

            if set {
                let start = block * block_bytes;

                output[start..start + 8].copy_from_slice(&WHITE);
                state[block] |= STATE_ALPHA | STATE_COLOR;
            }

            block += 1;
        }
    }

    Ok(())
}

fn decode_alpha(
    bits: &mut BitReader<'_>,
    block_bytes: usize,
    component_bytes: usize,
    output: &mut [u8],
    state: &mut [u8],
    value: [u8; 8],
) -> Result<()> {
    let tree = texture_tree();
    let mut block = 0;

    while let Some(offset) = state[block..]
        .iter()
        .position(|flags| flags & STATE_ALPHA == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;
        let nonzero = bits.peek(1)? != 0;

        if set {
            bits.drop(1)?;
        }

        for _ in 0..count {
            let offset = state[block..]
                .iter()
                .position(|flags| flags & STATE_ALPHA == 0)
                .ok_or(ErrorKind::InvalidData)?;

            block += offset;

            if set {
                let start = block * block_bytes;
                let dst = &mut output[start..start + component_bytes];

                if nonzero {
                    dst.copy_from_slice(&value[..component_bytes]);
                } else {
                    dst.fill(0);
                }

                state[block] |= STATE_ALPHA;
            }

            block += 1;
        }
    }

    Ok(())
}

fn decode_color(
    bits: &mut BitReader<'_>,
    block_bytes: usize,
    component_bytes: usize,
    color_offset: usize,
    output: &mut [u8],
    state: &mut [u8],
    value: u64,
) -> Result<()> {
    let tree = texture_tree();
    let value = value.to_le_bytes();
    let mut block = 0;

    while let Some(offset) = state[block..]
        .iter()
        .position(|flags| flags & STATE_COLOR == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            let offset = state[block..]
                .iter()
                .position(|flags| flags & STATE_COLOR == 0)
                .ok_or(ErrorKind::InvalidData)?;

            block += offset;

            if set {
                let start = block * block_bytes + color_offset;

                output[start..start + component_bytes].copy_from_slice(&value[..component_bytes]);

                state[block] |= STATE_COLOR;
            }

            block += 1;
        }
    }

    Ok(())
}

fn copy_raw(
    input: &[u8],
    pos: &mut usize,
    output: &mut [u8],
    state: &[u8],
    block_bytes: usize,
    component_bytes: usize,
    component_offset: usize,
    mask: u8,
) -> Result<()> {
    for stripe in (0..component_bytes).step_by(4) {
        for (block, &flags) in state.iter().enumerate() {
            if flags & mask != 0 {
                continue;
            }

            let end = pos.checked_add(4).ok_or(ErrorKind::InvalidData)?;

            let src = input.get(*pos..end).ok_or(ErrorKind::UnexpectedEof)?;

            let start = block * block_bytes + component_offset + stripe;

            output[start..start + 4].copy_from_slice(src);
            *pos = end;
        }
    }

    Ok(())
}

#[inline]
fn encode_color(red: u32, green: u32, blue: u32, dxt1: bool) -> u64 {
    let (red, red_error) = quantize(red, 5);
    let (green, green_error) = quantize(green, 6);
    let (blue, blue_error) = quantize(blue, 5);

    let (r1, r2) = endpoints(red, red_error);
    let (g1, g2) = endpoints(green, green_error);
    let (b1, b2) = endpoints(blue, blue_error);

    let mut color1 = r1 | (g1 << 5) | (b1 << 11);
    let mut color2 = r2 | (g2 << 5) | (b2 << 11);

    let mut weight = 0;
    let mut count = 0;

    if r1 != r2 {
        weight += if r1 == red { red_error } else { 12 - red_error };

        count += 1;
    }

    if g1 != g2 {
        weight += if g1 == green {
            green_error
        } else {
            12 - green_error
        };

        count += 1;
    }

    if b1 != b2 {
        weight += if b1 == blue {
            blue_error
        } else {
            12 - blue_error
        };

        count += 1;
    }

    if count != 0 {
        weight = (weight + count / 2) / count;
    }

    let special = dxt1 && (weight == 5 || weight == 6 || count != 0);

    if count != 0 && !special {
        if color2 == 0xffff {
            color1 -= 1;
            weight = 12;
        } else {
            color2 += 1;
            weight = 0;
        }
    }

    if color2 >= color1 {
        std::mem::swap(&mut color1, &mut color2);
        weight = 12 - weight;
    }

    let index: u32 = if special {
        2
    } else {
        match weight {
            0..=1 => 0,
            2..=5 => 2,
            6..=9 => 3,
            _ => 1,
        }
    };

    let indices = index * 0x5555_5555;

    u64::from(color1) | (u64::from(color2) << 16) | (u64::from(indices) << 32)
}

#[inline(always)]
fn quantize(value: u32, bits: u32) -> (u32, u32) {
    let (base, expanded, mask) = match bits {
        5 => {
            let base = (value - (value >> 5)) >> 3;
            (base, (base << 3) + (base >> 2), 0x11)
        }

        6 => {
            let base = (value - (value >> 6)) >> 2;
            (base, (base << 2) + (base >> 4), 0x1111)
        }

        _ => unreachable!(),
    };

    let error = 12 * (value - expanded) / (8 - u32::from(base & mask == mask));

    (base, error)
}

#[inline(always)]
fn endpoints(base: u32, error: u32) -> (u32, u32) {
    match error {
        0..=1 => (base, base),
        2..=5 => (base, base + 1),
        6..=9 => (base + 1, base),
        _ => (base + 1, base + 1),
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_index: usize,
    word: u64,
    bit_index: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut this = Self {
            data,
            byte_index: 0,
            word: 0,
            bit_index: 0,
        };

        this.fill();
        this
    }

    fn fill(&mut self) {
        while self.bit_index <= 32 {
            let word_index = self.byte_index >> 2;
            if (word_index + 1) % 16_384 == 0 {
                self.byte_index += 4;
            }

            let Some(bytes) = self.data.get(self.byte_index..self.byte_index + 4) else {
                break;
            };
            self.byte_index += 4;

            let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            self.word |= (word as u64) << (32 - self.bit_index);
            self.bit_index += 32;
        }
    }

    #[inline(always)]
    fn peek(&self, count: u32) -> Result<u32> {
        if count > self.bit_index {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        if count == 0 {
            return Ok(0);
        }

        Ok((self.word >> (64 - count)) as u32)
    }

    #[inline(always)]
    fn drop(&mut self, count: u32) -> Result<()> {
        if count > self.bit_index {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        self.word <<= count;
        self.bit_index -= count;
        if self.bit_index <= 32 {
            self.fill();
        }

        Ok(())
    }

    #[inline(always)]
    fn read(&mut self, count: u32) -> Result<u32> {
        if count > self.bit_index {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        let value = if count == 0 {
            0
        } else {
            (self.word >> (64 - count)) as u32
        };

        self.word <<= count;
        self.bit_index -= count;
        if self.bit_index <= 32 {
            self.fill();
        }

        Ok(value)
    }

    #[inline(always)]
    fn pos(&self) -> Result<usize> {
        if self.bit_index >= 32 {
            self.byte_index
                .checked_sub(4)
                .ok_or(ErrorKind::InvalidData.into())
        } else {
            Ok(self.byte_index)
        }
    }
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

    #[inline(always)]
    fn add(&mut self, symbol: u16, bits: u8) -> Result<()> {
        let symbol_index = symbol as usize;
        let bits_index = bits as usize;
        if symbol_index >= MAX_SYMBOLS || bits_index >= MAX_CODE_BITS {
            return Err(ErrorKind::InvalidData.into());
        }

        self.next[symbol_index] = self.head[bits_index];
        self.head[bits_index] = symbol;

        Ok(())
    }

    fn build(self) -> Result<Option<HuffmanTree>> {
        let mut hash_bits = [0; HASH_SIZE];
        let mut hash_symbol = [0; HASH_SIZE];
        let mut long = [LongEntry::default(); MAX_CODE_BITS - HASH_BITS];
        let mut long_len = 0usize;
        let mut symbols = [0; MAX_SYMBOLS];

        let mut code = 0u32;
        let mut any = false;
        let mut symbol_len = 0usize;

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
                let symbol_start = symbol_len;

                while symbol != Self::NONE {
                    if symbol_len >= MAX_SYMBOLS {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    symbols[symbol_len] = symbol;
                    symbol_len += 1;

                    code = code.wrapping_sub(1);
                    symbol = self.next[symbol as usize];
                }

                if symbol_len != symbol_start {
                    if long_len >= long.len() {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    long[long_len] = LongEntry {
                        comparison: code.wrapping_add(1).wrapping_shl(32 - width as u32),
                        bits: width as u8,
                        end: (symbol_len - 1) as u16,
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
        let mut lengths = [16u8; 256];

        macro_rules! set {
            ($bits:expr => $($symbol:expr),* $(,)?) => {
                $(
                    lengths[$symbol] = $bits;
                )*
            };
        }

        set!(3 => 0x08, 0x09, 0x0A);
        set!(4 => 0x00, 0x07, 0x0B, 0x0C);
        set!(5 => 0x06, 0x29, 0x2A, 0xE0);
        set!(6 => 0x04, 0x05, 0x20, 0x28, 0x2B, 0x2C, 0x40, 0x4A);
        set!(7 => 0x03, 0x0D, 0x25, 0x26, 0x27, 0x48, 0x49);
        set!(8 => 0x24, 0x47, 0x4B, 0x4C, 0x69, 0x6A);
        set!(9 => 0x23, 0x46, 0x60, 0x63, 0x67, 0x68, 0x88, 0x89, 0xA0, 0xE8);
        set!(10 =>
            0x01, 0x02, 0x2D, 0x43, 0x44, 0x45, 0x65, 0x66,
            0x80, 0x87, 0x8A, 0xA8, 0xA9, 0xC0, 0xC9, 0xE9
        );
        set!(11 =>
            0x0E, 0x4D, 0x64, 0x6B, 0x6C, 0x84, 0x85,
            0x8B, 0xA4, 0xA5, 0xAA, 0xC8, 0xE5
        );
        set!(12 => 0x83, 0x86, 0xA6, 0xA7, 0xC7, 0xCA, 0xE7);
        set!(13 => 0x22, 0x2E, 0x8C, 0xC4, 0xE4, 0xE6);
        set!(14 => 0x4E, 0x6D, 0xC6, 0xEC);
        set!(15 => 0x0F, 0x10, 0x11, 0x8D, 0xAB, 0xAC, 0xCC, 0xEA);

        let mut builder = HuffmanTreeBuilder::new();

        for symbol in (0u16..=255).rev() {
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

    Ok(builder.build()?.unwrap_or(HuffmanTree::Single(single)))
}

#[inline(always)]
fn texture_tree() -> &'static HuffmanTree {
    static TREE: LazyLock<HuffmanTree> = LazyLock::new(|| {
        let mut builder = HuffmanTreeBuilder::new();

        builder.add(0x01, 1).unwrap();
        builder.add(0x12, 2).unwrap();

        for symbol in (0x02..=0x11).rev() {
            builder.add(symbol, 6).unwrap();
        }

        builder.build().unwrap().unwrap()
    });

    &TREE
}

#[derive(Clone, Copy, Default)]
struct LongEntry {
    comparison: u32,
    bits: u8,
    end: u16,
}

enum HuffmanTree {
    Single(u16),
    Multi {
        hash_bits: [u8; HASH_SIZE],
        hash_symbol: [u16; HASH_SIZE],

        long: [LongEntry; MAX_CODE_BITS - HASH_BITS],
        long_len: usize,

        symbols: [u16; MAX_SYMBOLS],
    },
}

impl HuffmanTree {
    #[inline(always)]
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
                let prefix = bits.peek(HASH_BITS as u32)? as usize;
                let width = hash_bits[prefix];
                if width != 0 {
                    bits.drop(width as u32)?;

                    return Ok(hash_symbol[prefix]);
                }

                let value = bits.peek(32)?;
                for entry in &long[..*long_len] {
                    if value < entry.comparison {
                        continue;
                    }

                    let delta = ((value - entry.comparison) >> (32 - entry.bits as u32)) as usize;
                    let end = entry.end as usize;
                    if delta > end {
                        return Err(ErrorKind::InvalidData.into());
                    }

                    bits.drop(entry.bits as u32)?;

                    return Ok(symbols[end - delta]);
                }

                Err(ErrorKind::InvalidData.into())
            }
        }
    }
}
