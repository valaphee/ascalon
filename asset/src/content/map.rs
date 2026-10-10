use super::{ContentType, Guid, Mail, Name, Progress, Ptr, String, WcharPtr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Map {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub r#type:          MapType,
    pub _030:            u32,
    _034:                u32,
    pub _038:            WcharPtr,
    pub _040:            WcharPtr,
    pub _048:            WcharPtr,
    pub _050:            WcharPtr,
    pub _058:            WcharPtr,
    pub _060:            WcharPtr,
    pub fileMap:         WcharPtr,
    _070:                u32,
    _074:                u32,
    pub _078:            WcharPtr,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _080:            Ptr<[()]>,
    pub _090:            WcharPtr,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _098:            MapFlags,
    pub _09c:            u32,
    _0a0:                u32,
    _0a4:                u32,
    pub levelMin:        u32,
    pub levelMax:        u32,
    pub _0b0:            String,
    pub _0c0:            Ptr<Mail>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _0c8:            Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _0d0:            Ptr<()>,
    pub _0d8:            String,
    _0e8:                u32,
    _0ec:                u32,
    _0f0:                u32,
    _0f4:                u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _0f8:            Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub pvp:             Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _108:            Ptr<()>,
    pub _110:            String,
    _120:                u32,
    _124:                u32,
    _128:                u32,
    _12c:                u32,
    _130:                u32,
    _134:                u32,
    _138:                u32,
    _13c:                u32,
    _140:                u32,
    _144:                u32,
    _148:                u32,
    _14c:                u32,
    _150:                u32,
    _154:                u32,
    _158:                u32,
    _15c:                u32,
    _160:                u32,
    _164:                u32,
    pub textName:        u32,
    pub textDescription: u32,
    pub _170:            u32,
    _174:                u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _178:            Ptr<[()]>,
    pub _188:            u32,
    _18c:                u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _190:            Ptr<[()]>,
    pub _1a0:            [u8; 16],
    _1b0:                u32,
    _1b4:                u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _1b8:            Ptr<[()]>,
    _1c8:                u32,
    _1cc:                u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _1d0:            Ptr<()>,
    pub _1d8:            Ptr<Progress>,
    _1e0:                u32,
    _1e4:                u32,
    _1e8:                u32,
    _1ec:                u32,
    pub _1f0:            Ptr<Progress>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _1f8:            Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _200:            Ptr<[()]>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _210:            Ptr<[()]>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _220:            Ptr<()>,
}

impl ContentType for Map {
    const ID: u32 = 45;
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[repr(u32)]
pub enum MapType {
    _0             = 0,
    _1             = 1,
    Pvp            = 2,
    Instance       = 4,
    _5             = 5,
    Tutorial       = 7,
    Center         = 9,
    BlueHome       = 10,
    GreenHome      = 11,
    RedHome        = 12,
    JumpPuzzle     = 14,
    EdgeOfTheMists = 15,
    _16            = 16,
    Unknown        = 18,
    _19            = 19,
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MapFlags: u32 {
        const INSTANCE_CHECKPOINT_OVERRIDE = 1 << 17;
    }
}
