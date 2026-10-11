use super::{ContentType, Guid, Item, Name, Progress, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct CraftingRecipe {
    pub contentGuid:      Guid,
    pub contentType:      u32,
    pub contentUid:       u32,
    pub contentName:      Ptr<Name>,
    pub contentFullName:  Ptr<Name>,
    pub dataId:           u32,
    pub _2c:              u32,
    pub ingredients:      Ptr<[u32]>,
    pub outputItem:       Ptr<Item>,
    pub outputItemCount:  u32,
    pub rating:           u32,
    pub _50:              u32,
    pub time:             u32,
    pub _58:              u32,
    _5c:                  u32,
    pub _60:              Ptr<Progress>,
    pub _68:              Ptr<u32>,
    _70:                  u32,
    _74:                  u32,
    pub guildIngredients: Ptr<[u32]>,
    pub item:             Ptr<Item>,
    pub _90:              Ptr<u32>,
    _98:                  u32,
    _9c:                  u32,
    _a0:                  u32,
    _a4:                  u32,
    pub _a8:              Ptr<u32>,
}

impl ContentType for CraftingRecipe {
    const ID: u32 = 12;
}
