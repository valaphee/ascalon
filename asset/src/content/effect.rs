use super::{ContentType, Guid, Name, Ptr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Effect {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub _28:             u32,
    pub _2c:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _30:             Ptr<()>,
    pub _38:             u32,
    pub _40:             u32,
    pub _44:             u32,
    pub _48:             Ptr<Effect>,
    pub _50:             u32,
    pub _54:             u32,
    pub _58:             u32,
    pub _5c:             u32,
}

impl ContentType for Effect {
    const ID: u32 = 183;
}
