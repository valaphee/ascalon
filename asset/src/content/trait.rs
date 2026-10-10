use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Trait {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _30:             Ptr<()>,
    pub _38:             u32,
    pub _3c:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _40:             Ptr<()>,
    pub _48:             u32,
    pub _4c:             u32,
    pub _50:             WcharPtr,
    pub _58:             u32,
    pub _5c:             u32,
    pub _60:             u32,
    pub _64:             u32,
    pub _68:             u32,
    pub _6c:             u32,
    pub _70:             u32,
    pub _74:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _78:             Ptr<()>,
    pub _80:             u32,
    pub _84:             u32,
}

impl ContentType for Trait {
    const ID: u32 = 77;
}
