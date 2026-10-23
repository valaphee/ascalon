use std::collections::HashMap;
use std::string::String as RustString;

use crate::packfile::Packfile;
use crate::packfile::cntc::PackContent;

pub struct Content {
    _packfiles: Vec<Packfile>,

    by_type:    HashMap<u32, Vec<*const u8>>,
    by_guid:    HashMap<Guid, *const u8>,
    by_name:    HashMap<RustString, *const u8>,
    by_data_id: HashMap<u32, *const u8>,
}

impl Content {
    pub fn new(packfiles: Vec<Packfile>) -> Self {
        let packs: Vec<_> = packfiles
            .iter()
            .map(|packfile| {
                assert_eq!(packfile.r#type(), *b"cntc");

                packfile
                    .chunks()
                    .find(|chunk| chunk.name() == *b"Main")
                    .map(|chunk| unsafe { &*(chunk.bytes().as_ptr() as *const PackContent) })
                    .unwrap()
            })
            .collect();

        let mut by_type: HashMap<u32, Vec<*const u8>> = HashMap::new();
        let mut by_guid = HashMap::new();
        let mut by_name = HashMap::new();
        let mut by_data_id = HashMap::new();

        unsafe {
            for pack in &packs {
                let content = pack.content.as_ptr();

                for fixup in pack.localOffsets.as_slice() {
                    let value = content
                        .cast_mut()
                        .add(fixup.relocOffset.get() as usize)
                        .cast::<usize>();
                    *value += pack.content.as_ptr() as usize;
                }

                for fixup in pack.externalOffsets.as_slice() {
                    let value = content
                        .cast_mut()
                        .add(fixup.relocOffset.get() as usize)
                        .cast::<usize>();
                    let target = &packs[fixup.targetFileIndex.get() as usize];
                    *value += target.content.as_ptr() as usize;
                }

                for fixup in pack.fileIndices.as_slice() {
                    let value = content
                        .cast_mut()
                        .add(fixup.relocOffset.get() as usize)
                        .cast::<usize>();
                    *value = packs[0].fileRefs.as_ptr().add(*value) as usize;
                }

                for fixup in pack.stringIndices.as_slice() {
                    let value = content
                        .cast_mut()
                        .add(fixup.relocOffset.get() as usize)
                        .cast::<usize>();
                    *value = pack.strings.as_ptr().add(*value) as usize;
                }

                for entry in pack.indexEntries.as_slice() {
                    let r#type = entry.r#type.get();
                    let type_info = &*packs[0].typeInfos.as_ptr().add(r#type as usize);

                    let content = content.add(entry.offset.get() as usize);

                    by_type.entry(r#type).or_default().push(content);

                    if type_info.guidOffset != u32::MAX {
                        let guid = *content
                            .add(type_info.guidOffset.get() as usize)
                            .cast::<Guid>();
                        by_guid.insert(guid, content);
                    }

                    /*if type_info.uidOffset != u32::MAX {
                        let data_id = *content
                            .add(type_info.uidOffset.get() as usize)
                            .cast::<u32>();
                        by_uid.insert(r#type << 22 | data_id & 0x3FFFFF, content);
                    }*/

                    if type_info.dataIdOffset != u32::MAX {
                        let data_id = *content
                            .add(type_info.dataIdOffset.get() as usize)
                            .cast::<u32>();
                        by_data_id.insert(r#type << 22 | data_id & 0x3FFFFF, content);
                    }

                    if type_info.nameOffset.get() != u32::MAX {
                        let name = &*content
                            .add(type_info.nameOffset.get() as usize)
                            .cast::<Ptr<Name>>();
                        by_name.insert(
                            RustString::from_utf16(name.as_ref().unwrap().0.0.as_slice()).unwrap(),
                            content,
                        );
                    }
                }
            }
        }

        Self {
            _packfiles: packfiles,
            by_type,
            by_guid,
            by_name,
            by_data_id,
        }
    }

    pub fn by_type<T: ContentType>(&self) -> &[&T] {
        let ptrs = self.by_type.get(&T::ID).map_or(&[][..], Vec::as_slice);
        unsafe { std::slice::from_raw_parts(ptrs.as_ptr().cast::<&T>(), ptrs.len()) }
    }

    pub fn by_guid<T: ContentType>(&self, guid: Guid) -> Option<&T> {
        let ptr = *self.by_guid.get(&guid)?;
        unsafe { (ptr.add(0x10).cast::<u32>().read() == T::ID).then(|| &*ptr.cast::<T>()) }
    }

    pub fn by_name<T: ContentType>(&self, name: &str) -> Option<&T> {
        let ptr = *self.by_name.get(name)?;
        unsafe { (ptr.add(0x10).cast::<u32>().read() == T::ID).then(|| &*ptr.cast::<T>()) }
    }

    pub fn by_data_id<T: ContentType>(&self, data_id: u32) -> Option<&T> {
        let ptr = *self.by_data_id.get(&(T::ID << 22 | data_id))?;
        Some(unsafe { &*ptr.cast::<T>() })
    }
}

pub trait ContentType {
    const ID: u32;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Guid(u32, u16, u16, [u8; 8]);

#[repr(transparent)]
pub struct Ptr<T: ?Sized>(*const T);

impl<T: ?Sized> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.0
    }

    pub unsafe fn as_ref(&self) -> Option<&T> {
        unsafe { self.as_ptr().as_ref() }
    }
}

#[repr(transparent)]
pub struct WcharPtr(*const u16);

impl WcharPtr {
    pub fn as_ptr(&self) -> *const u16 {
        self.0
    }

    pub unsafe fn len(&self) -> usize {
        let mut ptr = self.as_ptr();
        if ptr.is_null() {
            return 0;
        }

        unsafe {
            while ptr.read_unaligned() != 0 {
                ptr = ptr.add(1);
            }

            ptr.offset_from_unsigned(self.as_ptr())
        }
    }

    pub unsafe fn as_bytes(&self) -> &[u8] {
        let ptr = self.as_ptr();
        if ptr.is_null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr.cast(), self.len() * 2) }
    }

    pub unsafe fn as_slice(&self) -> &[u16] {
        let ptr = self.as_ptr();
        if ptr.is_null() {
            return &[];
        }

        unsafe { std::slice::from_raw_parts(ptr, self.len()) }
    }
}

#[repr(transparent)]
pub struct FileRef(WcharPtr);

impl FileRef {
    pub unsafe fn file_id(&self) -> Option<u32> {
        let [a, b, ..] = (unsafe { self.0.as_slice() }) else {
            return None;
        };

        let (a, b) = (*a, *b);
        if a <= 0xFF || b <= 0xFF {
            return None;
        }

        Some((u32::from(a) - 0xFF) + (u32::from(b) - 0x100) * 0xFF00)
    }
}

#[repr(C)]
pub struct String(WcharPtr, u32);

#[repr(C)]
pub struct Name(String, String);
