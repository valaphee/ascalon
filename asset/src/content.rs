use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::packfile::cntc::PackContent;

pub struct ContentServer {
    by_type:    HashMap<u32, Vec<*const u8>>,
    by_guid:    HashMap<Guid, *const u8>,
    by_data_id: HashMap<u32, *const u8>,
    by_name:    HashMap<u64, *const u8>,
}

impl ContentServer {
    pub fn new(content: &[&PackContent]) -> Self {
        let mut this = Self {
            by_type:    HashMap::new(),
            by_guid:    HashMap::new(),
            by_data_id: HashMap::new(),
            by_name:    HashMap::new(),
        };

        unsafe {
            for _content in content {
                assert_eq!(
                    ContentFlags::from_bits_retain(_content.flags.get()),
                    ContentFlags::MANGLED
                );

                let data = _content.content.as_ptr();

                for fixup in _content.localOffsets.as_slice() {
                    let value = data
                        .add(fixup.relocOffset.get() as usize)
                        .cast_mut()
                        .cast::<usize>();
                    let offset = usize::from_le(value.read_unaligned());
                    value.write_unaligned(data as usize + offset);
                }

                for fixup in _content.externalOffsets.as_slice() {
                    let target = &content[fixup.targetFileIndex.get() as usize];

                    let value = data
                        .add(fixup.relocOffset.get() as usize)
                        .cast_mut()
                        .cast::<usize>();
                    let offset = usize::from_le(value.read_unaligned());
                    value.write_unaligned(target.content.as_ptr() as usize + offset);
                }

                for fixup in _content.fileIndices.as_slice() {
                    let value = data
                        .add(fixup.relocOffset.get() as usize)
                        .cast_mut()
                        .cast::<usize>();
                    let file_index = usize::from_le(value.read_unaligned());
                    value.write_unaligned(
                        content[0].fileRefs.as_slice()[file_index].as_ptr() as usize
                    );
                }

                for fixup in _content.stringIndices.as_slice() {
                    let value = data
                        .add(fixup.relocOffset.get() as usize)
                        .cast_mut()
                        .cast::<usize>();
                    let string_index = usize::from_le(value.read_unaligned());
                    value.write_unaligned(
                        _content.strings.as_slice()[string_index].as_ptr() as usize
                    );
                }

                for entry in _content.indexEntries.as_slice() {
                    let r#type = entry.r#type.get();
                    let type_info = &content[0].typeInfos.as_slice()[r#type as usize];

                    let data = &_content.content.as_slice()[entry.offset.get() as usize..];

                    this.by_type.entry(r#type).or_default().push(data.as_ptr());

                    let guid_offset = type_info.guidOffset.get();
                    if guid_offset != u32::MAX {
                        let guid = data[guid_offset as usize..].as_ptr().cast::<Guid>().read();

                        this.by_guid.insert(guid, data.as_ptr());
                    }

                    let name_offset = type_info.nameOffset.get();
                    if name_offset != u32::MAX {
                        let name = data[name_offset as usize..]
                            .as_ptr()
                            .cast::<Ptr<Name>>()
                            .read_unaligned();

                        let mut value = 0u64;
                        for &word in name.as_ref().unwrap().0.0.as_slice().iter() {
                            value = (value << 6)
                                | u64::from(match word as u8 {
                                    word @ b'A'..=b'Z' => word - b'A',
                                    word @ b'a'..=b'z' => word - b'a' + 26,
                                    word @ b'0'..=b'9' => word - b'0' + 52,
                                    b'+' => 62,
                                    b'/' => 63,
                                    _ => continue,
                                });
                        }

                        this.by_name.insert(value, data.as_ptr());
                    }

                    let data_id_offset = type_info.dataIdOffset.get();
                    if data_id_offset != u32::MAX {
                        let data_id = data[data_id_offset as usize..]
                            .as_ptr()
                            .cast::<u32>()
                            .read();

                        this.by_data_id
                            .insert(r#type << 22 | data_id & 0x3FFFFF, data.as_ptr());
                    }
                }
            }
        }

        this
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
        let (namespace, name) = name.rsplit_once('.')?;

        let ptr = *self
            .by_name
            .get(&(mangle_name(namespace) << 30 | mangle_name(name)))?;
        unsafe { (ptr.add(0x10).cast::<u32>().read() == T::ID).then(|| &*ptr.cast::<T>()) }
    }

    pub fn by_data_id<T: ContentType>(&self, data_id: u32) -> Option<&T> {
        let ptr = *self.by_data_id.get(&(T::ID << 22 | data_id))?;
        Some(unsafe { &*ptr.cast::<T>() })
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ContentFlags: u32 {
        const ENCRYPTED = 1 << 0;
        const MANGLED   = 1 << 1;
    }
}

fn mangle_name(name: &str) -> u64 {
    let mut digest = Sha256::new();
    for word in name.encode_utf16() {
        digest.update(word.to_le_bytes());
    }

    let mut hash = 0xCBF29CE484222325u64;
    for chunk in digest.finalize().chunks_exact(4) {
        let mut value = u32::from_le_bytes(chunk.try_into().unwrap());

        for _ in 0..4 {
            hash = (hash ^ value as u64).wrapping_mul(0x100000001B3);
            value >>= 8;
        }
    }

    hash.swap_bytes() >> 34
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Guid(u32, u16, u16, [u8; 8]);

#[cfg(feature = "graphql")]
#[async_graphql::Scalar]
impl async_graphql::ScalarType for Guid {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        let async_graphql::Value::String(value) = value else {
            return Err(async_graphql::InputValueError::expected_type(value));
        };

        let bytes = u128::from_str_radix(&value.replace('-', ""), 16)
            .map_err(async_graphql::InputValueError::custom)?
            .to_be_bytes();

        Ok(Self(
            u32::from_be_bytes(bytes[0..4].try_into().unwrap()),
            u16::from_be_bytes(bytes[4..6].try_into().unwrap()),
            u16::from_be_bytes(bytes[6..8].try_into().unwrap()),
            bytes[8..16].try_into().unwrap(),
        ))
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0,
            self.1,
            self.2,
            self.3[0],
            self.3[1],
            self.3[2],
            self.3[3],
            self.3[4],
            self.3[5],
            self.3[6],
            self.3[7],
        ))
    }
}

#[repr(transparent)]
pub struct Ptr<T: ?Sized>(*const T);

unsafe impl<T: ?Sized> Send for Ptr<T> {}
unsafe impl<T: ?Sized> Sync for Ptr<T> {}

impl<T: ?Sized> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.0
    }

