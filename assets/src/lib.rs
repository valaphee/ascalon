pub mod archive;
pub mod inflate;
pub mod packfile;
pub mod strings;

pub fn file_name_to_id(name: &[u16]) -> u32 {
    return (name[0] as u32 - 0xff) + (name[1] as u32 - 0x100) * 0xff00;
}
