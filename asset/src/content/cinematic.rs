use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Cinematic {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             WcharPtr,
    pub _38:             u32,
    pub _3c:             u32,
    _40:                 Ptr<()>,
    pub _48:             u32,
    pub _4c:             u32,
    _50:                 Ptr<()>,
    _58:                 Ptr<()>,
    pub _60:             u32,
    pub _64:             u32,
    pub _68:             u32,
    pub _6c:             u32,
}

impl ContentType for Cinematic {
    const ID: u32 = 7;
}
