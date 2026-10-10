use super::{Color, ContentType, Guid, Name, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Team {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub textName:        u32,
    _2c:                 u32,
    _30:                 Ptr<()>,
    pub _38:             Ptr<Color>,
    pub _40:             Ptr<Color>,
    pub _48:             Ptr<Color>,
    pub _50:             Ptr<Color>,
    pub _58:             Ptr<Color>,
    pub _60:             u32,
    pub _64:             u32,
}

impl ContentType for Team {
    const ID: u32 = 401;
}
