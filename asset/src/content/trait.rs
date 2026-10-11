#![allow(non_snake_case)]

use super::{ContentType, FileRef, Guid, Name, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Trait {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _2c:                 u32,
    pub _30:             Ptr<u32>,
    pub _38:             u32,
    _3c:                 u32,
    pub _40:             Ptr<u32>,
    pub _48:             u32,
    pub _4c:             u32,
    pub _50:             FileRef,
    pub _58:             u32,
    pub _5c:             u32,
    pub _60:             u32,
    pub textName:        u32,
    pub textDescription: u32,
    pub _6c:             u32,
    pub _70:             u32,
    _74:                 u32,
    pub _78:             Ptr<u32>,
    pub _80:             u32,
    _84:                 u32,
}

impl ContentType for Trait {
    const ID: u32 = 77;
}