    pub unsafe fn as_ref(&self) -> Option<&T> {
        unsafe { self.as_ptr().as_ref() }
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::async_trait::async_trait]
impl<T: async_graphql::OutputType> async_graphql::OutputType for Ptr<T> {
    fn type_name() -> std::borrow::Cow<'static, str> {
        T::type_name()
    }

    fn qualified_type_name() -> std::string::String {
        T::type_name().into_owned()
    }

    fn create_type_info(registry: &mut async_graphql::registry::Registry) -> std::string::String {
        T::create_type_info(registry);
        Self::qualified_type_name()
    }

    fn resolve(
        &self,
        ctx: &async_graphql::ContextSelectionSet<'_>,
        field: &async_graphql::Positioned<async_graphql::parser::types::Field>,
    ) -> impl Future<Output = async_graphql::ServerResult<async_graphql::Value>> + Send {
        async move {
            match unsafe { self.as_ref() } {
                Some(value) => value.resolve(ctx, field).await,
                None => Ok(async_graphql::Value::Null),
            }
        }
    }
}

#[cfg(feature = "graphql")]
impl<T: async_graphql::OutputType> async_graphql::OutputType for Ptr<[T]> {
    fn type_name() -> std::borrow::Cow<'static, str> {
        <&[T] as async_graphql::OutputType>::type_name()
    }

    fn qualified_type_name() -> std::string::String {
        format!("[{}]", T::qualified_type_name())
    }

    fn create_type_info(registry: &mut async_graphql::registry::Registry) -> std::string::String {
        <&[T] as async_graphql::OutputType>::create_type_info(registry);
        Self::qualified_type_name()
    }

    async fn resolve(
        &self,
        ctx: &async_graphql::ContextSelectionSet<'_>,
        field: &async_graphql::Positioned<async_graphql::parser::types::Field>,
    ) -> async_graphql::ServerResult<async_graphql::Value> {
        match unsafe { self.as_ref() } {
            Some(slice) => slice.resolve(ctx, field).await,
            None => Ok(async_graphql::Value::Null),
        }
    }
}

