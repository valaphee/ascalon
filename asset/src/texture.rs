use std::io::{ErrorKind, Result};

use crate::BitReader;

pub const FMT_COLOR: u32 = 0x10;
pub const FMT_ALPHA: u32 = 0x20;
pub const FMT_DEDUCED_ALPHA: u32 = 0x40;
pub const FMT_PLAIN: u32 = 0x80;
pub const FMT_BICOLOR: u32 = 0x200;

pub const CMP_WHITE: u32 = 0x01;
pub const CMP_ALPHA4: u32 = 0x02;
pub const CMP_ALPHA8: u32 = 0x04;
pub const CMP_COLOR: u32 = 0x08;

pub fn inflate(
    block_size: usize,
    format: u32,
    compression: u32,
    input: &[u8],
    output: &mut [u8],
) -> Result<()> {
    if block_size == 0 || !output.len().is_multiple_of(block_size) {
        return Err(ErrorKind::InvalidData.into());
    }

    let has_two_components = format & (FMT_PLAIN | FMT_COLOR | FMT_ALPHA)
        == FMT_PLAIN | FMT_COLOR | FMT_ALPHA
        || format & FMT_BICOLOR != 0;

    let mut bits = BitReader::new(input);

    let component_size = block_size / if has_two_components { 2 } else { 1 };
    let color_offset = if has_two_components {
        component_size
    } else {
        0
    };

    let blocks = output.len() / block_size;
    let mut alpha = vec![false; blocks];
    let mut color = vec![false; blocks];

    if compression & CMP_WHITE != 0 {
        decode_white(&mut bits, block_size, output, &mut alpha, &mut color)?;
    }

    if compression & CMP_ALPHA4 != 0 {
        let a = bits.read(4)? as u8;
        let a = a | a << 4;

        decode_alpha(
            &mut bits,
            block_size,
            component_size,
            output,
            &mut alpha,
            [a; 8],
        )?;
    }

    if compression & CMP_ALPHA8 != 0 {
        let a = bits.read(8)? as u8;

        decode_alpha(
            &mut bits,
            block_size,
            component_size,
            output,
            &mut alpha,
            [a, a, 0, 0, 0, 0, 0, 0],
        )?;
    }

    if compression & CMP_COLOR != 0 {
        let b = bits.read(8)?;
        let g = bits.read(8)?;
        let r = bits.read(8)?;

        decode_color(
            &mut bits,
            block_size,
            component_size,
            color_offset,
            output,
            &mut color,
            encode_color(r, g, b, format & FMT_DEDUCED_ALPHA != 0),
        )?;
    }

    let mut input_pos = bits.position().div_ceil(32) * 4;

    if (format & FMT_ALPHA != 0 && format & FMT_DEDUCED_ALPHA == 0) || format & FMT_BICOLOR != 0 {
        for block in 0..blocks {
            if alpha[block] {
                continue;
            }

            let src = input
                .get(input_pos..input_pos + component_size)
                .ok_or(ErrorKind::UnexpectedEof)?;
            output[block * block_size..][..component_size].copy_from_slice(src);
            input_pos += component_size;
        }
    }

    if format & (FMT_COLOR | FMT_BICOLOR) != 0 {
        for offset in (0..component_size).step_by(4) {
            for block in 0..blocks {
                if color[block] {
                    continue;
                }

                let src = input
                    .get(input_pos..input_pos + 4)
                    .ok_or(ErrorKind::UnexpectedEof)?;
                output[block * block_size + color_offset + offset..][..4].copy_from_slice(src);
                input_pos += 4;
            }
        }
    }

    Ok(())
}

fn decode_white(
    bits: &mut BitReader<'_>,
    block_size: usize,
    output: &mut [u8],
    alpha: &mut [bool],
    color: &mut [bool],
) -> Result<()> {
    const WHITE: [u8; 8] = 0xFFFF_FFFF_FFFF_FFFEu64.to_le_bytes();

    let mut block = 0;

    while let Some(offset) = color[block..].iter().position(|x| !*x) {
        block += offset;

        let count = read_run(bits)?;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            block += color[block..]
                .iter()
                .position(|x| !*x)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size;

                output
                    .get_mut(start..start + 8)
                    .ok_or(ErrorKind::InvalidData)?
                    .copy_from_slice(&WHITE);

                alpha[block] = true;
                color[block] = true;
            }

            block += 1;
        }
    }

    Ok(())
}

