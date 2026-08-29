use std::io::{self, ErrorKind};

use openssl::{
    bn::{BigNum, BigNumContext, MsbOption},
    rand::rand_bytes,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

fn big_num_le(bytes: &[u8]) -> io::Result<BigNum> {
    let mut bytes = bytes.to_vec();
    bytes.reverse();

    BigNum::from_slice(&bytes).map_err(io::Error::other)
}

pub struct ClientParams {
    g: BigNum,
    p: BigNum,
    y: BigNum,
}

pub struct ServerParams {
    p: BigNum,
    x: BigNum,
}

impl ClientParams {
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        Ok(Self {
            g: big_num_le(data.get(0x04..0x08).ok_or(ErrorKind::InvalidData)?)?,
            p: big_num_le(data.get(0x08..0x48).ok_or(ErrorKind::InvalidData)?)?,
            y: big_num_le(data.get(0x48..0x88).ok_or(ErrorKind::InvalidData)?)?,
        })
    }
}

impl ServerParams {
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        Ok(Self {
            p: big_num_le(data.get(0x08..0x48).ok_or(ErrorKind::InvalidData)?)?,
            x: big_num_le(data.get(0x88..0xC8).ok_or(ErrorKind::InvalidData)?)?,
        })
    }
}

pub async fn client_dh_key_exchange(
    stream: &mut TcpStream,
    params: &ClientParams,
) -> io::Result<[u8; 20]> {
    let mut x = BigNum::new().map_err(io::Error::other)?;
    x.rand(512, MsbOption::MAYBE_ZERO, false)
        .map_err(io::Error::other)?;

    let mut ctx = BigNumContext::new().map_err(io::Error::other)?;

    let mut y = BigNum::new().map_err(io::Error::other)?;
    y.mod_exp(&params.g, &x, &params.p, &mut ctx)
        .map_err(io::Error::other)?;

    let mut s = BigNum::new().map_err(io::Error::other)?;
    s.mod_exp(&params.y, &x, &params.p, &mut ctx)
        .map_err(io::Error::other)?;

    let mut y = y.to_vec_padded(64).map_err(io::Error::other)?;
    y.reverse();

    let mut packet = [0u8; 66];
    packet[0] = 0;
    packet[1] = 66;
    packet[2..].copy_from_slice(&y);
    stream.write_all(&packet).await?;

    let mut packet = [0u8; 22];
    stream.read_exact(&mut packet).await?;

    let mut s = s.to_vec();
    s.reverse();

    let mut k = [0u8; 20];
    k.copy_from_slice(&packet[2..]);
    for (k, s) in k.iter_mut().zip(&s) {
        *k ^= *s;
    }

    Ok(k)
}

pub async fn server_dh_key_exchange(
    stream: &mut TcpStream,
    params: &ServerParams,
) -> io::Result<[u8; 20]> {
    let mut packet = [0u8; 66];
    stream.read_exact(&mut packet).await?;

    let mut y = packet[2..].to_vec();
    y.reverse();
    let y = BigNum::from_slice(&y).map_err(io::Error::other)?;

    let mut ctx = BigNumContext::new().map_err(io::Error::other)?;

    let mut s = BigNum::new().map_err(io::Error::other)?;
    s.mod_exp(&y, &params.x, &params.p, &mut ctx)
        .map_err(io::Error::other)?;

    let mut s = s.to_vec();
    s.reverse();

    let mut k = [0u8; 20];
    rand_bytes(&mut k).map_err(io::Error::other)?;

    let mut k_encrypted = k;
    for (k, s) in k_encrypted.iter_mut().zip(&s) {
        *k ^= *s;
    }

    let mut packet = [0u8; 22];
    packet[0] = 1;
    packet[1] = 22;
    packet[2..].copy_from_slice(&k_encrypted);
    stream.write_all(&packet).await?;

    Ok(k)
}

pub fn rc4_hash(value: &[u8]) -> [u8; 20] {
    assert!(!value.is_empty());

    let mut block = [0u8; 20];
    if value.len() >= 20 {
        block.copy_from_slice(&value[..20]);
    } else {
        for i in 0..20 {
            block[i] = value[i % value.len()];
        }
    }

    for i in 20..value.len() {
        block[i % 20] ^= value[i];
    }

    let mut w = [
        u32::from_le_bytes(block[0..4].try_into().unwrap()),
        u32::from_le_bytes(block[4..8].try_into().unwrap()),
        u32::from_le_bytes(block[8..12].try_into().unwrap()),
        u32::from_le_bytes(block[12..16].try_into().unwrap()),
        u32::from_le_bytes(block[16..20].try_into().unwrap()),
    ];

    let mut a = 0x6745_2301u32;
    let mut b = 0xEFCD_AB89u32;
    let mut c = 0x98BA_DCFEu32;
    let mut d = 0x1032_5476u32;
    let mut e = 0xC3D2_E1F0u32;

    e = e
        .wrapping_add(w[0])
        .wrapping_add(d ^ (b & (c ^ d)))
        .wrapping_add(a.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    b = b.rotate_left(30);

    d = d
        .wrapping_add(w[1])
        .wrapping_add(c ^ (a & (b ^ c)))
        .wrapping_add(e.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    a = a.rotate_left(30);

    c = c
        .wrapping_add(w[2])
        .wrapping_add(b ^ (e & (a ^ b)))
        .wrapping_add(d.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    e = e.rotate_left(30);

    b = b
        .wrapping_add(w[3])
        .wrapping_add(a ^ (d & (e ^ a)))
        .wrapping_add(c.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    d = d.rotate_left(30);

    a = a
        .wrapping_add(w[4])
        .wrapping_add(e ^ (c & (d ^ e)))
        .wrapping_add(b.rotate_left(5))
        .wrapping_add(0x5A82_7999);
    c = c.rotate_left(30);

    w[0] = w[0].wrapping_add(a);
    w[1] = w[1].wrapping_add(b);
    w[2] = w[2].wrapping_add(c);
    w[3] = w[3].wrapping_add(d);
    w[4] = w[4].wrapping_add(e);

    for (dst, word) in block.chunks_exact_mut(4).zip(w) {
        dst.copy_from_slice(&word.to_le_bytes());
    }

    block
}
