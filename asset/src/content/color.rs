use super::{ContentType, Guid, Name, Ptr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Color {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _34:                 u32,
    pub materials:       Ptr<[ColorMaterial]>,
    _40:                 u32,
    _44:                 u32,
    pub textName:        u32,
    _4c:                 u32,
}

impl ContentType for Color {
    const ID: u32 = 9;
}

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct ColorMaterial {
    pub brightness: f32,
    pub contrast:   f32,
    pub hue:        f32,
    pub saturation: f32,
    pub lightness:  f32,
    pub r#type:     ColorMaterialType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[repr(u32)]
pub enum ColorMaterialType {
    Default = 0,
    Cloth   = 1,
    Leather = 2,
    Metal   = 3,
    Fur     = 4,
}

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct ColorPalette {
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
    pub _3c:             u32,
}

impl ContentType for ColorPalette {
    const ID: u32 = 147;
}