fn decode_alpha(
    bits: &mut BitReader<'_>,
    block_size: usize,
    component_size: usize,
    output: &mut [u8],
    alpha: &mut [bool],
    value: [u8; 8],
) -> Result<()> {
    let mut block = 0;

    while let Some(offset) = alpha[block..].iter().position(|x| !*x) {
        block += offset;

        let count = read_run(bits)?;
        let set = bits.read(1)? != 0;

        let nonzero = bits.peek(1)? != 0;

        if set {
            bits.read(1)?;
        }

        for _ in 0..count {
            block += alpha[block..]
                .iter()
                .position(|x| !*x)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size;
                let dst = output
                    .get_mut(start..start + component_size)
                    .ok_or(ErrorKind::InvalidData)?;

                if nonzero {
                    dst.copy_from_slice(&value[..component_size]);
                } else {
                    dst.fill(0);
                }

                alpha[block] = true;
            }

            block += 1;
        }
    }

    Ok(())
}

fn decode_color(
    bits: &mut BitReader<'_>,
    block_size: usize,
    component_size: usize,
    color_offset: usize,
    output: &mut [u8],
    color: &mut [bool],
    value: u64,
) -> Result<()> {
    let value = value.to_le_bytes();
    let mut block = 0;

    while let Some(offset) = color[block..].iter().position(|x| !*x) {
        block += offset;

        let count = read_run(bits)?;
        let set = bits.read(1)? != 0;

        for _ in 0..count {
            block += color[block..]
                .iter()
                .position(|x| !*x)
                .ok_or(ErrorKind::InvalidData)?;

            if set {
                let start = block * block_size + color_offset;

                output
                    .get_mut(start..start + component_size)
                    .ok_or(ErrorKind::InvalidData)?
                    .copy_from_slice(&value[..component_size]);

                color[block] = true;
            }

            block += 1;
        }
    }

    Ok(())
}

#[inline(always)]
fn read_run(bits: &mut BitReader<'_>) -> Result<usize> {
    if bits.read(1)? != 0 {
        return Ok(1);
    }

    if bits.read(1)? != 0 {
        return Ok(0x12);
    }

    Ok(0x11 - bits.read(4)? as usize)
}

#[inline]
fn encode_color(r: u32, g: u32, b: u32, deduced_alpha: bool) -> u64 {
    let (r, re) = quantize5(r);
    let (g, ge) = quantize6(g);
    let (b, be) = quantize5(b);

    let (r1, r2) = endpoints(r, re);
    let (g1, g2) = endpoints(g, ge);
    let (b1, b2) = endpoints(b, be);

    let mut color1 = r1 | (g1 << 5) | (b1 << 11);
    let mut color2 = r2 | (g2 << 5) | (b2 << 11);

    let mut weight = 0;
    let mut count = 0;

    if r1 != r2 {
        weight += if r1 == r { re } else { 12 - re };
        count += 1;
    }

    if g1 != g2 {
        weight += if g1 == g { ge } else { 12 - ge };
        count += 1;
    }

    if b1 != b2 {
        weight += if b1 == b { be } else { 12 - be };
        count += 1;
    }

    weight = (weight + count / 2).checked_div(count).unwrap_or(weight);

    let special = deduced_alpha && (weight == 5 || weight == 6 || count != 0);

    if count != 0 && !special {
        if color2 == 0xFFFF {
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

    let index = if special {
        2
    } else {
        match weight {
            0..=1 => 0,
            2..=5 => 2,
            6..=9 => 3,
            _ => 1,
        }
    };

    color1 as u64 | ((color2 as u64) << 16) | (((index * 0x5555_5555) as u64) << 32)
}

#[inline(always)]
fn quantize5(value: u32) -> (u32, u32) {
    let base = (value - (value >> 5)) >> 3;
    let expanded = (base << 3) + (base >> 2);

    (
        base,
        12 * (value - expanded) / (8 - (base & 0x11 == 0x11) as u32),
    )
}

#[inline(always)]
fn quantize6(value: u32) -> (u32, u32) {
    let base = (value - (value >> 6)) >> 2;
    let expanded = (base << 2) + (base >> 4);

    (
        base,
        12 * (value - expanded) / (8 - (base & 0x1111 == 0x1111) as u32),
    )
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