#[repr(transparent)]
pub struct WcharPtr(*const u16);

unsafe impl Send for WcharPtr {}
unsafe impl Sync for WcharPtr {}

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

impl WcharPtr {
    pub unsafe fn file_id(&self) -> Option<u32> {
        let [a, b, ..] = (unsafe { self.as_slice() }) else {
            return None;
        };

        let (a, b) = (*a, *b);
        if a <= 0xFF || b <= 0xFF {
            return None;
        }

        Some((u32::from(a) - 0xFF) + (u32::from(b) - 0x100) * 0xFF00)
    }
}

#[cfg(feature = "graphql")]
impl async_graphql::OutputType for WcharPtr {
    fn type_name() -> std::borrow::Cow<'static, str> {
        std::string::String::type_name()
    }

    fn qualified_type_name() -> std::string::String {
        Self::type_name().into_owned()
    }

    fn create_type_info(registry: &mut async_graphql::registry::Registry) -> std::string::String {
        std::string::String::create_type_info(registry);
        Self::qualified_type_name()
    }

    fn resolve(
        &self,
        _: &async_graphql::ContextSelectionSet<'_>,
        _: &async_graphql::Positioned<async_graphql::parser::types::Field>,
    ) -> impl Future<Output = async_graphql::ServerResult<async_graphql::Value>> + Send {
        std::future::ready(Ok(if self.as_ptr().is_null() {
            async_graphql::Value::Null
        } else {
            async_graphql::Value::String(std::string::String::from_utf16le_lossy(unsafe {
                self.as_bytes()
            }))
        }))
    }
}

#[repr(C, align(8))]
pub struct String(WcharPtr, u32);

#[cfg(feature = "graphql")]
impl async_graphql::OutputType for String {
    fn type_name() -> std::borrow::Cow<'static, str> {
        std::string::String::type_name()
    }

    fn qualified_type_name() -> std::string::String {
        Self::type_name().into_owned()
    }

    fn create_type_info(registry: &mut async_graphql::registry::Registry) -> std::string::String {
        std::string::String::create_type_info(registry);
        Self::qualified_type_name()
    }

    fn resolve(
        &self,
        ctx: &async_graphql::ContextSelectionSet<'_>,
        field: &async_graphql::Positioned<async_graphql::parser::types::Field>,
    ) -> impl Future<Output = async_graphql::ServerResult<async_graphql::Value>> + Send {
        self.0.resolve(ctx, field)
    }
}

#[repr(C)]
pub struct Name(String, String);

#[cfg(feature = "graphql")]
impl async_graphql::OutputType for Name {
    fn type_name() -> std::borrow::Cow<'static, str> {
        std::string::String::type_name()
    }

    fn create_type_info(registry: &mut async_graphql::registry::Registry) -> std::string::String {
        std::string::String::create_type_info(registry)
    }

    fn resolve(
        &self,
        ctx: &async_graphql::ContextSelectionSet<'_>,
        field: &async_graphql::Positioned<async_graphql::parser::types::Field>,
    ) -> impl Future<Output = async_graphql::ServerResult<async_graphql::Value>> + Send {
        self.0.resolve(ctx, field)
    }
}

pub trait ContentType {
    const ID: u32;
}

mod achievement;
pub use achievement::*;

mod cinematic;
pub use cinematic::*;

mod color;
pub use color::*;

mod configuration;
pub use configuration::*;

mod crafting_recipe;
pub use crafting_recipe::*;

mod currency;
pub use currency::*;

mod effect;
pub use effect::*;

mod emote;
pub use emote::*;

mod item;
pub use item::*;

mod mail;
pub use mail::*;

mod map;
pub use map::*;

mod marker;
pub use marker::*;

mod progress;
pub use progress::*;

mod sector;
pub use sector::*;

mod skill;
pub use skill::*;

mod skin;
pub use skin::*;

mod species;
pub use species::*;

mod table;
pub use table::*;

mod team;
pub use team::*;

mod r#trait;
pub use r#trait::*;
