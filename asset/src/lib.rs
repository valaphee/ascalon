#![feature(read_le)]

pub mod archive;
pub mod compress;
pub mod packfile;

pub mod atex;
pub mod strs;

pub mod content;

fn rc4_hash(input: &[u8]) -> [u8; 20] {
    use zerocopy::FromBytes as _;
    use zerocopy::little_endian::U32;

    assert!(!input.is_empty());

    let mut bytes = [0u8; 20];
    if input.len() >= 20 {
        bytes.copy_from_slice(&input[..20]);
    } else {
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = input[i % input.len()];
        }
    }

    for (i, byte) in input.iter().enumerate().skip(20) {
        bytes[i % 20] ^= byte;
    }

    let words = <[U32; 5]>::mut_from_bytes(&mut bytes).unwrap();

    let mut a = 0x67452301u32;
    let mut b = 0xEFCDAB89u32;
    let mut c = 0x98BADCFEu32;
    let mut d = 0x10325476u32;
    let mut e = 0xC3D2E1F0u32;

    e = e
        .wrapping_add(words[0].get())
        .wrapping_add(d ^ (b & (c ^ d)))
        .wrapping_add(a.rotate_left(5))
        .wrapping_add(0x5A827999);
    b = b.rotate_left(30);

    d = d
        .wrapping_add(words[1].get())
        .wrapping_add(c ^ (a & (b ^ c)))
        .wrapping_add(e.rotate_left(5))
        .wrapping_add(0x5A827999);
    a = a.rotate_left(30);

    c = c
        .wrapping_add(words[2].get())
        .wrapping_add(b ^ (e & (a ^ b)))
        .wrapping_add(d.rotate_left(5))
        .wrapping_add(0x5A827999);
    e = e.rotate_left(30);

    b = b
        .wrapping_add(words[3].get())
        .wrapping_add(a ^ (d & (e ^ a)))
        .wrapping_add(c.rotate_left(5))
        .wrapping_add(0x5A827999);
    d = d.rotate_left(30);

    a = a
        .wrapping_add(words[4].get())
        .wrapping_add(e ^ (c & (d ^ e)))
        .wrapping_add(b.rotate_left(5))
        .wrapping_add(0x5A827999);
    c = c.rotate_left(30);

    words[0].set(words[0].get().wrapping_add(a));
    words[1].set(words[1].get().wrapping_add(b));
    words[2].set(words[2].get().wrapping_add(c));
    words[3].set(words[3].get().wrapping_add(d));
    words[4].set(words[4].get().wrapping_add(e));

    bytes
}
