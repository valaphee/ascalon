#![allow(non_snake_case)]

use super::{ContentType, Effect, FileRef, Guid, Name, Ptr, Team};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Marker {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub _28:             u32,
    pub _2c:             f32,
    pub _30:             Ptr<u32>,
    pub _38:             u32,
    _3c:                 u32,
    pub _40:             f32,
    pub _44:             f32,
    pub _48:             Ptr<Effect>,
    pub _50:             FileRef,
    pub _58:             FileRef,
    pub _60:             u32,
    pub _64:             u32,
    pub _68:             Ptr<Team>,
    pub _70:             u32,
    pub _74:             u32,
    pub _78:             FileRef,
    pub _80:             FileRef,
    pub _88:             f32,
    _8c:                 u32,
    pub _90:             FileRef,
    pub _98:             u32,
    _9c:                 u32,
}

impl ContentType for Marker {
    const ID: u32 = 293;
}
