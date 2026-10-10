use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Skill {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             u32,
    pub textName:        u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub flags:           SkillFlags,
    _3c:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _40:             Ptr<[()]>,
    pub fileIcon:        WcharPtr,
    pub _58:             u32,
    pub _5c:             u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _60:             Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _68:             Ptr<[()]>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _78:             Ptr<()>,
    _80:                 u32,
    _84:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _88:             Ptr<[()]>,
    pub _98:             u32,
    _9c:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _a0:             Ptr<[()]>,
    pub _b0:             Ptr<Skill>,
}

impl ContentType for Skill {
    const ID: u32 = 64;
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct SkillFlags: u32 {
        const GROUND_TARGETED = 1 << 12;
    }
}
