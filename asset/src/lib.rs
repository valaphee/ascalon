pub mod archive;
pub mod inflate;
pub mod packfile;
pub mod strings;

pub fn file_name_to_id(name: &[u16]) -> Option<u32> {
    let [a, b, ..] = name else {
        return None;
    };

    if *a <= 0xff || *b <= 0xff {
        return None;
    }

    Some((u32::from(*a) - 0xff) + (u32::from(*b) - 0x100) * 0xff00)
}
