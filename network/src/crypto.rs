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
