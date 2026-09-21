use std::{
    io::{ErrorKind, Result},
    sync::LazyLock,
};

use crate::inflate::{BitReader, HuffmanTree, HuffmanTreeBuilder};

const DXT1: u32 = u32::from_le_bytes(*b"DXT1");
const DXTA: u32 = u32::from_le_bytes(*b"DXTA");

const STATE_ALPHA: u8 = 1;
const STATE_COLOR: u8 = 2;

pub fn inflate_texture(input: &[u8], output: &mut [u8], format: u32) -> Result<()> {
    let block_size = if matches!(format, DXT1 | DXTA) { 8 } else { 16 };
    let color_offset = if matches!(format, DXT1 | DXTA) { 0 } else { 8 };

    let mut state = vec![0u8; output.len() / block_size];
    let mut bits = BitReader::new(input);

    bits.skip(32)?;
    let compression = bits.read(32)?;

    if compression & 1 != 0 {
        decode_white(&mut bits, block_size, output, &mut state)?;
    }

    if compression & 2 != 0 {
        let alpha = bits.read(4)? as u8;
        let alpha = alpha | alpha << 4;

        decode_alpha(&mut bits, block_size, output, &mut state, [alpha; 8])?;
    }

    if compression & 4 != 0 {
        let alpha = bits.read(8)? as u8;

        decode_alpha(
            &mut bits,
            block_size,
            output,
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
            block_size,
            color_offset,
            output,
            &mut state,
            encode_color(red, green, blue, format == DXT1),
        )?;
    }

    let mut pos = bits.pos()?;

    if format != DXT1 {
        for (block, &state) in state.iter().enumerate() {
            if state & STATE_ALPHA != 0 {
                continue;
            }

            let start = block * block_size;

            output[start..start + 8]
                .copy_from_slice(input.get(pos..pos + 8).ok_or(ErrorKind::UnexpectedEof)?);

            pos += 8;
        }
    }

    if format != DXTA {
        for offset in [0, 4] {
            for (block, &state) in state.iter().enumerate() {
                if state & STATE_COLOR != 0 {
                    continue;
                }

                let start = block * block_size + color_offset + offset;

                output[start..start + 4]
                    .copy_from_slice(input.get(pos..pos + 4).ok_or(ErrorKind::UnexpectedEof)?);

                pos += 4;
            }
        }
    }

    Ok(())
}

fn decode_white(
    bits: &mut BitReader<'_>,
    block_size: usize,
    output: &mut [u8],
    state: &mut [u8],
) -> Result<()> {
    const WHITE: [u8; 8] = 0xffff_ffff_ffff_fffeu64.to_le_bytes();

    let tree = texture_tree();
    let mut block = 0;

    while let Some(offset) = state[block..]
        .iter()
        .position(|state| state & STATE_COLOR == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            block += state[block..]
                .iter()
                .position(|state| state & STATE_COLOR == 0)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size;

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
    block_size: usize,
    output: &mut [u8],
    state: &mut [u8],
    value: [u8; 8],
) -> Result<()> {
    let tree = texture_tree();
    let mut block = 0;

    while let Some(offset) = state[block..]
        .iter()
        .position(|state| state & STATE_ALPHA == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;
        let nonzero = bits.peek(1)? != 0;

        if set {
            bits.skip(1)?;
        }

        for _ in 0..count {
            block += state[block..]
                .iter()
                .position(|state| state & STATE_ALPHA == 0)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size;
                let dst = &mut output[start..start + 8];

                if nonzero {
                    dst.copy_from_slice(&value);
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
    block_size: usize,
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
        .position(|state| state & STATE_COLOR == 0)
    {
        block += offset;

        let count = tree.read(bits)? as usize;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            block += state[block..]
                .iter()
                .position(|state| state & STATE_COLOR == 0)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size + color_offset;

                output[start..start + 8].copy_from_slice(&value);

                state[block] |= STATE_COLOR;
            }

            block += 1;
        }
    }

    Ok(())
}

#[inline]
fn encode_color(red: u32, green: u32, blue: u32, deduced_alpha: bool) -> u64 {
    let (red, red_error) = quantize5(red);
    let (green, green_error) = quantize6(green);
    let (blue, blue_error) = quantize5(blue);

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

    let special = deduced_alpha && (weight == 5 || weight == 6 || count != 0);

    if count != 0 && !special {
        if color2 == 0xffff {
            weight = 12;
            color1 -= 1;
        } else {
            weight = 0;
            color2 += 1;
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

    color1 as u64 | ((color2 as u64) << 16) | ((indices as u64) << 32)
}

#[inline(always)]
fn quantize5(value: u32) -> (u32, u32) {
    let base = (value - (value >> 5)) >> 3;
    let expanded = (base << 3) + (base >> 2);

    let error = 12 * (value - expanded) / (8 - (base & 0x11 == 0x11) as u32);

    (base, error)
}

#[inline(always)]
fn quantize6(value: u32) -> (u32, u32) {
    let base = (value - (value >> 6)) >> 2;
    let expanded = (base << 2) + (base >> 4);

    let error = 12 * (value - expanded) / (8 - (base & 0x1111 == 0x1111) as u32);

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
