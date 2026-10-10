use super::{ContentType, Guid, Name, Progress, Ptr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Emote {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             u32,
    pub _34:             u32,
    pub _38:             u32,
    pub _3c:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _40:             Ptr<()>,
    pub _48:             u32,
    pub _4c:             u32,
    pub _50:             u32,
    pub _54:             u32,
    pub _58:             u32,
    pub _5c:             u32,
    pub _60:             Ptr<Progress>,
    pub _68:             u32,
    pub _6c:             u32,
}

impl ContentType for Emote {
    const ID: u32 = 19;
}
